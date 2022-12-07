pub mod context;

use async_component_winit::WinitComponent;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
pub use skia_safe as skia;
pub use taffy;
use taffy::{
    prelude::{Layout, Node, Size},
    style::Style,
    tree::LayoutTree,
    Taffy,
};

use std::{collections::hash_map::Entry, ffi::CString, sync::Arc};

use async_component::{AsyncComponent, PhantomState, StateCell, components::map::HashMapComponent};
use context::skia::SkiaSurfaceRenderer;
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

pub trait FibreElement {
    fn mount(&mut self, node: &mut FibreNode);

    fn unmount(&mut self);

    fn draw(&self, renderer: &mut SkiaSurfaceRenderer, layout: &Layout);

    fn on_event(&mut self, event: &mut Event<()>);
}

pub trait FibreComponent: AsyncComponent + FibreElement {}
impl<T: AsyncComponent + FibreElement> FibreComponent for T {}

pub type BoxedFibreComponent = Box<dyn FibreComponent>;

#[derive(AsyncComponent)]
pub struct Fibre {
    window: Arc<Window>,

    skia_window_ctx: SkiaWindowContext,

    #[component(Self::on_component_change)]
    components: HashMapComponent<Node, BoxedFibreComponent>,

    root_node: Node,

    #[state]
    layout_engine: StateCell<Taffy>,

    command_sender: UnboundedSender<FibreCommand>,

    #[stream(Self::on_command)]
    command_recv: UnboundedReceiver<FibreCommand>,

    #[state]
    _state: PhantomState,
}

impl Fibre {
    pub fn new(
        window: Arc<Window>,
        skia_window_ctx: SkiaWindowContext,
        component: impl FibreComponent + 'static,
    ) -> Self {
        let (command_sender, command_recv) = unbounded();

        let mut layout_engine = Taffy::new();

        let (width, height) = window.inner_size().into();

        let root_node = layout_engine
            .new_leaf(Self::create_root_style(width, height))
            .unwrap();

        let mut fibre = Self {
            window,
            skia_window_ctx,

            components: HashMapComponent::new(),

            root_node,
            layout_engine: layout_engine.into(),

            command_sender,
            command_recv,

            _state: Default::default(),
        };

        fibre.append_root(Box::new(component));

        fibre
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

    fn append_root(&mut self, element: BoxedFibreComponent) {
        self.append_child(self.root_node, element)
    }

    fn append_child(&mut self, parent: Node, mut element: BoxedFibreComponent) {
        let node = self.layout_engine.new_leaf(Style::DEFAULT).unwrap();

        self.layout_engine.add_child(parent, node).unwrap();

        element.mount(&mut self.create_node(node));

        self.components.insert(node, element);
    }

    fn update_style(&mut self, node: Node, style: Style) {
        self.layout_engine.set_style(node, style).unwrap();
    }

    fn retire(&mut self, node: Node) {
        if let Entry::Occupied(mut entry) = self.components.entry(node) {
            entry.get_mut().unmount();
            entry.remove();
        }

        Self::traverse_layout_children(&self.layout_engine, node, &mut |node| {
            if let Entry::Occupied(mut entry) = self.components.entry(*node) {
                entry.get_mut().unmount();
                entry.remove();
            }
        });

        self.layout_engine.remove(node).unwrap();
    }

    fn create_node(&mut self, node: Node) -> FibreNode {
        FibreNode { node, fibre: self }
    }

    fn create_channel(&self, node: Node) -> FibreChannel {
        FibreChannel {
            node,
            sender: self.command_sender.clone(),
        }
    }

    fn on_command(&mut self, command: FibreCommand) {
        match command {
            FibreCommand::AppendRoot(element) => {
                self.append_root(element);
            }

            FibreCommand::AppendChild(parent, element) => {
                self.append_child(parent, element);
            }

            FibreCommand::UpdateStyle(node, style) => {
                self.update_style(node, style);
            }

            FibreCommand::Retire(node) => {
                self.retire(node);
            }
        }
    }

    fn render(&mut self) {
        self.layout_engine
            .compute_layout(self.root_node, Size::MAX_CONTENT)
            .unwrap();

        let mut renderer = self.skia_window_ctx.render();

        renderer.canvas().clear(0);

        Self::traverse_layout_children(&self.layout_engine, self.root_node, &mut |node| {
            if let Some(component) = self.components.get_mut(node) {
                component.draw(&mut renderer, self.layout_engine.layout(*node).unwrap());
            }
        });

        renderer.finish();
    }

    fn traverse_layout_children(layout: &Taffy, node: Node, func: &mut impl FnMut(&Node)) {
        for child_node in LayoutTree::children(layout, node) {
            func(child_node);
            Self::traverse_layout_children(layout, *child_node, func);
        }
    }
}

impl WinitComponent for Fibre {
    fn on_event(&mut self, event: &mut Event<()>, _: &mut ControlFlow) {
        for component in self.components.values_mut() {
            component.on_event(event);
        }

        match event {
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Resized(size),
            } => {
                self.layout_engine
                    .set_style(
                        self.root_node,
                        Self::create_root_style(size.width as _, size.height as _),
                    )
                    .unwrap();

                self.skia_window_ctx.resize(size.width, size.height);
            }

            Event::RedrawRequested(_) => {
                self.render();
            }

            _ => {}
        }
    }
}

enum FibreCommand {
    /// Append component to root
    AppendRoot(Box<dyn FibreComponent>),

    /// Append as child of first parent node
    AppendChild(Node, Box<dyn FibreComponent>),

    /// Update style of node
    UpdateStyle(Node, Style),

    /// Remove node
    Retire(Node),
}

#[derive(Clone)]
pub struct FibreChannel {
    node: Node,
    sender: UnboundedSender<FibreCommand>,
}

impl FibreChannel {
    pub fn append_root(&self, component: impl FibreComponent + 'static) {
        self.sender
            .unbounded_send(FibreCommand::AppendRoot(Box::new(component)))
            .ok();
    }

    pub fn append_child(&self, component: impl FibreComponent + 'static) {
        self.sender
            .unbounded_send(FibreCommand::AppendChild(self.node, Box::new(component)))
            .ok();
    }

    pub fn update_style(&self, style: Style) {
        self.sender
            .unbounded_send(FibreCommand::UpdateStyle(self.node, style))
            .ok();
    }

    pub fn retire(&self) {
        self.sender
            .unbounded_send(FibreCommand::Retire(self.node))
            .ok();
    }
}

pub struct FibreNode<'a> {
    node: Node,
    fibre: &'a mut Fibre,
}

impl FibreNode<'_> {
    pub fn append_root(&mut self, component: impl FibreComponent + 'static) {
        self.fibre.append_root(Box::new(component));
    }

    pub fn append_child(&mut self, component: impl FibreComponent + 'static) {
        self.fibre.append_child(self.node, Box::new(component));
    }

    pub fn update_style(&mut self, style: Style) {
        self.fibre.update_style(self.node, style)
    }

    pub fn retire(&mut self) {
        self.fibre.retire(self.node);
    }

    pub fn create_channel(&self) -> FibreChannel {
        self.fibre.create_channel(self.node)
    }
}

pub fn run<Component: AsyncComponent + FibreElement + 'static>(
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

    async_component_winit::run(event_loop, Fibre::new(window, skia_window_ctx, component))
}
