use std::sync::Arc;

use async_component::{AsyncComponent, PhantomState};
use async_component_winit::WinitComponent;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Window,
};

use crate::{context::skia::SkiaWindowContext, Element};

#[derive(Debug, AsyncComponent)]
pub struct RootContainer<T: AsyncComponent + Element> {
    window: Arc<Window>,

    skia_window_ctx: SkiaWindowContext,

    #[component(Self::on_component_change)]
    component: T,

    #[state]
    _state: PhantomState,
}

impl<T: AsyncComponent + Element> RootContainer<T> {
    pub fn new(window: Arc<Window>, skia_window_ctx: SkiaWindowContext, component: T) -> Self {
        Self {
            window,
            skia_window_ctx,
            component,
            _state: Default::default(),
        }
    }

    fn on_component_change(&mut self) {
        self.window.request_redraw();
    }
}

impl<T: AsyncComponent + Element> WinitComponent for RootContainer<T> {
    fn on_event(&mut self, event: &mut Event<()>, _: &mut ControlFlow) {
        self.component.on_event(event);

        match event {
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Resized(size),
            } => self.skia_window_ctx.resize(size.width, size.height),

            Event::RedrawRequested(_) => {
                let mut renderer = self.skia_window_ctx.render();

                renderer.canvas().clear(0);

                self.component.draw(&mut renderer);

                renderer.finish();
            }

            _ => {}
        }
    }
}
