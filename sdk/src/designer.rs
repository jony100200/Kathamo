//! Serializable, agent-editable UI document shared by Kathamo Designer and consuming apps.
//! The document defines interface components and layout, never application functionality.

use std::collections::HashSet;
use std::path::Path;

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignKind {
    Panel,
    Viewport,
    Hierarchy,
    Inspector,
    AssetBrowser,
    Console,
    Timeline,
    NodeGraph,
    Toolbar,
    Button,
    Label,
    TextInput,
    Toggle,
}

impl DesignKind {
    pub const ALL: [Self; 13] = [
        Self::Panel, Self::Viewport, Self::Hierarchy, Self::Inspector,
        Self::AssetBrowser, Self::Console, Self::Timeline, Self::NodeGraph,
        Self::Toolbar, Self::Button, Self::Label, Self::TextInput, Self::Toggle,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Panel => "Panel",
            Self::Viewport => "Viewport",
            Self::Hierarchy => "Hierarchy",
            Self::Inspector => "Inspector",
            Self::AssetBrowser => "AssetBrowser",
            Self::Console => "Console",
            Self::Timeline => "Timeline",
            Self::NodeGraph => "NodeGraph",
            Self::Toolbar => "Toolbar",
            Self::Button => "Button",
            Self::Label => "Label",
            Self::TextInput => "TextInput",
            Self::Toggle => "Toggle",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }

    pub fn default_size(self) -> Vec2 {
        match self {
            Self::Panel | Self::Hierarchy | Self::Inspector |
            Self::AssetBrowser | Self::Console => Vec2::new(230.0, 260.0),
            Self::Viewport => Vec2::new(480.0, 330.0),
            Self::Timeline | Self::NodeGraph => Vec2::new(450.0, 180.0),
            Self::Toolbar => Vec2::new(450.0, 36.0),
            Self::Button => Vec2::new(140.0, 34.0),
            Self::Label => Vec2::new(170.0, 28.0),
            Self::TextInput => Vec2::new(200.0, 32.0),
            Self::Toggle => Vec2::new(160.0, 30.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignTemplate { Empty, Unity, Blender, Krita }

impl DesignTemplate {
    pub const ALL: [Self; 4] = [Self::Empty, Self::Unity, Self::Blender, Self::Krita];
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "Empty", Self::Unity => "Unity",
            Self::Blender => "Blender", Self::Krita => "Krita",
        }
    }
}

/// One component instance; coordinates are relative to its parent, or to the document if root.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignElement {
    pub id: String,
    pub kind: DesignKind,
    pub parent: Option<String>,
    pub label: String,
    pub bounds: Rect,
    pub background: Color,
    pub foreground: Color,
    pub visible: bool,
}

impl DesignElement {
    pub fn new(id: impl Into<String>, kind: DesignKind, label: impl Into<String>, bounds: Rect) -> Self {
        Self {
            id: id.into(), kind, parent: None, label: label.into(), bounds,
            background: Color::rgb(32, 39, 52),
            foreground: Color::rgb(226, 234, 246),
            visible: true,
        }
    }
}

/// Persistent component tree. Agents can edit its public nodes by stable ID.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignDocument {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub elements: Vec<DesignElement>,
    next_id: u64,
}

impl DesignDocument {
    pub fn new(name: impl Into<String>, width: f32, height: f32) -> Self {
        Self { name: name.into(), width: width.max(320.0), height: height.max(240.0),
            elements: Vec::new(), next_id: 1 }
    }

    pub fn template(preset: DesignTemplate) -> Self {
        let mut doc = Self::new(format!("{} Editor UI", preset.name()), 1200.0, 760.0);
        if preset == DesignTemplate::Empty { return doc; }
        doc.place(DesignKind::Toolbar, "Main Toolbar", 0.0, 0.0, 1200.0, 40.0);
        match preset {
            DesignTemplate::Unity | DesignTemplate::Blender => {
                doc.place(DesignKind::Hierarchy, "Hierarchy", 0.0, 40.0, 230.0, 490.0);
                doc.place(DesignKind::Viewport, "Scene View", 234.0, 40.0, 720.0, 490.0);
                doc.place(DesignKind::Inspector, "Inspector", 958.0, 40.0, 242.0, 490.0);
                doc.place(DesignKind::AssetBrowser, "Project Assets", 0.0, 534.0, 680.0, 226.0);
                doc.place(DesignKind::Console, "Console", 684.0, 534.0, 516.0, 226.0);
            }
            DesignTemplate::Krita => {
                doc.place(DesignKind::Toolbar, "Paint Tools", 0.0, 44.0, 70.0, 716.0);
                doc.place(DesignKind::Viewport, "Painting Canvas", 74.0, 44.0, 850.0, 716.0);
                doc.place(DesignKind::Inspector, "Brush Properties", 928.0, 44.0, 272.0, 352.0);
                doc.place(DesignKind::Hierarchy, "Layers", 928.0, 400.0, 272.0, 360.0);
            }
            DesignTemplate::Empty => {}
        }
        doc
    }

    fn place(&mut self, kind: DesignKind, title: &str, x: f32, y: f32, w: f32, h: f32) {
        self.add(kind, title, Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, h)));
    }

    /// Create a child component with parent-relative coordinates.
    pub fn add_child(&mut self, parent: &str, kind: DesignKind,
                     label: impl Into<String>, bounds: Rect) -> Result<String, String> {
        if self.find(parent).is_none() { return Err("unknown parent".into()); }
        let id = self.add(kind, label, bounds);
        self.find_mut(&id).unwrap().parent = Some(parent.to_owned());
        Ok(id)
    }

    pub fn add(&mut self, kind: DesignKind, label: impl Into<String>, bounds: Rect) -> String {
        let id = loop {
            let id = format!("component_{}", self.next_id);
            self.next_id += 1;
            if self.find(&id).is_none() { break id; }
        };
        self.elements.push(DesignElement::new(id.clone(), kind, label, bounds));
        id
    }

    pub fn find(&self, id: &str) -> Option<&DesignElement> {
        self.elements.iter().find(|node| node.id == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut DesignElement> {
        self.elements.iter_mut().find(|node| node.id == id)
    }

    /// Reparent a component, preserving parent-relative coordinates.
    pub fn set_parent(&mut self, id: &str, parent: Option<&str>) -> Result<(), String> {
        if self.find(id).is_none() { return Err("unknown component".into()); }
        let mut next = parent;
        for _ in 0..=self.elements.len() {
            match next {
                Some(p) if p == id => return Err("component hierarchy cycle".into()),
                Some(p) => next = Some(self.find(p).ok_or("unknown parent")?).and_then(|n| n.parent.as_deref()),
                None => {
                    self.find_mut(id).unwrap().parent = parent.map(str::to_string);
                    return Ok(());
                }
            }
        }
        Err("component hierarchy too deep".into())
    }

    /// Bounds in absolute design-document coordinates.
    pub fn absolute_bounds(&self, id: &str) -> Option<Rect> {
        let node = self.find(id)?;
        let size = node.bounds.size();
        let mut pos = node.bounds.min;
        let mut parent = node.parent.as_deref();
        for _ in 0..self.elements.len() {
            match parent {
                Some(p) => {
                    let node = self.find(p)?;
                    pos = pos + node.bounds.min;
                    parent = node.parent.as_deref();
                }
                None => return Some(Rect::from_min_size(pos, size)),
            }
        }
        None
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if self.find(id).is_none() { return false; }
        let mut removed = HashSet::new();
        removed.insert(id.to_owned());
        loop {
            let before = removed.len();
            for node in &self.elements {
                if node.parent.as_ref().is_some_and(|p| removed.contains(p)) {
                    removed.insert(node.id.clone());
                }
            }
            if removed.len() == before { break; }
        }
        self.elements.retain(|node| !removed.contains(&node.id));
        true
    }

    /// Topmost visible component under a design-space point.
    pub fn hit_test(&self, point: Vec2) -> Option<&DesignElement> {
        self.elements.iter().rev().find(|node| {
            node.visible && self.absolute_bounds(&node.id).is_some_and(|r| r.contains(point))
        })
    }

    /// Draw using Kathamo commands; the Designer and exported app share this implementation.
    pub fn render(&self, buf: &mut CommandBuffer, origin: Vec2, scale: f32,
                  selected: Option<&str>, palette: &ThemePalette) {
        let zoom = scale.max(0.05);
        for node in &self.elements {
            if !node.visible { continue; }
            let Some(abs) = self.absolute_bounds(&node.id) else { continue };
            let rect = Rect::from_min_size(origin + abs.min * zoom, abs.size() * zoom);
            draw_component(buf, rect, zoom, node, selected == Some(&node.id), palette);
        }
    }

    /// Human-readable, versioned document. No extra serialization crate required.
    pub fn serialize(&self) -> String {
        let mut out = format!("KATHAMO_DESIGN_1\nD\t{}\t{}\t{}\t{}\n",
            hex(&self.name), self.width, self.height, self.next_id);
        for n in &self.elements {
            let parent = n.parent.as_deref().map(hex).unwrap_or_else(|| "-".into());
            out.push_str(&format!("N\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                hex(&n.id), parent, n.kind.name(), hex(&n.label),
                n.bounds.min.x, n.bounds.min.y, n.bounds.width(), n.bounds.height(),
                color_hex(n.background), color_hex(n.foreground), if n.visible { 1 } else { 0 }));
        }
        out
    }

    pub fn deserialize(source: &str) -> Result<Self, String> {
        if source.len() > 4 * 1024 * 1024 { return Err("document too large".into()); }
        let mut lines = source.lines();
        if lines.next() != Some("KATHAMO_DESIGN_1") {
            return Err("unsupported design file version".into());
        }
        let header = fields(lines.next().ok_or("missing document header")?, 5)?;
        if header[0] != "D" { return Err("invalid document header".into()); }
        let mut doc = Self::new(unhex(header[1])?, positive(header[2])?, positive(header[3])?);
        doc.next_id = header[4].parse::<u64>().map_err(|e| e.to_string())?.max(1);
        let mut known = HashSet::new();
        for line in lines {
            if doc.elements.len() >= 4096 { return Err("too many components".into()); }
            let f = fields(line, 12)?;
            if f[0] != "N" { return Err("invalid component record".into()); }
            let id = unhex(f[1])?;
            if id.is_empty() || !known.insert(id.clone()) { return Err("duplicate component ID".into()); }
            let parent = if f[2] == "-" { None } else { Some(unhex(f[2])?) };
            let kind = DesignKind::parse(f[3]).ok_or("unknown component kind")?;
            let (x, y, w, h) = (finite(f[5])?, finite(f[6])?, positive(f[7])?, positive(f[8])?);
            let mut node = DesignElement::new(id, kind, unhex(f[4])?,
                Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, h)));
            node.parent = parent;
            node.background = parse_color(f[9])?;
            node.foreground = parse_color(f[10])?;
            node.visible = match f[11] { "0" => false, "1" => true,
                _ => return Err("invalid visible flag".into()) };
            doc.elements.push(node);
        }
        // Parent order may vary when an agent reparents existing components.
        for node in &doc.elements {
            if node.parent.as_ref().is_some_and(|p| !known.contains(p)) {
                return Err("unknown component parent".into());
            }
            if doc.absolute_bounds(&node.id).is_none() {
                return Err("component hierarchy cycle".into());
            }
        }
        Ok(doc)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        std::fs::write(path, self.serialize()).map_err(|e| e.to_string())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::deserialize(&source)
    }

    /// Export an editable Rust function constructing the same runtime UI document.
    pub fn export_rust(&self) -> String {
        let mut out = format!(
            "use kathamo::prelude::*;\n\n// Render with: build_ui().render(&mut ctx.cmd_buf, ctx.bounds().min, 1.0, None, &ctx.theme);\npub fn build_ui() -> DesignDocument {{\n    let mut ui = DesignDocument::new({:?}, {}f32, {}f32);\n",
            self.name, self.width, self.height);
        for n in &self.elements {
            out.push_str(&format!(
                "    ui.elements.push(DesignElement {{ id: {:?}.into(), kind: DesignKind::{}, parent: {}, label: {:?}.into(), bounds: Rect::from_min_size(Vec2::new({}f32, {}f32), Vec2::new({}f32, {}f32)), background: Color::rgba({}, {}, {}, {}), foreground: Color::rgba({}, {}, {}, {}), visible: {} }});\n",
                n.id, n.kind.name(),
                n.parent.as_deref().map(|p| format!("{:?}.map(str::to_owned)", Some(p)))
                    .unwrap_or_else(|| "None".into()),
                n.label, n.bounds.min.x, n.bounds.min.y, n.bounds.width(), n.bounds.height(),
                n.background.r, n.background.g, n.background.b, n.background.a,
                n.foreground.r, n.foreground.g, n.foreground.b, n.foreground.a, n.visible));
        }
        out.push_str("    ui\n}\n");
        out
    }

    pub fn export_to(&self, path: impl AsRef<Path>) -> Result<(), String> {
        std::fs::write(path, self.export_rust()).map_err(|e| e.to_string())
    }
}

fn fields(line: &str, count: usize) -> Result<Vec<&str>, String> {
    let parts: Vec<_> = line.split('\t').collect();
    if parts.len() != count { return Err("invalid design record".into()); }
    Ok(parts)
}

fn finite(raw: &str) -> Result<f32, String> {
    let n: f32 = raw.parse::<f32>().map_err(|e| e.to_string())?;
    if n.is_finite() && n.abs() <= 100_000.0 { Ok(n) } else { Err("invalid coordinate".into()) }
}

fn positive(raw: &str) -> Result<f32, String> {
    let n = finite(raw)?;
    if n > 0.0 { Ok(n) } else { Err("invalid dimension".into()) }
}

fn hex(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() * 2);
    for b in raw.bytes() { out.push_str(&format!("{b:02x}")); }
    out
}

fn unhex(raw: &str) -> Result<String, String> {
    if raw.len() % 2 != 0 { return Err("invalid encoded string".into()); }
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    for pair in raw.as_bytes().chunks(2) {
        let s = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
        bytes.push(u8::from_str_radix(s, 16).map_err(|e| e.to_string())?);
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn color_hex(c: Color) -> String {
    format!("{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a)
}

fn parse_color(raw: &str) -> Result<Color, String> {
    if raw.len() != 8 { return Err("invalid colour".into()); }
    let p = |i: usize| u8::from_str_radix(&raw[i..i + 2], 16).map_err(|e| e.to_string());
    Ok(Color::rgba(p(0)?, p(2)?, p(4)?, p(6)?))
}

fn rect_cmd(buf: &mut CommandBuffer, rect: Rect, fill: Color, border: Color, radius: f32) {
    buf.push(DrawCommand::Rect {
        rect, fill, radius: CornerRadius::same(radius),
        stroke: Stroke::new(1.0, border),
    });
}

fn text_cmd(buf: &mut CommandBuffer, text: &str, pos: Vec2, size: f32,
            color: Color, strong: bool) {
    buf.push(DrawCommand::Text { text: text.into(), pos, size, color, strong });
}

fn draw_component(buf: &mut CommandBuffer, r: Rect, zoom: f32, node: &DesignElement,
                  selected: bool, palette: &ThemePalette) {
    let outline = if selected { palette.accent } else { palette.border };
    let size = (12.0 * zoom).clamp(8.0, 26.0);
    let header_h = (29.0 * zoom).min(r.height());
    let panel = !matches!(node.kind, DesignKind::Label | DesignKind::Button |
                          DesignKind::TextInput | DesignKind::Toggle);
    let fill = node.background;
    rect_cmd(buf, r, if panel { fill } else { palette.surface }, outline, 3.0);
    buf.push(DrawCommand::ClipPush { rect: r });
    match node.kind {
        DesignKind::Label => {
            text_cmd(buf, &node.label, r.min + Vec2::new(8.0, 6.0) * zoom, size,
                     node.foreground, false);
        }
        DesignKind::Button => {
            text_cmd(buf, &node.label, r.min + Vec2::new(10.0, 8.0) * zoom, size,
                     node.foreground, true);
        }
        DesignKind::TextInput => {
            text_cmd(buf, &node.label, r.min + Vec2::new(8.0, 8.0) * zoom, size,
                     palette.text_muted, false);
        }
        DesignKind::Toggle => {
            text_cmd(buf, &node.label, r.min + Vec2::new(8.0, 7.0) * zoom, size,
                     node.foreground, false);
            let knob = Rect::from_min_size(r.max - Vec2::new(36.0, 24.0) * zoom,
                Vec2::new(26.0, 15.0) * zoom);
            rect_cmd(buf, knob, palette.accent, palette.border, 6.0);
        }
        DesignKind::Toolbar => {
            text_cmd(buf, &node.label, r.min + Vec2::new(12.0, 9.0) * zoom, size,
                     node.foreground, true);
            let mut x = r.min.x + 125.0 * zoom;
            for title in ["Move", "Rotate", "Scale", "Play"] {
                if x + 55.0 * zoom < r.max.x {
                    rect_cmd(buf, Rect::from_min_size(Vec2::new(x, r.min.y + 5.0 * zoom),
                        Vec2::new(55.0, 25.0) * zoom), palette.elevated,
                        palette.border, 3.0);
                    text_cmd(buf, title, Vec2::new(x + 5.0 * zoom, r.min.y + 9.0 * zoom),
                        size * 0.8, node.foreground, false);
                }
                x += 60.0 * zoom;
            }
        }
        _ => {
            rect_cmd(buf, Rect::from_min_size(r.min, Vec2::new(r.width(), header_h)),
                     palette.elevated, outline, 1.0);
            text_cmd(buf, &node.label, r.min + Vec2::new(10.0, 7.0) * zoom,
                     size, node.foreground, true);
            let inner = Rect::from_min_max(Vec2::new(r.min.x + 7.0 * zoom,
                    r.min.y + header_h + 8.0 * zoom),
                r.max - Vec2::splat(7.0 * zoom));
            if inner.width() > 0.0 && inner.height() > 0.0 {
                match node.kind {
                    DesignKind::Viewport => {
                        rect_cmd(buf, inner, palette.canvas, palette.border, 2.0);
                        text_cmd(buf, "Perspective  |  Shaded",
                            inner.min + Vec2::new(12.0, 14.0) * zoom,
                            size * 0.85, palette.text_muted, false);
                        let center = inner.center();
                        buf.push(DrawCommand::Line {
                            start: Vec2::new(center.x, inner.min.y + 25.0 * zoom),
                            end: Vec2::new(center.x, inner.max.y),
                            stroke: Stroke::new(1.0, palette.border),
                        });
                        buf.push(DrawCommand::Line {
                            start: Vec2::new(inner.min.x, center.y),
                            end: Vec2::new(inner.max.x, center.y),
                            stroke: Stroke::new(1.0, palette.border),
                        });
                        rect_cmd(buf, Rect::from_min_size(center - Vec2::splat(28.0 * zoom),
                            Vec2::splat(56.0 * zoom)), palette.elevated_high, palette.accent, 2.0);
                    }
                    DesignKind::Hierarchy | DesignKind::Inspector | DesignKind::Console => {
                        let rows: &[&str] = match node.kind {
                            DesignKind::Hierarchy => &["Scene", "  Main Camera", "  Directional Light",
                                "  Player", "  Environment"],
                            DesignKind::Inspector => &["Transform", "Position   0, 0, 0",
                                "Rotation   0, 0, 0", "Scale      1, 1, 1", "Material"],
                            _ => &["[info]  Editor initialized", "[info]  Scene loaded",
                                "[debug] Preview mode"],
                        };
                        for (i, title) in rows.iter().enumerate() {
                            let y = inner.min.y + i as f32 * 25.0 * zoom;
                            if y + 14.0 * zoom < inner.max.y {
                                text_cmd(buf, title, Vec2::new(inner.min.x + 6.0 * zoom, y),
                                    size * 0.88, palette.text_muted, false);
                            }
                        }
                    }
                    DesignKind::AssetBrowser => {
                        let width = 63.0 * zoom;
                        for i in 0..6 {
                            let col = i % 3;
                            let row = i / 3;
                            let p = inner.min + Vec2::new(
                                (10.0 + col as f32 * 76.0) * zoom,
                                (12.0 + row as f32 * 82.0) * zoom);
                            if p.x + width < inner.max.x && p.y + width < inner.max.y {
                                rect_cmd(buf, Rect::from_min_size(p, Vec2::splat(width)),
                                         palette.elevated, palette.border, 4.0);
                                text_cmd(buf, "Asset", p + Vec2::new(8.0, 22.0) * zoom,
                                    size * 0.8, palette.text_muted, false);
                            }
                        }
                    }
                    DesignKind::Timeline => {
                        for i in 0..4 {
                            let p = inner.min + Vec2::new(8.0, (10.0 + i as f32 * 31.0) * zoom);
                            if p.y + 20.0 * zoom < inner.max.y {
                                rect_cmd(buf, Rect::from_min_size(p,
                                    Vec2::new(inner.width() * 0.64, 20.0 * zoom)),
                                    if i % 2 == 0 { palette.accent } else { palette.elevated },
                                    palette.border, 2.0);
                            }
                        }
                    }
                    DesignKind::NodeGraph => {
                        for (i, title) in ["Input", "Process", "Output"].iter().enumerate() {
                            let x = inner.min.x + (15.0 + i as f32 * 118.0) * zoom;
                            let rr = Rect::from_min_size(Vec2::new(x, inner.min.y + 40.0 * zoom),
                                Vec2::new(100.0, 72.0) * zoom);
                            if rr.max.x < inner.max.x {
                                rect_cmd(buf, rr, palette.elevated, palette.accent, 4.0);
                                text_cmd(buf, title, rr.min + Vec2::new(10.0, 9.0) * zoom,
                                    size * 0.9, node.foreground, true);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    buf.push(DrawCommand::ClipPop);
    if selected {
        // Bottom-right resizing handle, also used by the visual Designer.
        rect_cmd(buf, Rect::from_min_size(r.max - Vec2::splat(10.0),
            Vec2::splat(10.0)), palette.accent, palette.accent, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_round_trip_and_edit() {
        let mut doc = DesignDocument::template(DesignTemplate::Unity);
        let id = doc.add(DesignKind::Button, "Compile", Rect::from_min_size(
            Vec2::new(20.0, 50.0), Vec2::new(140.0, 34.0)));
        let copy = DesignDocument::deserialize(&doc.serialize()).unwrap();
        assert_eq!(copy.elements, doc.elements);
        assert_eq!(copy.hit_test(Vec2::new(30.0, 60.0)).unwrap().id, id);
        assert!(copy.export_rust().contains("build_ui"));
    }

    #[test]
    fn hierarchy_validation() {
        let mut doc = DesignDocument::new("Test", 1200.0, 800.0);
        let a = doc.add(DesignKind::Panel, "Parent", Rect::from_min_size(Vec2::ZERO, Vec2::splat(200.0)));
        let b = doc.add(DesignKind::Button, "Child", Rect::from_min_size(Vec2::splat(5.0), Vec2::splat(30.0)));
        doc.set_parent(&b, Some(&a)).unwrap();
        assert!(doc.set_parent(&a, Some(&b)).is_err());
        assert_eq!(doc.absolute_bounds(&b).unwrap().min, Vec2::splat(5.0));
        assert!(doc.remove(&a));
        assert!(doc.elements.is_empty());
    }
}
