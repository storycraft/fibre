use std::{cell::RefCell, rc::Rc};

use async_component::{AsyncComponent, StateCell, context::StateContext};
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

#[derive(AsyncComponent)]
pub struct WidgetNode {
    cx: StateContext,

    node: Node,

    #[state(Self::on_style_update)]
    pub style: StateCell<Style>,

    layout: Rc<RefCell<Taffy>>,
}

impl WidgetNode {
    pub fn new_root(cx: &StateContext, style: Style) -> Self {
        let mut layout = Taffy::new();

        let node = layout.new_leaf(style.clone()).unwrap();
        let style = StateCell::new(cx.clone(), style);

        Self {
            cx: cx.clone(),

            node,

            style,

            layout: Rc::new(RefCell::new(layout)),
        }
    }

    fn on_style_update(&mut self, _: ()) {
        self.layout
            .borrow_mut()
            .set_style(self.node, self.style.clone())
            .unwrap();
    }

    pub fn new_child(&self, style: Style) -> WidgetNode {
        let mut layout = self.layout.borrow_mut();

        let child_node = layout.new_leaf(Style::DEFAULT).unwrap();
        layout.add_child(self.node, child_node).unwrap();

        WidgetNode {
            cx: self.cx.clone(),
            node: child_node,
            style: StateCell::new(self.cx.clone(), style),
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
