use std::{cell::RefCell, rc::Rc};

use async_component::AsyncComponent;
use taffy::{
    prelude::{Layout, Node, Size},
    style::Style,
    Taffy,
};
use winit::event::Event;

use crate::context::skia::SkiaSurfaceRenderer;

pub trait FibreComponent: AsyncComponent {
    fn draw(&self, renderer: &mut SkiaSurfaceRenderer);

    fn on_event(&mut self, event: &mut Event<()>);
}

pub struct WidgetNode {
    node: Node,

    layout: Rc<RefCell<Taffy>>,
}

impl WidgetNode {
    pub fn new_root(style: Style) -> Self {
        let mut layout = Taffy::new();

        let node = layout.new_leaf(style).unwrap();

        Self {
            node,

            layout: Rc::new(RefCell::new(layout)),
        }
    }

    pub fn set_style(&mut self, style: Style) {
        self.layout
            .borrow_mut()
            .set_style(self.node, style)
            .unwrap();
    }

    pub fn new_child(&self, style: Style) -> WidgetNode {
        let mut layout = self.layout.borrow_mut();

        let child_node = layout.new_leaf(style).unwrap();
        layout.add_child(self.node, child_node).unwrap();

        WidgetNode {
            node: child_node,
            layout: self.layout.clone(),
        }
    }

    pub fn compute_layout(&mut self) {
        let mut layout = self.layout.borrow_mut();

        layout.compute_layout(self.node, Size::MAX_CONTENT).unwrap();
    }

    pub fn layout(&self) -> Layout {
        self.layout.borrow().layout(self.node).unwrap().clone()
    }
}

impl Drop for WidgetNode {
    fn drop(&mut self) {
        self.layout.borrow_mut().remove(self.node).unwrap();
    }
}
