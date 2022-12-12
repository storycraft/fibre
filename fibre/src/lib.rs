pub mod component;
pub mod context;

use async_component_winit::WinitComponent;
use component::{FibreComponent, WidgetNode};
pub use skia_safe as skia;
pub use taffy;
use taffy::{prelude::Size, style::Style};

use std::{ffi::CString, sync::Arc};

use async_component::AsyncComponent;
use glutin::{
    config::ConfigTemplateBuilder,
    display::{Display, DisplayApiPreference},
    prelude::{GlConfig, GlDisplay},
    surface::{GlSurface, SwapInterval},
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

use crate::context::{gl::GlWindowContext, skia::SkiaWindowContext};

#[derive(AsyncComponent)]
pub struct Fibre<T: FibreComponent> {
    window: Arc<Window>,

    skia_window_ctx: SkiaWindowContext,

    root_node: WidgetNode,

    #[component(Self::on_component_change)]
    component: T,
}

impl<T: FibreComponent> Fibre<T> {
    pub fn new(
        window: Arc<Window>,
        skia_window_ctx: SkiaWindowContext,
        component_fn: impl FnOnce(&Arc<Window>, WidgetNode) -> T,
    ) -> Self {
        let (width, height) = window.inner_size().into();
        let root_node = WidgetNode::new_root(Self::create_root_style(width, height));

        let component = component_fn(&window, root_node.new_child(Style::DEFAULT));

        Self {
            window,
            skia_window_ctx,

            root_node,
            component,
        }
    }

    fn create_root_style(width: f32, height: f32) -> Style {
        Style {
            size: Size::from_points(width, height),
            ..Default::default()
        }
    }

    fn on_component_change(&mut self) {
        self.window.request_redraw();
    }

    fn render(&mut self) {
        let mut renderer = self.skia_window_ctx.render();

        renderer.canvas().clear(0);

        self.root_node.compute_layout();
        self.component.draw(&mut renderer);

        renderer.finish();
    }
}

impl<T: FibreComponent> WinitComponent for Fibre<T> {
    fn on_event(&mut self, event: &mut Event<()>, _: &mut ControlFlow) {
        self.component.on_event(event);

        match event {
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Resized(size),
            } => {
                self.root_node.set_style(Self::create_root_style(size.width as _, size.height as _));

                self.skia_window_ctx.resize(size.width, size.height);
            }

            Event::RedrawRequested(_) => {
                self.render();
            }

            _ => {}
        }
    }
}

pub fn run<Component: FibreComponent + 'static>(
    setup_func: impl FnOnce(&Arc<Window>, WidgetNode) -> Component,
) -> ! {
    let event_loop = EventLoopBuilder::with_user_event().build();

    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let display = unsafe {
        Display::new(
            window.raw_display_handle(),
            DisplayApiPreference::EglThenWgl(Some(window.raw_window_handle())),
        )
    }
    .expect("Failed to create display");

    let template = ConfigTemplateBuilder::new()
        .compatible_with_native_window(window.raw_window_handle())
        .build();

    let config = unsafe { display.find_configs(template) }
        .unwrap()
        .reduce(|config, acc| {
            if config.num_samples() > 0 {
                config
            } else {
                acc
            }
        })
        .expect("No available configs");

    println!("Picked a config with {} samples", config.num_samples());

    gl::load_with(|addr| {
        let addr = CString::new(addr).unwrap();
        display.get_proc_address(&addr)
    });

    let gl_window_ctx = GlWindowContext::new(config, &window);
    gl_window_ctx
        .surface()
        .set_swap_interval(gl_window_ctx.context(), SwapInterval::DontWait)
        .unwrap();

    let skia_window_ctx = SkiaWindowContext::new(window.inner_size().into(), gl_window_ctx);

    let window = Arc::new(window);

    async_component_winit::run(
        event_loop,
        Fibre::new(window, skia_window_ctx, setup_func),
    )
}
