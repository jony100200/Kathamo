//! Render-and-interact runtime for DesignDocument. Application-neutral: widgets emit actions;
//! the consuming application owns business logic and supplies/observes bound values.

use std::collections::HashMap;
use std::sync::Arc;

use ui_core::{Color, CornerRadius, Rect, Stroke, TextEditBuffer, Vec2};
use ui_platform::{Key, Modifiers, PointerButton, PointerState};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::{
    CanvasHost, HierarchyTree, NodeEditor, Timeline,
};
use ui_widgets::controls::{render_button, ButtonStyle};
use ui_widgets::input::{render_text_input, render_toggle};

use crate::components::{AssetBrowser, ConsoleView};
use crate::designer::{DesignDocument, DesignElement, DesignKind};

#[derive(Clone, Debug, PartialEq)]
pub enum UiValue {
    Text(String),
    Bool(bool),
    Number(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiEventKind {
    Activated,
    Changed(UiValue),
    Selected(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiEvent {
    pub component_id: String,
    pub action_id: String,
    pub kind: UiEventKind,
}

/// Consumers can supply a real widget implementation for a built-in kind or specific ID.
/// Coordinates are logical window coordinates and the runtime clips to the component bounds.
pub trait DesignWidget {
    fn render(&mut self, buf: &mut CommandBuffer, rect: Rect,
              ptr: &PointerState, palette: &ThemePalette);
    fn pointer_down(&mut self, _pos: Vec2, _button: PointerButton, _rect: Rect,
                    _ptr: &PointerState) -> Option<UiEventKind> { None }
    fn pointer_move(&mut self, _pos: Vec2, _delta: Vec2, _rect: Rect) {}
    fn pointer_up(&mut self, _pos: Vec2, _button: PointerButton, _rect: Rect) -> Option<UiEventKind> { None }
    fn scroll(&mut self, _delta: Vec2, _pos: Vec2, _rect: Rect) {}
    fn key(&mut self, _key: Key, _modifiers: Modifiers) -> Option<UiEventKind> { None }
    fn text(&mut self, _text: &str) -> Option<UiEventKind> { None }
}

type WidgetFactory = Arc<dyn Fn() -> Box<dyn DesignWidget> + Send + Sync>;

/// An extendible per-instance component registry.
#[derive(Default)]
pub struct ComponentRegistry {
    by_kind: HashMap<DesignKind, WidgetFactory>,
    by_id: HashMap<String, WidgetFactory>,
}

impl ComponentRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register_kind<F>(&mut self, kind: DesignKind, factory: F)
    where F: Fn() -> Box<dyn DesignWidget> + Send + Sync + 'static {
        self.by_kind.insert(kind, Arc::new(factory));
    }

    pub fn register_id<F>(&mut self, id: impl Into<String>, factory: F)
    where F: Fn() -> Box<dyn DesignWidget> + Send + Sync + 'static {
        self.by_id.insert(id.into(), Arc::new(factory));
    }

    fn factory(&self, node: &DesignElement) -> Option<&WidgetFactory> {
        self.by_id.get(&node.id).or_else(|| self.by_kind.get(&node.kind))
    }
}

/// Constraint rules are separate from the document's saved authored rectangles.
/// An optional rule adapts an element to its actual parent when the window resizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResponsiveRule {
    Absolute,
    /// Keep distances to the chosen edges in document units.
    Anchored { left: bool, right: bool, top: bool, bottom: bool,
               min_size: Vec2, max_size: Option<Vec2> },
    /// Fill the parent, excluding an inset on all sides.
    Fill { inset: f32 },
}

enum Instance {
    Custom(Box<dyn DesignWidget>),
    Graph(NodeEditor),
    Hierarchy(HierarchyTree),
    Assets(AssetBrowser),
    Console(ConsoleView),
    Timeline(Timeline),
    Canvas(CanvasHost),
}

impl Instance {
    fn from_kind(kind: DesignKind) -> Option<Self> {
        match kind {
            DesignKind::NodeGraph => {
                let mut editor = NodeEditor::default();
                editor.nodes.clear();
                editor.connections.clear();
                editor.groups.clear();
                Some(Self::Graph(editor))
            }
            DesignKind::Hierarchy => Some(Self::Hierarchy(HierarchyTree::new())),
            DesignKind::AssetBrowser => Some(Self::Assets(AssetBrowser::new())),
            DesignKind::Console => Some(Self::Console(ConsoleView::new())),
            DesignKind::Timeline => Some(Self::Timeline(Timeline::default())),
            DesignKind::Viewport => Some(Self::Canvas(CanvasHost::new_2d())),
            _ => None,
        }
    }

    fn render(&mut self, buf: &mut CommandBuffer, rect: Rect,
              ptr: &PointerState, pal: &ThemePalette) {
        match self {
            Self::Custom(w) => w.render(buf, rect, ptr, pal),
            Self::Graph(w) => w.render(buf, rect, pal),
            Self::Hierarchy(w) => w.render(buf, rect, pal),
            Self::Assets(w) => w.render(buf, rect, ptr, pal),
            Self::Console(w) => w.render(buf, rect, pal),
            Self::Timeline(w) => w.render(buf, rect, pal),
            Self::Canvas(w) => w.render(buf, rect, pal),
        }
    }

    fn down(&mut self, pos: Vec2, button: PointerButton,
            rect: Rect, ptr: &PointerState) -> Option<UiEventKind> {
        match self {
            Self::Custom(w) => w.pointer_down(pos, button, rect, ptr),
            Self::Graph(w) => { w.handle_mouse_down(pos, button, rect); None }
            Self::Hierarchy(w) => w.handle_mouse_down(pos, button, rect, ptr)
                .map(|v| UiEventKind::Selected(format!("{v:?}"))),
            Self::Assets(w) => w.handle_mouse_down(pos, button, rect).map(UiEventKind::Selected),
            Self::Canvas(w) => { w.handle_mouse_down(pos, button, rect); None }
            Self::Timeline(w) if button == PointerButton::Primary => {
                if pos.y <= rect.min.y + 28.0 {
                    let value = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                    w.scrub_to((value * w.total_frames as f32) as u32);
                    Some(UiEventKind::Changed(UiValue::Number(w.current_frame as f32)))
                } else { None }
            }
            _ => None,
        }
    }

    fn moving(&mut self, pos: Vec2, delta: Vec2, rect: Rect) {
        match self {
            Self::Custom(w) => w.pointer_move(pos, delta, rect),
            Self::Graph(w) => w.handle_mouse_move(pos, delta, rect),
            Self::Canvas(w) => {
                let ptr = PointerState { primary_down: true, ..PointerState::default() };
                w.handle_mouse_move(pos, delta, rect, &ptr);
            }
            _ => {}
        }
    }

    fn up(&mut self, pos: Vec2, button: PointerButton, rect: Rect) -> Option<UiEventKind> {
        match self {
            Self::Custom(w) => w.pointer_up(pos, button, rect),
            Self::Graph(w) => { w.handle_mouse_up(pos, button, rect); None }
            Self::Canvas(w) => { w.handle_mouse_up(pos, button); None }
            _ => None,
        }
    }

    fn wheel(&mut self, delta: Vec2, pos: Vec2, rect: Rect) {
        match self {
            Self::Custom(w) => w.scroll(delta, pos, rect),
            Self::Graph(w) => w.handle_scroll(delta, pos, rect),
            Self::Canvas(w) => { w.handle_scroll(delta, pos, rect); }
            _ => {}
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) -> Option<UiEventKind> {
        match self {
            Self::Custom(w) => w.key(key, modifiers),
            Self::Graph(w) => { w.handle_key_down(key); None }
            _ => None,
        }
    }

    fn text(&mut self, text: &str) -> Option<UiEventKind> {
        match self {
            Self::Custom(w) => w.text(text),
            _ => None,
        }
    }
}

/// A live runtime for designer-generated documents. No model runtime or app backend is embedded.
pub struct DesignRuntime {
    pub registry: ComponentRegistry,
    pub values: HashMap<String, UiValue>,
    /// component stable ID -> external state key
    pub bindings: HashMap<String, String>,
    /// component stable ID -> external action ID
    pub actions: HashMap<String, String>,
    pub responsive: HashMap<String, ResponsiveRule>,
    pub focused: Option<String>,
    instances: HashMap<String, (DesignKind, Instance)>,
    text_inputs: HashMap<String, TextEditBuffer>,
    pressed: Option<String>,
    events: Vec<UiEvent>,
    // Logical document-sized layout plus optional rule overrides.
    computed: HashMap<String, Rect>,
    bounds: Rect,
    root_scale: Vec2,
}

impl Default for DesignRuntime {
    fn default() -> Self { Self::new() }
}

impl DesignRuntime {
    pub fn new() -> Self {
        Self {
            registry: ComponentRegistry::new(), values: HashMap::new(),
            bindings: HashMap::new(), actions: HashMap::new(),
            responsive: HashMap::new(), focused: None, instances: HashMap::new(),
            text_inputs: HashMap::new(), pressed: None, events: Vec::new(),
            computed: HashMap::new(),
            bounds: Rect::from_min_size(Vec2::ZERO, Vec2::new(1200.0, 760.0)),
            root_scale: Vec2::new(1.0, 1.0),
        }
    }

    pub fn bind(&mut self, component_id: impl Into<String>, key: impl Into<String>) {
        self.bindings.insert(component_id.into(), key.into());
    }

    pub fn bind_action(&mut self, component_id: impl Into<String>, action_id: impl Into<String>) {
        self.actions.insert(component_id.into(), action_id.into());
    }

    fn state_key<'a>(&'a self, id: &'a str) -> &'a str {
        self.bindings.get(id).map(String::as_str).unwrap_or(id)
    }

    pub fn set(&mut self, key: impl Into<String>, value: UiValue) {
        self.values.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<&UiValue> { self.values.get(key) }
    pub fn take_events(&mut self) -> Vec<UiEvent> { std::mem::take(&mut self.events) }

    /// Return mutable real widgets for data population by consuming applications.
    pub fn graph(&mut self, id: &str) -> Option<&mut NodeEditor> {
        match self.instances.get_mut(id) {
            Some((_, Instance::Graph(graph))) => Some(graph), _ => None,
        }
    }

    pub fn hierarchy(&mut self, id: &str) -> Option<&mut HierarchyTree> {
        match self.instances.get_mut(id) {
            Some((_, Instance::Hierarchy(tree))) => Some(tree), _ => None,
        }
    }

    pub fn assets(&mut self, id: &str) -> Option<&mut AssetBrowser> {
        match self.instances.get_mut(id) {
            Some((_, Instance::Assets(browser))) => Some(browser), _ => None,
        }
    }

    pub fn canvas(&mut self, id: &str) -> Option<&mut CanvasHost> {
        match self.instances.get_mut(id) {
            Some((_, Instance::Canvas(canvas))) => Some(canvas), _ => None,
        }
    }

    pub fn sync(&mut self, doc: &DesignDocument) {
        // Explicit consumer bindings take precedence; document defaults are applied once.
        for (id, key) in &doc.state_bindings {
            self.bindings.entry(id.clone()).or_insert_with(|| key.clone());
        }
        for (id, action) in &doc.action_bindings {
            self.actions.entry(id.clone()).or_insert_with(|| action.clone());
        }
        self.instances.retain(|id, _| doc.find(id).is_some());
        self.text_inputs.retain(|id, _| doc.find(id).is_some());
        for node in &doc.elements {
            let recreate = self.instances.get(&node.id)
                .is_some_and(|(kind, _)| *kind != node.kind);
            if recreate { self.instances.remove(&node.id); }
            if !self.instances.contains_key(&node.id) {
                let instance = self.registry.factory(node).map(|f| Instance::Custom(f()))
                    .or_else(|| Instance::from_kind(node.kind));
                if let Some(value) = instance {
                    self.instances.insert(node.id.clone(), (node.kind, value));
                }
            }
            if node.kind == DesignKind::TextInput {
                self.text_inputs.entry(node.id.clone()).or_insert_with(|| {
                    TextEditBuffer::new("")
                });
            }
        }
    }

    /// Recompute real component bounds. Layout rules use document coordinates and
    /// preserve parent-relative positioning. The root scales to its assigned window.
    pub fn relayout(&mut self, doc: &DesignDocument, bounds: Rect) {
        self.bounds = bounds;
        // Uniform DPI-safe document scale leaves any extra width/height available
        // for anchored or fill-constrained components rather than stretching fonts.
        let scale = (bounds.width() / doc.width.max(1.0))
            .min(bounds.height() / doc.height.max(1.0)).max(0.01);
        self.root_scale = Vec2::splat(scale);
        self.computed.clear();
        // Parent ordering is not required for loading, so resolve recursively.
        for node in &doc.elements {
            self.resolve(doc, &node.id, 0);
        }
    }

    fn resolve(&mut self, doc: &DesignDocument, id: &str, depth: usize) -> Option<Rect> {
        if let Some(r) = self.computed.get(id) { return Some(*r); }
        if depth > doc.elements.len() { return None; }
        let node = doc.find(id)?;
        let parent = if let Some(parent_id) = &node.parent {
            self.resolve(doc, parent_id, depth + 1)?
        } else { self.bounds };
        let authored_parent = if let Some(parent_id) = &node.parent {
            doc.find(parent_id)?.bounds.size()
        } else { Vec2::new(doc.width, doc.height) };
        let authored = node.bounds;
        let scaled = Vec2::new(authored.width() * self.root_scale.x,
                               authored.height() * self.root_scale.y);
        let parent_authored_size = Vec2::new(
            authored_parent.x * self.root_scale.x, authored_parent.y * self.root_scale.y);
        let size_delta = parent.size() - parent_authored_size;
        let mut pos = parent.min + Vec2::new(
            authored.min.x * self.root_scale.x, authored.min.y * self.root_scale.y);
        let mut size = scaled;
        match self.responsive.get(id).copied().unwrap_or(ResponsiveRule::Absolute) {
            ResponsiveRule::Absolute => {}
            ResponsiveRule::Fill { inset } => {
                let inset = inset.max(0.0);
                let scaled = Vec2::new(inset * self.root_scale.x, inset * self.root_scale.y);
                pos = parent.min + scaled;
                size = Vec2::new((parent.width() - scaled.x * 2.0).max(1.0),
                                 (parent.height() - scaled.y * 2.0).max(1.0));
            }
            ResponsiveRule::Anchored { left, right, top, bottom, min_size, max_size } => {
                if left && right { size.x += size_delta.x; }
                else if right { pos.x += size_delta.x; }
                if top && bottom { size.y += size_delta.y; }
                else if bottom { pos.y += size_delta.y; }
                size.x = size.x.max(min_size.x * self.root_scale.x).max(1.0);
                size.y = size.y.max(min_size.y * self.root_scale.y).max(1.0);
                if let Some(max) = max_size {
                    size.x = size.x.min(max.x.max(min_size.x) * self.root_scale.x);
                    size.y = size.y.min(max.y.max(min_size.y) * self.root_scale.y);
                }
            }
        }
        let rect = Rect::from_min_size(pos, size);
        self.computed.insert(id.to_owned(), rect);
        Some(rect)
    }

    pub fn component_bounds(&self, id: &str) -> Option<Rect> { self.computed.get(id).copied() }

    /// Render actual registered widgets and standard interactive controls, not Designer mock shapes.
    pub fn render(&mut self, doc: &DesignDocument, buf: &mut CommandBuffer,
                  bounds: Rect, ptr: &PointerState, pal: &ThemePalette) {
        self.sync(doc);
        self.relayout(doc, bounds);
        buf.push(DrawCommand::ClipPush { rect: bounds });
        for node in &doc.elements {
            if !node.visible { continue; }
            let Some(rect) = self.component_bounds(&node.id) else { continue };
            if !rect.intersects(bounds) { continue; }
            buf.push(DrawCommand::ClipPush { rect });
            if let Some((_, instance)) = self.instances.get_mut(&node.id) {
                instance.render(buf, rect, ptr, pal);
            } else {
                let value = self.values.get(self.bindings.get(&node.id).map(String::as_str)
                    .unwrap_or(&node.id));
                match node.kind {
                    DesignKind::Button => render_button(buf, rect, &node.label, ButtonStyle::Secondary,
                                                        rect.contains(ptr.position), pal),
                    DesignKind::Toggle => render_toggle(buf,
                        matches!(value, Some(UiValue::Bool(true))),
                        rect.contains(ptr.position), &node.label, rect, pal),
                    DesignKind::TextInput => render_text_input(buf,
                        match value { Some(UiValue::Text(s)) => s.as_str(), _ => "" },
                        &node.label, rect,
                        self.focused.as_deref() == Some(node.id.as_str()),
                        rect.contains(ptr.position), pal),
                    _ => {
                        buf.push(DrawCommand::Rect { rect, fill: node.background,
                            radius: CornerRadius::same(3.0),
                            stroke: Stroke::new(1.0, pal.border) });
                        buf.push(DrawCommand::Text { text: node.label.clone(),
                            pos: rect.min + Vec2::new(8.0, 7.0), size: 12.0,
                            color: node.foreground, strong: true });
                    }
                }
            }
            buf.push(DrawCommand::ClipPop);
        }
        buf.push(DrawCommand::ClipPop);
    }

    /// Topmost focusable component; nearest children take precedence when nested.
    fn hit<'a>(&self, doc: &'a DesignDocument, pos: Vec2) -> Option<&'a DesignElement> {
        doc.elements.iter().rev().find(|node| node.visible &&
            self.component_bounds(&node.id).is_some_and(|r| r.contains(pos)))
    }

    fn emit(&mut self, id: &str, kind: UiEventKind) {
        let action_id = self.actions.get(id).cloned().unwrap_or_else(|| id.into());
        self.events.push(UiEvent { component_id: id.into(), action_id, kind });
    }

    pub fn pointer_down(&mut self, doc: &DesignDocument, pos: Vec2,
                        button: PointerButton, ptr: &PointerState) -> bool {
        let Some(node) = self.hit(doc, pos) else {
            self.focused = None;
            self.pressed = None;
            return false;
        };
        let id = node.id.clone();
        let rect = self.component_bounds(&id).unwrap();
        self.focused = Some(id.clone());
        self.pressed = Some(id.clone());
        if let Some((_, instance)) = self.instances.get_mut(&id) {
            if let Some(event) = instance.down(pos, button, rect, ptr) {
                self.emit(&id, event);
            }
        }
        true
    }

    pub fn pointer_move(&mut self, pos: Vec2, delta: Vec2) {
        if let Some(id) = self.pressed.clone() {
            if let (Some(rect), Some((_, instance))) =
                (self.component_bounds(&id), self.instances.get_mut(&id)) {
                instance.moving(pos, delta, rect);
            }
        }
    }

    pub fn pointer_up(&mut self, doc: &DesignDocument, pos: Vec2, button: PointerButton) -> bool {
        let Some(id) = self.pressed.take() else { return false; };
        let Some(rect) = self.component_bounds(&id) else { return false; };
        if let Some((_, instance)) = self.instances.get_mut(&id) {
            if let Some(event) = instance.up(pos, button, rect) { self.emit(&id, event); }
        } else if button == PointerButton::Primary && rect.contains(pos) {
            match doc.find(&id).map(|n| n.kind) {
                Some(DesignKind::Button) => self.emit(&id, UiEventKind::Activated),
                Some(DesignKind::Toggle) => {
                    let key = self.state_key(&id).to_owned();
                    let state = !matches!(self.values.get(&key), Some(UiValue::Bool(true)));
                    self.values.insert(key, UiValue::Bool(state));
                    self.emit(&id, UiEventKind::Changed(UiValue::Bool(state)));
                }
                _ => {}
            }
        }
        true
    }

    pub fn scroll(&mut self, doc: &DesignDocument, delta: Vec2, pos: Vec2) -> bool {
        let Some(id) = self.hit(doc, pos).map(|n| n.id.clone()) else { return false; };
        if let (Some(rect), Some((_, instance))) =
            (self.component_bounds(&id), self.instances.get_mut(&id)) {
            instance.wheel(delta, pos, rect);
            return true;
        }
        false
    }

    pub fn key_down(&mut self, doc: &DesignDocument, key: Key, modifiers: Modifiers) -> bool {
        if key == Key::Tab {
            let controls: Vec<&str> = doc.elements.iter()
                .filter(|n| n.visible && matches!(n.kind,
                    DesignKind::Button | DesignKind::Toggle | DesignKind::TextInput))
                .map(|n| n.id.as_str()).collect();
            if controls.is_empty() { return false; }
            let index = self.focused.as_ref()
                .and_then(|id| controls.iter().position(|x| *x == id))
                .unwrap_or(if modifiers.shift { 0 } else { controls.len() - 1 });
            let next = if modifiers.shift {
                (index + controls.len() - 1) % controls.len()
            } else { (index + 1) % controls.len() };
            self.focused = Some(controls[next].to_owned());
            return true;
        }
        let Some(id) = self.focused.clone() else { return false; };
        if let Some((_, instance)) = self.instances.get_mut(&id) {
            if let Some(event) = instance.key(key.clone(), modifiers) {
                self.emit(&id, event);
                return true;
            }
        }
        let kind = doc.find(&id).map(|n| n.kind);
        match (kind, &key) {
            (Some(DesignKind::Button), Key::Enter | Key::Space) =>
                self.emit(&id, UiEventKind::Activated),
            (Some(DesignKind::Toggle), Key::Enter | Key::Space) => {
                let k = self.state_key(&id).to_owned();
                let checked = !matches!(self.values.get(&k), Some(UiValue::Bool(true)));
                self.values.insert(k, UiValue::Bool(checked));
                self.emit(&id, UiEventKind::Changed(UiValue::Bool(checked)));
            }
            (Some(DesignKind::TextInput), Key::Backspace | Key::Delete |
                 Key::ArrowLeft | Key::ArrowRight) => {
                let k = self.state_key(&id).to_owned();
                let initial = match self.values.get(&k) { Some(UiValue::Text(v)) => v.clone(),
                    _ => String::new() };
                let editor = self.text_inputs.entry(id.clone())
                    .or_insert_with(|| TextEditBuffer::new(""));
                if editor.as_str() != initial { editor.set_text(initial); }
                // TextEditBuffer keeps cursor/selection on Unicode scalar boundaries.
                match key {
                    Key::Backspace => { editor.backspace(); }
                    Key::Delete => { editor.delete(); }
                    Key::ArrowLeft => editor.move_left(modifiers.shift),
                    Key::ArrowRight => editor.move_right(modifiers.shift),
                    _ => {}
                }
                let changed = UiValue::Text(editor.as_str().to_owned());
                self.values.insert(k, changed.clone());
                self.emit(&id, UiEventKind::Changed(changed));
            }
            _ => return false,
        }
        true
    }

    pub fn text_input(&mut self, doc: &DesignDocument, text: &str) -> bool {
        let Some(id) = self.focused.clone() else { return false; };
        if let Some((_, instance)) = self.instances.get_mut(&id) {
            if let Some(event) = instance.text(text) { self.emit(&id, event); return true; }
        }
        if doc.find(&id).map(|n| n.kind) != Some(DesignKind::TextInput) {
            return false;
        }
        let k = self.state_key(&id).to_owned();
        let initial = match self.values.get(&k) { Some(UiValue::Text(v)) => v.clone(),
            _ => String::new() };
        let editor = self.text_inputs.entry(id.clone())
            .or_insert_with(|| TextEditBuffer::new(""));
        if editor.as_str() != initial { editor.set_text(initial); }
        editor.insert_str(text);
        let changed = UiValue::Text(editor.as_str().to_owned());
        self.values.insert(k, changed.clone());
        self.emit(&id, UiEventKind::Changed(changed));
        true
    }

    /// Semantic, screen-position based component inspection for UI agents.
    pub fn inspect(&self, doc: &DesignDocument) -> Vec<(String, DesignKind, Rect, bool)> {
        doc.elements.iter().filter_map(|n|
            self.component_bounds(&n.id).map(|r|
                (n.id.clone(), n.kind, r, self.focused.as_deref() == Some(n.id.as_str()))
            )).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::Vec2;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, h))
    }
    #[test]
    fn button_and_bound_toggle_are_real_actions() {
        let mut doc = DesignDocument::new("Test", 800.0, 600.0);
        let button = doc.add(DesignKind::Button, "Run", rect(10.0, 10.0, 100.0, 30.0));
        let toggle = doc.add(DesignKind::Toggle, "Enabled", rect(10.0, 80.0, 120.0, 30.0));
        let mut runtime = DesignRuntime::new();
        runtime.bind_action(&button, "command.run");
        runtime.bind(&toggle, "prefs.enabled");
        runtime.relayout(&doc, rect(0.0, 0.0, 800.0, 600.0));
        let pointer = PointerState::default();
        assert!(runtime.pointer_down(&doc, Vec2::new(20.0, 20.0), PointerButton::Primary, &pointer));
        runtime.pointer_up(&doc, Vec2::new(20.0, 20.0), PointerButton::Primary);
        assert_eq!(runtime.take_events()[0].action_id, "command.run");
        runtime.pointer_down(&doc, Vec2::new(20.0, 90.0), PointerButton::Primary, &pointer);
        runtime.pointer_up(&doc, Vec2::new(20.0, 90.0), PointerButton::Primary);
        assert_eq!(runtime.get("prefs.enabled"), Some(&UiValue::Bool(true)));
    }

    #[test]
    fn responsive_rule_and_keyboard_text() {
        let mut doc = DesignDocument::new("Test", 800.0, 600.0);
        let id = doc.add(DesignKind::TextInput, "Prompt", rect(50.0, 30.0, 400.0, 30.0));
        let mut rt = DesignRuntime::new();
        rt.responsive.insert(id.clone(), ResponsiveRule::Anchored {
            left: true, right: true, top: true, bottom: false,
            min_size: Vec2::new(120.0, 30.0), max_size: None,
        });
        rt.relayout(&doc, rect(0.0, 0.0, 1000.0, 600.0));
        assert_eq!(rt.component_bounds(&id).unwrap().width(), 600.0);
        rt.focused = Some(id.clone());
        assert!(rt.text_input(&doc, "বাংলা hello"));
        assert_eq!(rt.get(&id), Some(&UiValue::Text("বাংলা hello".into())));
    }
}
