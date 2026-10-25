//! Native Kathamo Visual Designer. Uses the exact DesignDocument model that consuming apps use.
//! Run: cargo run -p kathamo --example visual_designer

use std::path::PathBuf;
use kathamo::prelude::*;

const PALETTE_X: f32 = 10.0;
const PALETTE_Y: f32 = 142.0;
const PALETTE_ROW: f32 = 27.0;
const TREE_Y: f32 = 585.0;
const PROP_Y: f32 = 190.0;
const PROP_ROW: f32 = 39.0;
const TOP_ACTIONS: [&str; 12] = [
    "New", "Unity", "Blender", "Krita", "Chat", "Save", "Open",
    "Export Rust", "Preview", "Undo", "Redo", "Delete",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditField { Name, Kind, Parent, Visible, X, Y, Width, Height, Background, Foreground, Placement, StateBinding, ActionBinding, Path }

impl EditField {
    const ALL: [Self; 13] = [
        Self::Name, Self::Kind, Self::Parent, Self::Visible,
        Self::X, Self::Y, Self::Width, Self::Height,
        Self::Background, Self::Foreground, Self::Placement,
        Self::StateBinding, Self::ActionBinding,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Label",
            Self::Kind => "Component type",
            Self::Parent => "Parent ID",
            Self::Visible => "Visible",
            Self::X => "X",
            Self::Y => "Y",
            Self::Width => "Width",
            Self::Height => "Height",
            Self::Background => "Background",
            Self::Foreground => "Text colour",
            Self::StateBinding => "State key",
            Self::ActionBinding => "Action ID",
            Self::Placement => "Layout",
            Self::Path => "Project file",
        }
    }
}

#[derive(Clone, Copy)]
struct CanvasGeometry {
    frame: Rect,
    page: Rect,
    origin: Vec2,
    scale: f32,
}

#[derive(Clone)]
struct Dragging {
    id: String,
    resize: bool,
}

/// Visual editing state; the exported application only needs DesignDocument.
pub struct VisualDesigner {
    pub document: DesignDocument,
    pub runtime: DesignRuntime,
    pub selected: Option<String>,
    pub save_path: PathBuf,
    pub preview: bool,
    pub zoom: f32,
    pub status: String,
    pending_kind: Option<DesignKind>,
    palette_drag: Option<DesignKind>,
    drag: Option<Dragging>,
    editing: Option<(EditField, String)>,
    undo: Vec<DesignDocument>,
    redo: Vec<DesignDocument>,
}

impl Default for VisualDesigner {
    fn default() -> Self { Self::new() }
}

impl VisualDesigner {
    pub fn new() -> Self {
        let project_folder = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("KathamoProjects");
        Self {
            document: DesignDocument::template(DesignTemplate::Unity),
            runtime: DesignRuntime::new(),
            selected: None,
            save_path: project_folder.join("untitled.kui"),
            preview: false,
            zoom: 1.0,
            status: "Ready  |  Drag components onto the canvas, or click to place".into(),
            pending_kind: None,
            palette_drag: None,
            drag: None,
            editing: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    fn canvas_geometry(&self, window: Rect) -> CanvasGeometry {
        let frame = if self.preview {
            Rect::from_min_size(Vec2::new(8.0, 64.0),
                Vec2::new((window.width() - 16.0).max(320.0),
                    (window.height() - 105.0).max(280.0)))
        } else {
            Rect::from_min_size(Vec2::new(233.0, 74.0),
                Vec2::new((window.width() - 533.0).max(320.0),
                    (window.height() - 110.0).max(280.0)))
        };
        let fit = ((frame.width() - 56.0) / self.document.width)
            .min((frame.height() - 56.0) / self.document.height)
            .max(0.1);
        let scale = fit * self.zoom;
        let content = Vec2::new(self.document.width, self.document.height) * scale;
        let origin = frame.center() - content * 0.5;
        CanvasGeometry {
            frame,
            page: Rect::from_min_size(origin, content),
            origin,
            scale,
        }
    }

    fn canvas_point(&self, pos: Vec2, window: Rect) -> Vec2 {
        let g = self.canvas_geometry(window);
        (pos - g.origin) * (1.0 / g.scale)
    }

    fn note_change(&mut self) {
        if self.undo.len() >= 80 { self.undo.remove(0); }
        self.undo.push(self.document.clone());
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.document, previous));
            self.selected = None;
            self.status = "Undo".into();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.document, next));
            self.selected = None;
            self.status = "Redo".into();
        }
    }

    fn select_template(&mut self, template: DesignTemplate) {
        self.note_change();
        self.document = DesignDocument::template(template);
        self.selected = None;
        self.pending_kind = None;
        self.zoom = 1.0;
        self.status = format!("{} UI template loaded", template.name());
    }

    fn save(&mut self) {
        let result = self.save_path.parent()
            .map(|p| std::fs::create_dir_all(p).map_err(|e| e.to_string()))
            .transpose()
            .and_then(|_| self.document.save(&self.save_path));
        self.status = match result {
            Ok(()) => format!("Saved {}", self.save_path.display()),
            Err(error) => format!("Save failed: {error}"),
        };
    }

    fn open(&mut self) {
        match DesignDocument::load(&self.save_path) {
            Ok(doc) => {
                self.note_change();
                self.document = doc;
                self.selected = None;
                self.zoom = 1.0;
                self.status = format!("Loaded {}", self.save_path.display());
            }
            Err(error) => self.status = format!("Open failed: {error}"),
        }
    }

    fn export(&mut self) {
        let path = self.save_path.with_extension("rs");
        let result = path.parent()
            .map(|p| std::fs::create_dir_all(p).map_err(|e| e.to_string()))
            .transpose()
            .and_then(|_| self.document.export_to(&path));
        self.status = match result {
            Ok(()) => format!("Rust exported: {}", path.display()),
            Err(error) => format!("Export failed: {error}"),
        };
    }

    fn toolbar_action(&mut self, index: usize) {
        match index {
            0 => self.select_template(DesignTemplate::Empty),
            1 => self.select_template(DesignTemplate::Unity),
            2 => self.select_template(DesignTemplate::Blender),
            3 => self.select_template(DesignTemplate::Krita),
            4 => self.select_template(DesignTemplate::ChatApp),
            5 => self.save(),
            6 => self.open(),
            7 => self.export(),
            8 => { self.preview = !self.preview; self.status = if self.preview {
                "Preview: click Preview or press Esc to edit".into()
            } else { "Edit mode".into() }; }
            9 => self.undo(),
            10 => self.redo(),
            11 => self.delete_selected(),
            _ => {}
        }
    }

    fn delete_selected(&mut self) {
        let Some(id) = self.selected.clone() else { return; };
        self.note_change();
        self.document.remove(&id);
        self.selected = None;
        self.status = "Component deleted".into();
    }

    fn add_component(&mut self, kind: DesignKind, pos: Vec2) {
        let size = kind.default_size();
        let max = Vec2::new(
            (self.document.width - size.x).max(0.0),
            (self.document.height - size.y).max(0.0),
        );
        let top_left = Vec2::new(
            (pos.x - size.x * 0.5).clamp(0.0, max.x),
            (pos.y - size.y * 0.5).clamp(0.0, max.y),
        );
        self.note_change();
        let id = self.document.add(kind, kind.name(),
            Rect::from_min_size(top_left, size));
        self.selected = Some(id);
        self.pending_kind = None;
        self.status = format!("Added {}", kind.name());
    }

    fn selected_value(&self, field: EditField) -> String {
        if field == EditField::Path {
            return self.save_path.to_string_lossy().into_owned();
        }
        let Some(node) = self.selected.as_deref().and_then(|id| self.document.find(id)) else {
            return String::new();
        };
        match field {
            EditField::Name => node.label.clone(),
            EditField::Kind => node.kind.name().into(),
            EditField::Parent => node.parent.clone().unwrap_or_default(),
            EditField::Visible => node.visible.to_string(),
            EditField::X => format!("{:.0}", node.bounds.min.x),
            EditField::Y => format!("{:.0}", node.bounds.min.y),
            EditField::Width => format!("{:.0}", node.bounds.width()),
            EditField::Height => format!("{:.0}", node.bounds.height()),
            EditField::Background => color_text(node.background),
            EditField::Foreground => color_text(node.foreground),
            EditField::StateBinding => self.document.state_bindings.get(&node.id).cloned().unwrap_or_default(),
            EditField::ActionBinding => self.document.action_bindings.get(&node.id).cloned().unwrap_or_default(),
            EditField::Placement => self.document.layout_rules.get(&node.id).copied()
                .unwrap_or_default().name().into(),
            EditField::Path => unreachable!(),
        }
    }

    fn edit(&mut self, field: EditField) {
        self.editing = Some((field, self.selected_value(field)));
        self.status = format!("Editing {}  |  Enter to apply  |  Esc to cancel", field.label());
    }

    fn commit_edit(&mut self) {
        let Some((field, value)) = self.editing.take() else { return; };
        if field == EditField::Path {
            if !value.trim().is_empty() {
                self.save_path = PathBuf::from(value);
                self.status = "Project path changed".into();
            }
            return;
        }
        let Some(id) = self.selected.clone() else { return; };
        if field == EditField::Placement {
            match DesignPlacement::parse(value.trim()) {
                Some(mode) => {
                    self.note_change();
                    self.document.layout_rules.insert(id, mode);
                    self.status = format!("Layout: {}", mode.name());
                }
                None => self.status = "Layout: Absolute / Fill / StretchX / StretchY / StretchBoth / Right / Bottom / BottomRight".into(),
            }
            return;
        }
        if matches!(field, EditField::StateBinding | EditField::ActionBinding) {
            self.note_change();
            let table = if field == EditField::StateBinding {
                &mut self.document.state_bindings
            } else { &mut self.document.action_bindings };
            if value.trim().is_empty() { table.remove(&id); }
            else { table.insert(id, value); }
            self.status = format!("Updated {}", field.label());
            return;
        }
        if field == EditField::Parent {
            let mut proposed = self.document.clone();
            let parent = value.trim();
            match proposed.set_parent(&id, if parent.is_empty() { None } else { Some(parent) }) {
                Ok(()) => {
                    self.note_change();
                    self.document = proposed;
                    self.status = "Parent changed".into();
                }
                Err(error) => self.status = format!("Parent rejected: {error}"),
            }
            return;
        }
        let new_kind = if field == EditField::Kind {
            match DesignKind::parse(&value) {
                Some(kind) => Some(kind),
                None => {
                    self.status = "Unknown type; use the palette component names".into();
                    return;
                }
            }
        } else { None };
        let new_visible = if field == EditField::Visible {
            match value.to_ascii_lowercase().as_str() {
                "true" | "yes" | "1" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => { self.status = "Visible accepts true or false".into(); return; }
            }
        } else { None };
        let numeric = match field {
            EditField::X | EditField::Y | EditField::Width | EditField::Height =>
                match value.parse::<f32>() {
                    Ok(n) if n.is_finite() && n.abs() < 100_000.0 => Some(n),
                    _ => { self.status = "Invalid number".into(); return; }
                },
            _ => None,
        };
        let color = match field {
            EditField::Background | EditField::Foreground => match parse_color(&value) {
                Some(c) => Some(c),
                None => { self.status = "Colour must be #RRGGBB".into(); return; }
            },
            _ => None,
        };
        self.note_change();
        if let Some(node) = self.document.find_mut(&id) {
            match field {
                EditField::Name => node.label = value,
                EditField::Kind => node.kind = new_kind.unwrap(),
                EditField::Parent | EditField::Placement | EditField::StateBinding | EditField::ActionBinding => unreachable!(),
                EditField::Visible => node.visible = new_visible.unwrap(),
                EditField::X => node.bounds = Rect::from_min_size(
                    Vec2::new(numeric.unwrap(), node.bounds.min.y), node.bounds.size()),
                EditField::Y => node.bounds = Rect::from_min_size(
                    Vec2::new(node.bounds.min.x, numeric.unwrap()), node.bounds.size()),
                EditField::Width => node.bounds = Rect::from_min_size(
                    node.bounds.min, Vec2::new(numeric.unwrap().max(8.0), node.bounds.height())),
                EditField::Height => node.bounds = Rect::from_min_size(
                    node.bounds.min, Vec2::new(node.bounds.width(), numeric.unwrap().max(8.0))),
                EditField::Background => node.background = color.unwrap(),
                EditField::Foreground => node.foreground = color.unwrap(),
                EditField::Path => {}
            }
            self.status = format!("Updated {}", field.label());
        }
    }

    fn top_button_rect(index: usize) -> Rect {
        Rect::from_min_size(Vec2::new(262.0 + index as f32 * 91.0, 11.0),
                            Vec2::new(86.0, 31.0))
    }

    fn palette_row(kind_idx: usize) -> Rect {
        Rect::from_min_size(Vec2::new(PALETTE_X, PALETTE_Y + kind_idx as f32 * PALETTE_ROW),
                            Vec2::new(206.0, 25.0))
    }

    fn property_row(index: usize, win: Rect) -> Rect {
        Rect::from_min_size(Vec2::new(win.max.x - 286.0,
            PROP_Y + index as f32 * PROP_ROW), Vec2::new(272.0, 36.0))
    }

    fn handle_primary_down(&mut self, pos: Vec2, window: Rect) -> bool {
        if pos.y < 57.0 {
            for index in 0..TOP_ACTIONS.len() {
                if Self::top_button_rect(index).contains(pos) {
                    self.toolbar_action(index);
                    return true;
                }
            }
        }
        if self.preview {
            return true;
        }
        if pos.x < 225.0 {
            for (i, kind) in DesignKind::ALL.iter().enumerate() {
                if Self::palette_row(i).contains(pos) {
                    self.pending_kind = Some(*kind);
                    self.palette_drag = Some(*kind);
                    self.status = format!("Place {} on canvas", kind.name());
                    return true;
                }
            }
            if pos.y >= TREE_Y + 42.0 {
                let index = ((pos.y - TREE_Y - 42.0) / 25.0).floor() as usize;
                if let Some(node) = self.document.elements.get(index) {
                    self.selected = Some(node.id.clone());
                    self.status = format!("Selected {}", node.label);
                    return true;
                }
            }
        }
        if pos.x > window.max.x - 295.0 {
            let path = Rect::from_min_size(
                Vec2::new(window.max.x - 287.0, 128.0), Vec2::new(270.0, 31.0));
            if path.contains(pos) {
                self.edit(EditField::Path);
                return true;
            }
            for (i, field) in EditField::ALL.iter().enumerate() {
                if Self::property_row(i, window).contains(pos) {
                    if self.selected.is_some() { self.edit(*field); }
                    return true;
                }
            }
        }
        let canvas = self.canvas_geometry(window);
        if canvas.page.contains(pos) {
            let point = self.canvas_point(pos, window);
            if let Some(kind) = self.pending_kind.take() {
                self.palette_drag = None;
                self.add_component(kind, point);
                return true;
            }
            if let Some(node) = self.document.hit_test(point) {
                let id = node.id.clone();
                let size = self.document.absolute_bounds(&id).unwrap();
                let resize = (point.x - size.max.x).abs() < 14.0 / canvas.scale &&
                             (point.y - size.max.y).abs() < 14.0 / canvas.scale;
                self.selected = Some(id.clone());
                self.note_change();
                self.drag = Some(Dragging { id, resize });
                self.status = "Drag to move; drag lower-right handle to resize".into();
            } else {
                self.selected = None;
            }
            return true;
        }
        false
    }

    fn handle_drag(&mut self, pos: Vec2, delta: Vec2, window: Rect) -> bool {
        if self.palette_drag.is_some() { return true; }
        let canvas = self.canvas_geometry(window);
        if let Some(drag) = self.drag.as_ref() {
            if let Some(node) = self.document.find_mut(&drag.id) {
                let movement = delta * (1.0 / canvas.scale);
                if drag.resize {
                    let b = node.bounds;
                    node.bounds = Rect::from_min_size(b.min,
                        Vec2::new((b.width() + movement.x).max(30.0),
                            (b.height() + movement.y).max(24.0)));
                } else {
                    let b = node.bounds;
                    node.bounds = Rect::from_min_size(b.min + movement, b.size());
                }
            }
            return true;
        }
        let _ = pos;
        false
    }

    fn handle_release(&mut self, pos: Vec2, window: Rect) -> bool {
        if let Some(kind) = self.palette_drag.take() {
            let geometry = self.canvas_geometry(window);
            if geometry.page.contains(pos) {
                self.add_component(kind, self.canvas_point(pos, window));
            }
            return true;
        }
        self.drag.take().is_some()
    }

    fn handle_shortcut(&mut self, key: Key, mods: Modifiers) -> bool {
        if self.editing.is_some() {
            match key {
                Key::Enter => self.commit_edit(),
                Key::Escape => { self.editing = None; self.status = "Edit cancelled".into(); },
                Key::Backspace => { if let Some((_, value)) = &mut self.editing { value.pop(); } },
                _ => return true,
            }
            return true;
        }
        if mods.ctrl || mods.command {
            match key {
                Key::Character('s' | 'S') => self.save(),
                Key::Character('o' | 'O') => self.open(),
                Key::Character('e' | 'E') => self.export(),
                Key::Character('z' | 'Z') if !mods.shift => self.undo(),
                Key::Character('y' | 'Y') => self.redo(),
                Key::Character('z' | 'Z') if mods.shift => self.redo(),
                _ => return false,
            }
            return true;
        }
        match key {
            Key::Delete | Key::Backspace => { self.delete_selected(); true }
            Key::Escape => {
                self.preview = false;
                self.pending_kind = None;
                self.selected = None;
                true
            }
            Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown => {
                if let Some(id) = self.selected.clone() {
                    self.note_change();
                    if let Some(node) = self.document.find_mut(&id) {
                        let d = match key {
                            Key::ArrowLeft => Vec2::new(-1.0, 0.0),
                            Key::ArrowRight => Vec2::new(1.0, 0.0),
                            Key::ArrowUp => Vec2::new(0.0, -1.0),
                            _ => Vec2::new(0.0, 1.0),
                        };
                        node.bounds = Rect::from_min_size(node.bounds.min + d, node.bounds.size());
                    }
                    true
                } else { false }
            }
            _ => false,
        }
    }
}

fn color_text(c: Color) -> String { format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b) }

fn parse_color(text: &str) -> Option<Color> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 { return None; }
    let r = u8::from_str_radix(&hex[..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..], 16).ok()?;
    Some(Color::rgb(r, g, b))
}

fn fill(buf: &mut CommandBuffer, r: Rect, color: Color, stroke: Color) {
    buf.push(DrawCommand::Rect { rect: r, fill: color,
        radius: CornerRadius::same(3.0), stroke: Stroke::new(1.0, stroke) });
}

fn label(buf: &mut CommandBuffer, title: impl Into<String>, p: Vec2,
         size: f32, colour: Color, bold: bool) {
    buf.push(DrawCommand::Text { text: title.into(), pos: p, size,
        color: colour, strong: bold });
}

impl KathamoApp for VisualDesigner {
    fn title(&self) -> &str { "Kathamo Visual Designer" }
    fn default_size(&self) -> (u32, u32) { (1600, 960) }

    fn update(&mut self, ctx: &mut AppContext) {
        let win = ctx.bounds();
        let t = ctx.theme;
        let w = win.width();
        let h = win.height();
        let buf = &mut ctx.cmd_buf;

        fill(buf, Rect::from_min_size(Vec2::ZERO, Vec2::new(w, 57.0)), t.surface, t.border);
        label(buf, "KATHAMO", Vec2::new(20.0, 17.0), 18.0, t.text, true);
        label(buf, "Visual Designer", Vec2::new(125.0, 20.0), 12.0, t.text_muted, false);

        for (i, title) in TOP_ACTIONS.iter().enumerate() {
            let r = Self::top_button_rect(i);
            fill(buf, r, if *title == "Preview" && self.preview {
                t.accent
            } else if r.contains(ctx.pointer.position) {
                t.elevated_high
            } else { t.elevated }, t.border);
            label(buf, *title, r.min + Vec2::new(9.0, 9.0), 11.0, t.text, false);
        }

        if !self.preview {
            let left = Rect::from_min_size(Vec2::new(0.0, 59.0),
                Vec2::new(225.0, (h - 94.0).max(200.0)));
            let right = Rect::from_min_size(Vec2::new(w - 295.0, 59.0),
                Vec2::new(295.0, (h - 94.0).max(200.0)));
            fill(buf, left, t.surface, t.border);
            fill(buf, right, t.surface, t.border);

            label(buf, "COMPONENTS", Vec2::new(17.0, 86.0), 13.0, t.text, true);
            label(buf, "Drag into the layout", Vec2::new(17.0, 109.0), 11.0, t.text_muted, false);
            for (i, kind) in DesignKind::ALL.iter().enumerate() {
                let r = Self::palette_row(i);
                fill(buf, r, if self.pending_kind == Some(*kind) {
                    t.elevated_high
                } else if r.contains(ctx.pointer.position) { t.elevated }
                else { t.surface }, t.border);
                label(buf, format!("+   {}", kind.name()), r.min + Vec2::new(10.0, 6.0),
                      11.0, if self.pending_kind == Some(*kind) { t.accent_hover } else { t.text }, false);
            }

            label(buf, "HIERARCHY", Vec2::new(17.0, TREE_Y), 13.0, t.text, true);
            let tree = Rect::from_min_size(Vec2::new(8.0, TREE_Y + 30.0),
                Vec2::new(211.0, (h - TREE_Y - 71.0).max(32.0)));
            fill(buf, tree, t.canvas, t.border);
            buf.push(DrawCommand::ClipPush { rect: tree });
            for (i, node) in self.document.elements.iter().enumerate() {
                let y = TREE_Y + 42.0 + i as f32 * 25.0;
                if y > tree.max.y { break; }
                let r = Rect::from_min_size(Vec2::new(12.0, y - 3.0),
                    Vec2::new(202.0, 23.0));
                if self.selected.as_deref() == Some(node.id.as_str()) {
                    fill(buf, r, t.elevated_high, t.accent);
                }
                label(buf, format!("{}  {}", node.kind.name(), node.label),
                    Vec2::new(20.0, y + 2.0), 10.0, t.text_muted, false);
            }
            buf.push(DrawCommand::ClipPop);

            label(buf, "PROPERTIES", Vec2::new(w - 279.0, 86.0), 13.0, t.text, true);
            label(buf, "Project file", Vec2::new(w - 280.0, 111.0), 10.0, t.text_muted, false);
            let path = Rect::from_min_size(Vec2::new(w - 287.0, 128.0), Vec2::new(272.0, 31.0));
            fill(buf, path, t.elevated, t.border);
            let path_value = if let Some((EditField::Path, text)) = &self.editing {
                text.as_str()
            } else { self.save_path.to_str().unwrap_or("") };
            label(buf, shorten(path_value, 35), path.min + Vec2::new(7.0, 9.0),
                10.0, t.text, false);
            if let Some(id) = &self.selected {
                if let Some(node) = self.document.find(id) {
                    label(buf, format!("{} | {}", node.kind.name(), id),
                        Vec2::new(w - 279.0, 167.0), 10.0, t.accent_hover, false);
                }
                for (i, field) in EditField::ALL.iter().enumerate() {
                    let r = Self::property_row(i, win);
                    label(buf, field.label(), r.min + Vec2::new(5.0, 2.0),
                        10.0, t.text_muted, false);
                    let val = Rect::from_min_size(r.min + Vec2::new(3.0, 15.0),
                        Vec2::new(262.0, 23.0));
                    fill(buf, val, if self.editing.as_ref().is_some_and(|(f, _)| f == field) {
                        t.elevated_high
                    } else { t.elevated }, t.border);
                    let text = match &self.editing {
                        Some((f, value)) if f == field => format!("{}|", value),
                        _ => self.selected_value(*field),
                    };
                    label(buf, shorten(&text, 35), val.min + Vec2::new(7.0, 4.0),
                        11.0, t.text, false);
                }
            } else {
                label(buf, "Select a component to edit", Vec2::new(w - 279.0, 205.0),
                    12.0, t.text_muted, false);
            }
            label(buf, "Click a property and type to edit", Vec2::new(w - 279.0, 782.0),
                10.0, t.text_muted, false);
            label(buf, "Enter: apply  |  Esc: cancel", Vec2::new(w - 279.0, 800.0),
                10.0, t.text_muted, false);
        }

        let g = self.canvas_geometry(win);
        fill(buf, g.frame, t.canvas, t.border);
        buf.push(DrawCommand::ClipPush { rect: g.frame });
        fill(buf, g.page, Color::rgb(18, 22, 30), t.border);
        // Grid is deliberately independent of renderer internals.
        let step = 80.0 * g.scale;
        if step > 9.0 {
            for i in 0..=15 {
                let x = g.origin.x + i as f32 * step;
                if x < g.page.max.x {
                    buf.push(DrawCommand::Line {
                        start: Vec2::new(x, g.page.min.y),
                        end: Vec2::new(x, g.page.max.y),
                        stroke: Stroke::new(1.0, t.border.with_alpha(60)),
                    });
                }
            }
            for i in 0..=10 {
                let y = g.origin.y + i as f32 * step;
                if y < g.page.max.y {
                    buf.push(DrawCommand::Line {
                        start: Vec2::new(g.page.min.x, y),
                        end: Vec2::new(g.page.max.x, y),
                        stroke: Stroke::new(1.0, t.border.with_alpha(60)),
                    });
                }
            }
        }
        if self.preview {
            // Live preview uses the same widgets and input handling as consumer applications.
            self.runtime.render(&self.document, buf, g.page, &ctx.pointer, &t);
        } else {
            self.document.render(buf, g.origin, g.scale, self.selected.as_deref(), &t);
        }
        if let Some(kind) = self.palette_drag {
            if g.page.contains(ctx.pointer.position) {
                let size = kind.default_size() * g.scale;
                fill(buf, Rect::from_min_size(ctx.pointer.position - size * 0.5, size),
                    t.accent.with_alpha(80), t.accent);
            }
        }
        buf.push(DrawCommand::ClipPop);
        if !self.preview {
            label(buf, format!("{}   {} x {}   {:.0}% ",
                self.document.name, self.document.width as i32,
                self.document.height as i32, g.scale * 100.0),
                g.frame.min + Vec2::new(16.0, 12.0), 11.0, t.text_muted, false);
        }

        fill(buf, Rect::from_min_size(Vec2::new(0.0, h - 34.0),
            Vec2::new(w, 34.0)), t.surface, t.border);
        label(buf, shorten(&self.status, 130), Vec2::new(18.0, h - 23.0),
              11.0, t.text_muted, false);
        label(buf, format!("{} components", self.document.elements.len()),
            Vec2::new(w - 160.0, h - 22.0), 10.0, t.text, false);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton,
                         ctx: &mut AppContext) -> bool {
        if self.preview {
            for i in 0..TOP_ACTIONS.len() {
                if Self::top_button_rect(i).contains(pos) {
                    self.toolbar_action(i);
                    return true;
                }
            }
            return self.runtime.pointer_down(&self.document, pos, button, &ctx.pointer);
        }
        if button != PointerButton::Primary { return false; }
        self.handle_primary_down(pos, ctx.bounds())
    }

    fn handle_mouse_move(&mut self, pos: Vec2, delta: Vec2,
                         ctx: &mut AppContext) -> bool {
        if self.preview {
            self.runtime.pointer_move(pos, delta);
            return true;
        }
        if !ctx.pointer.primary_down { return false; }
        self.handle_drag(pos, delta, ctx.bounds())
    }

    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton,
                       ctx: &mut AppContext) -> bool {
        if self.preview {
            let consumed = self.runtime.pointer_up(&self.document, pos, button);
            let events = self.runtime.take_events();
            if let Some(event) = events.last() {
                self.status = format!("UI event: {} ({:?})", event.action_id, event.kind);
            }
            return consumed;
        }
        if button != PointerButton::Primary { return false; }
        self.handle_release(pos, ctx.bounds())
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, ctx: &mut AppContext) -> bool {
        if self.preview && self.runtime.scroll(&self.document, delta, pos) {
            return true;
        }
        if self.canvas_geometry(ctx.bounds()).frame.contains(pos) {
            self.zoom = (self.zoom * if delta.y > 0.0 { 1.08 } else { 0.92 }).clamp(0.35, 3.0);
            true
        } else { false }
    }

    fn handle_key_down(&mut self, key: Key, modifiers: Modifiers,
                       _ctx: &mut AppContext) -> bool {
        if self.preview {
            if key == Key::Escape {
                self.preview = false;
                return true;
            }
            if self.runtime.key_down(&self.document, key.clone(), modifiers) {
                let events = self.runtime.take_events();
                if let Some(event) = events.last() {
                    self.status = format!("UI event: {} ({:?})", event.action_id, event.kind);
                }
                return true;
            }
        }
        self.handle_shortcut(key, modifiers)
    }

    fn handle_ime(&mut self, event: ImeEvent, ctx: &mut AppContext) -> bool {
        if self.preview { return self.runtime.ime(&self.document, event); }
        if let ImeEvent::Commit(text) = event {
            return self.handle_text_input(&text, ctx);
        }
        false
    }

    fn handle_text_input(&mut self, text: &str, _ctx: &mut AppContext) -> bool {
        if self.preview { return self.runtime.text_input(&self.document, text); }
        if let Some((_, value)) = &mut self.editing {
            value.push_str(text);
            true
        } else { false }
    }
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max { return text.to_owned(); }
    let mut out: String = text.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

fn main() {
    let mut designer = VisualDesigner::new();
    let mut headless = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--file" => {
                if let Some(path) = args.next() {
                    designer.save_path = PathBuf::from(path);
                    if designer.save_path.exists() {
                        designer.open();
                    }
                }
            }
            _ => {}
        }
    }
    if headless {
        let mut runner = HeadlessAppRunner::new(designer);
        let count = runner.step_frame();
        assert!(count > 0, "Visual Designer must emit UI draw commands");
        let image = runner.capture_pixels().expect("Designer CPU rendering failed");
        assert_eq!(image.len(), 1600 * 960);
        println!("Kathamo Visual Designer: {} UI commands, {} rendered pixels",
            count, image.len());
        return;
    }
    run_native(designer).expect("Kathamo Visual Designer could not start");
}
