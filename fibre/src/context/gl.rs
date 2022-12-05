use std::num::NonZeroU32;

use glutin::{
    config::Config,
    context::ContextAttributesBuilder,
    context::PossiblyCurrentContext,
    display::GetGlDisplay,
    prelude::{GlDisplay, NotCurrentGlContextSurfaceAccessor},
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use raw_window_handle::HasRawWindowHandle;
use winit::window::Window;

#[derive(Debug)]
pub struct GlWindowContext {
    config: Config,

    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
}

impl GlWindowContext {
    pub fn new(config: Config, window: &Window) -> Self {
        let (width, height) = window.inner_size().into();
        let raw_window_handle = window.raw_window_handle();

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(1_u32.max(width)).unwrap(),
            NonZeroU32::new(1_u32.max(height)).unwrap(),
        );

        let surface = unsafe {
            config
                .display()
                .create_window_surface(&config, &attrs)
                .unwrap()
        };

        let context = unsafe {
            config.display().create_context(
                &config,
                &ContextAttributesBuilder::new().build(Some(window.raw_window_handle())),
            )
        }
        .unwrap();
        let context = context.make_current(&surface).unwrap();

        Self {
            config,

            context,
            surface,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn surface(&self) -> &Surface<WindowSurface> {
        &self.surface
    }

    pub fn context(&self) -> &PossiblyCurrentContext {
        &self.context
    }
}
