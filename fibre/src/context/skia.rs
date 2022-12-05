use std::num::NonZeroU32;

use gl::types::GLint;
use glutin::{config::Config, prelude::GlConfig, surface::GlSurface};
use skia_safe::{
    gpu::{gl::FramebufferInfo, BackendRenderTarget, DirectContext, SurfaceOrigin},
    Canvas, ColorType, Surface,
};

use super::gl::GlWindowContext;

#[derive(Debug)]
pub struct SkiaWindowContext {
    gl_context: GlWindowContext,

    context: DirectContext,
    surface: Surface,
}

impl SkiaWindowContext {
    pub fn new(window_size: (i32, i32), gl_context: GlWindowContext) -> Self {
        let mut context = DirectContext::new_gl(None, None).unwrap();

        let surface = create_surface(window_size, gl_context.config(), &mut context);

        Self {
            gl_context,
            context,
            surface,
        }
    }

    pub fn gl_context(&self) -> &GlWindowContext {
        &self.gl_context
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface = create_surface(
            (width as _, height as _),
            self.gl_context.config(),
            &mut self.context,
        );

        self.gl_context.surface().resize(
            self.gl_context.context(),
            NonZeroU32::new(1_u32.max(width)).unwrap(),
            NonZeroU32::new(1_u32.max(height)).unwrap(),
        );
    }

    pub fn render<'a>(&'a mut self) -> SkiaSurfaceRenderer<'a> {
        SkiaSurfaceRenderer {
            window: &self.gl_context,
            canvas: self.surface.canvas(),
        }
    }
}

pub struct SkiaSurfaceRenderer<'a> {
    window: &'a GlWindowContext,
    canvas: &'a mut Canvas,
}

impl SkiaSurfaceRenderer<'_> {
    pub fn canvas(&mut self) -> &mut Canvas {
        self.canvas
    }

    pub fn finish(self) {
        if let Some(mut surface) = unsafe { self.canvas.surface() } {
            surface.flush();
        }

        self.window
            .surface()
            .swap_buffers(self.window.context())
            .unwrap();
    }
}

fn create_surface(
    (width, height): (i32, i32),
    config: &Config,
    context: &mut DirectContext,
) -> Surface {
    let framebuffer_info = {
        let mut fboid: GLint = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

        FramebufferInfo {
            fboid: fboid.try_into().unwrap(),
            format: skia_safe::gpu::gl::Format::RGBA8.into(),
        }
    };

    let render_target = BackendRenderTarget::new_gl(
        (width, height),
        Some(config.num_samples() as _),
        config.stencil_size() as _,
        framebuffer_info,
    );

    skia_safe::Surface::from_backend_render_target(
        context,
        &render_target,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .unwrap()
}
