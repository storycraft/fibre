use async_component::{AsyncComponent, StateCell};
use fibre::{context::skia::SkiaSurfaceRenderer, skia::Paint, Element};
use skia_safe::{Color4f, Point};
use winit::event::{Event, WindowEvent};

fn main() {
    fibre::run(|_| TestComponent::new());
}

#[derive(Debug, AsyncComponent)]
pub struct TestComponent {
    #[state]
    cursor: StateCell<Point>,
}

impl TestComponent {
    pub fn new() -> Self {
        Self {
            cursor: Default::default(),
        }
    }
}

impl Element for TestComponent {
    fn draw(&self, renderer: &mut SkiaSurfaceRenderer) {
        let mut font = skia_safe::Font::default();
        font.set_size(50.0);

        renderer.canvas().draw_str(
            "Skia",
            *self.cursor,
            &font,
            &Paint::new(Color4f::from(0xffffffff), None),
        );
    }

    fn on_event(&mut self, event: &mut Event<()>) {
        if let Event::WindowEvent {
            event: WindowEvent::CursorMoved { ref position, .. },
            ..
        } = event
        {
            *self.cursor = Point::new(position.x as _, position.y as _);
        }
    }
}
