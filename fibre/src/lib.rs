pub mod context;
pub mod root;

pub use skia_safe as skia;

use std::{ffi::CString, sync::Arc};

use async_component::AsyncComponent;
use context::skia::SkiaSurfaceRenderer;
use glutin::{
    config::ConfigTemplateBuilder,
    display::{Display, DisplayApiPreference},
    prelude::{GlConfig, GlDisplay},
    surface::{GlSurface, SwapInterval},
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{
    event::Event,
    event_loop::EventLoopBuilder,
    window::{Window, WindowBuilder},
};

use crate::{
    context::{gl::GlWindowContext, skia::SkiaWindowContext},
    root::RootContainer,
};

pub trait Element {
    fn draw(&self, renderer: &mut SkiaSurfaceRenderer);

    fn on_event(&mut self, event: &mut Event<()>);
}

pub fn run<Component: AsyncComponent + Element + 'static>(
    setup_func: impl FnOnce(&Arc<Window>) -> Component,
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
            if config.num_samples() > acc.num_samples() {
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

    let component = setup_func(&window);

    async_component_winit::run(
        event_loop,
        RootContainer::new(window, skia_window_ctx, component),
    )
}
