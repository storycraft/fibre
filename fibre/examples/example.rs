use std::time::{Duration, Instant};

use async_component::{components::VecComponent, AsyncComponent, StateCell};
use fibre::{
    component::{FibreComponent, WidgetNode},
    context::skia::SkiaSurfaceRenderer,
    skia::Paint,
};
use skia_safe::{Color4f, Point, Rect};
use taffy::{
    prelude::Size,
    style::{AlignSelf, Style},
};
use winit::event::{Event, WindowEvent};

fn main() {
    fibre::run(|_, node| TestComponent::new(node));
}

#[derive(AsyncComponent)]
#[component(Self::update)]
pub struct TestComponent {
    #[component]
    node: WidgetNode,

    #[component]
    circles: VecComponent<FadingCircle>,
}

impl TestComponent {
    pub fn new(mut node: WidgetNode) -> Self {
        *node.style = Style {
            align_self: AlignSelf::Center,

            size: Size::from_points(100.0, 100.0),
            ..Default::default()
        };

        Self {
            node,
            circles: VecComponent::new(),
        }
    }

    fn update(&mut self) {
        self.circles.retain(|circle| !circle.expired());
    }
}

impl FibreComponent for TestComponent {
    fn draw(&self, renderer: &mut SkiaSurfaceRenderer) {
        let layout = self.node.layout();

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

        for circle in &self.circles {
            circle.draw(renderer);
        }
    }

    fn on_event(&mut self, event: &mut Event<()>) {
        if let Event::WindowEvent {
            event: WindowEvent::CursorMoved { ref position, .. },
            ..
        } = event
        {
            self.circles.push(FadingCircle::new(
                (position.x as _, position.y as _),
                16.0,
                Duration::from_secs(1),
            ));
        }
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
}

impl FadingCircle {
    fn new(position: (f32, f32), radius: f32, duration: Duration) -> Self {
        Self {
            position,
            radius,
            duration,
            start: Instant::now(),
            elapsed: Default::default(),
        }
    }

    fn update(&mut self) {
        *self.elapsed = self.start.elapsed();
    }

    pub fn expired(&self) -> bool {
        *self.elapsed > self.duration
    }
}

impl FibreComponent for FadingCircle {
    fn draw(&self, renderer: &mut SkiaSurfaceRenderer) {
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
