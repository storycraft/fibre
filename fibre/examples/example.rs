use std::time::{Duration, Instant};

use async_component::{AsyncComponent, PhantomState, StateCell};
use fibre::{
    context::skia::SkiaSurfaceRenderer, skia::Paint, FibreChannel, FibreElement, FibreNode,
};
use skia_safe::{Color4f, Point, Rect};
use taffy::{
    prelude::{Layout, Size},
    style::{AlignSelf, Display, Style},
};
use winit::event::{Event, WindowEvent};

fn main() {
    fibre::run(|_| TestComponent::new());
}

#[derive(AsyncComponent)]
pub struct TestComponent {
    #[state]
    _state: PhantomState,

    channel: Option<FibreChannel>,
}

impl TestComponent {
    pub fn new() -> Self {
        Self {
            _state: Default::default(),
            channel: None,
        }
    }
}

impl FibreElement for TestComponent {
    fn draw(&self, renderer: &mut SkiaSurfaceRenderer, layout: &Layout) {
        let mut font = skia_safe::Font::default();
        font.set_size(50.0);

        renderer.canvas().draw_rect(
            Rect::new(
                layout.location.x,
                layout.location.y,
                layout.location.x + layout.size.width,
                layout.location.y + layout.size.height,
            ),
            &Paint::new(Color4f::from(0xffffffff), None),
        );
    }

    fn on_event(&mut self, event: &mut Event<()>) {
        if let Event::WindowEvent {
            event: WindowEvent::CursorMoved { ref position, .. },
            ..
        } = event
        {
            self.channel
                .as_mut()
                .unwrap()
                .append_root(FadingCircle::new(
                    (position.x as _, position.y as _),
                    16.0,
                    Duration::from_secs(1),
                ));
        }
    }

    fn mount(&mut self, node: &mut FibreNode) {
        node.update_style(Style {
            align_self: AlignSelf::Center,

            size: Size::from_points(100.0, 100.0),
            ..Default::default()
        });

        self.channel = Some(node.create_channel());
    }

    fn unmount(&mut self) {
        self.channel.take();
    }
}

#[derive(AsyncComponent)]
#[component(Self::update)]
pub struct FadingCircle {
    position: (f32, f32),
    radius: f32,

    duration: Duration,

    start: Instant,

    #[state]
    elapsed: StateCell<Duration>,

    channel: Option<FibreChannel>,
}

impl FadingCircle {
    fn new(position: (f32, f32), radius: f32, duration: Duration) -> Self {
        Self {
            position,
            radius,
            duration,
            start: Instant::now(),
            elapsed: Default::default(),
            channel: None,
        }
    }

    fn update(&mut self) {
        *self.elapsed = self.start.elapsed();
        if *self.elapsed > self.duration {
            self.channel.as_mut().unwrap().retire();
        }
    }
}

impl FibreElement for FadingCircle {
    fn mount(&mut self, node: &mut FibreNode) {
        node.update_style(Style {
            display: Display::None,
            ..Default::default()
        });

        self.channel = Some(node.create_channel());
    }

    fn unmount(&mut self) {
        self.channel.take();
    }

    fn draw(&self, renderer: &mut SkiaSurfaceRenderer, _: &Layout) {
        let mut color = Color4f::from(0xffff0000);

        color.a *= 1.0 - 1.0_f32.min(self.elapsed.as_secs_f32() / self.duration.as_secs_f32());

        renderer.canvas().draw_circle(
            Point::new(self.position.0, self.position.1),
            self.radius,
            &Paint::new(color, None),
        );
    }

    fn on_event(&mut self, _: &mut Event<()>) {}
}
