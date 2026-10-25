//! Data-only graph document for creative node UIs. This validates graph structure, not execution.
//! Application runtimes own capability dispatch; Kathamo owns editing and serialization.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::{GraphNode, NodeConnection, NodeEditor, NodeGroup, NodePin};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphDocument {
    pub nodes: Vec<GraphNode>,
    pub connections: Vec<NodeConnection>,
    pub groups: Vec<NodeGroup>,
}

impl GraphDocument {
    pub fn empty() -> Self { Self::default() }

    pub fn from_editor(editor: &NodeEditor) -> Self {
        Self {
            nodes: editor.nodes.clone(),
            connections: editor.connections.clone(),
            groups: editor.groups.clone(),
        }
    }

    pub fn apply_to(&self, editor: &mut NodeEditor) -> Result<(), String> {
        self.validate()?;
        editor.nodes = self.nodes.clone();
        editor.connections = self.connections.clone();
        editor.groups = self.groups.clone();
        editor.selected_node = None;
        editor.pending_wire = None;
        editor.dragging_node = None;
        Ok(())
    }

    /// Validate references, port direction/types, unique input wiring and acyclic structure.
    pub fn validate(&self) -> Result<(), String> {
        if self.nodes.len() > 50000 || self.connections.len() > 200000 {
            return Err("graph exceeds safety limits".into());
        }
        let mut ids = HashSet::new();
        for node in &self.nodes {
            if node.id.is_empty() || !ids.insert(node.id.as_str()) {
                return Err("graph has duplicate or empty node IDs".into());
            }
            if !node.pos.x.is_finite() || !node.pos.y.is_finite() ||
               !node.size.x.is_finite() || !node.size.y.is_finite() ||
               node.size.x <= 0.0 || node.size.y <= 0.0 {
                return Err(format!("invalid geometry for node {}", node.id));
            }
            let mut ports = HashSet::new();
            for pin in node.inputs.iter().chain(node.outputs.iter()) {
                if pin.id.is_empty() || !ports.insert(pin.id.as_str()) {
                    return Err(format!("duplicate pin ID in {}", node.id));
                }
            }
            if node.inputs.iter().any(|p| p.is_output) ||
                node.outputs.iter().any(|p| !p.is_output) {
                return Err(format!("pin direction mismatch in {}", node.id));
            }
        }
        let by_id: HashMap<&str, &GraphNode> = self.nodes.iter()
            .map(|n| (n.id.as_str(), n)).collect();
        let mut sinks: HashSet<(&str, &str)> = HashSet::new();
        for c in &self.connections {
            let from = by_id.get(c.from_node.as_str())
                .ok_or_else(|| format!("unknown output node {}", c.from_node))?;
            let to = by_id.get(c.to_node.as_str())
                .ok_or_else(|| format!("unknown input node {}", c.to_node))?;
            if c.from_node == c.to_node { return Err("self-cycle".into()); }
            let output = from.outputs.iter().find(|p| p.id == c.from_pin)
                .ok_or("unknown output socket")?;
            let input = to.inputs.iter().find(|p| p.id == c.to_pin)
                .ok_or("unknown input socket")?;
            if output.pin_type != input.pin_type &&
               output.pin_type != "Any" && input.pin_type != "Any" {
                return Err(format!("incompatible pins: {} -> {}",
                    output.pin_type, input.pin_type));
            }
            if !sinks.insert((c.to_node.as_str(), c.to_pin.as_str())) {
                return Err("input socket has multiple connections".into());
            }
        }
        let mut group_ids = HashSet::new();
        for group in &self.groups {
            if group.id.is_empty() || !group_ids.insert(&group.id) {
                return Err("duplicate group ID".into());
            }
        }
        if self.sorted_nodes().len() != self.nodes.len() {
            return Err("graph contains a directed cycle".into());
        }
        Ok(())
    }

    fn sorted_nodes(&self) -> Vec<String> {
        let mut indegree: HashMap<&str, usize> = self.nodes.iter()
            .map(|n| (n.id.as_str(), 0)).collect();
        let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
        for c in &self.connections {
            if let Some(d) = indegree.get_mut(c.to_node.as_str()) { *d += 1; }
            outgoing.entry(c.from_node.as_str()).or_default().push(&c.to_node);
        }
        let mut ready: VecDeque<&str> = self.nodes.iter()
            .filter(|n| indegree.get(n.id.as_str()) == Some(&0))
            .map(|n| n.id.as_str()).collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = ready.pop_front() {
            order.push(id.to_owned());
            if let Some(next) = outgoing.get(id) {
                for target in next {
                    if let Some(d) = indegree.get_mut(*target) {
                        *d -= 1;
                        if *d == 0 { ready.push_back(target); }
                    }
                }
            }
        }
        order
    }

    /// Both compact guided steps and the advanced graph use the identical node IDs.
    pub fn guided_steps(&self) -> Result<Vec<(String, String)>, String> {
        self.validate()?;
        Ok(self.sorted_nodes().iter().filter_map(|id|
            self.nodes.iter().find(|n| &n.id == id)
                .map(|n| (n.id.clone(), n.title.clone()))).collect())
    }

    /// Persist graph geometry, typed ports, links, and groups, never executable code.
    pub fn serialize(&self) -> Result<String, String> {
        self.validate()?;
        let mut out = String::from("KATHAMO_GRAPH_1\n");
        for n in &self.nodes {
            let color = n.preview_color.map(rgba).unwrap_or_else(|| "-".into());
            out.push_str(&format!("N\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                hex(&n.id), hex(&n.title), hex(&n.category), n.pos.x, n.pos.y,
                n.size.x, n.size.y, if n.is_collapsed { 1 } else { 0 }, color));
            for p in n.inputs.iter().chain(n.outputs.iter()) {
                out.push_str(&format!("P\t{}\t{}\t{}\t{}\t{}\n",
                    hex(&n.id), hex(&p.id), hex(&p.name),
                    if p.is_output { 1 } else { 0 }, hex(&p.pin_type)));
            }
        }
        for c in &self.connections {
            out.push_str(&format!("E\t{}\t{}\t{}\t{}\n",
                hex(&c.from_node), hex(&c.from_pin),
                hex(&c.to_node), hex(&c.to_pin)));
        }
        for g in &self.groups {
            out.push_str(&format!("G\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                hex(&g.id), hex(&g.title), g.bounds.min.x,
                g.bounds.min.y, g.bounds.width(), g.bounds.height(), rgba(g.color)));
        }
        Ok(out)
    }

    pub fn deserialize(source: &str) -> Result<Self, String> {
        if source.len() > 32 * 1024 * 1024 { return Err("graph file too large".into()); }
        let mut lines = source.lines();
        if lines.next() != Some("KATHAMO_GRAPH_1") {
            return Err("unsupported graph format".into());
        }
        let mut doc = Self::empty();
        for line in lines {
            let p: Vec<_> = line.split('\t').collect();
            match p.first().copied() {
                Some("N") if p.len() == 10 => {
                    let node = GraphNode {
                        id: unhex(p[1])?, title: unhex(p[2])?,
                        category: unhex(p[3])?,
                        pos: Vec2::new(finite(p[4])?, finite(p[5])?),
                        size: Vec2::new(finite(p[6])?, finite(p[7])?),
                        is_collapsed: bit(p[8])?,
                        preview_color: if p[9] == "-" { None }
                            else { Some(parse_color(p[9])?) },
                        inputs: Vec::new(), outputs: Vec::new(),
                    };
                    doc.nodes.push(node);
                }
                Some("P") if p.len() == 6 => {
                    let id = unhex(p[1])?;
                    let pin = NodePin {
                        id: unhex(p[2])?, name: unhex(p[3])?,
                        is_output: bit(p[4])?, pin_type: unhex(p[5])?,
                    };
                    let node = doc.nodes.iter_mut().find(|n| n.id == id)
                        .ok_or("pin must follow its node")?;
                    if pin.is_output { node.outputs.push(pin); }
                    else { node.inputs.push(pin); }
                }
                Some("E") if p.len() == 5 => {
                    doc.connections.push(NodeConnection {
                        from_node: unhex(p[1])?, from_pin: unhex(p[2])?,
                        to_node: unhex(p[3])?, to_pin: unhex(p[4])?,
                    });
                }
                Some("G") if p.len() == 8 => {
                    doc.groups.push(NodeGroup {
                        id: unhex(p[1])?, title: unhex(p[2])?,
                        bounds: Rect::from_min_size(
                            Vec2::new(finite(p[3])?, finite(p[4])?),
                            Vec2::new(finite(p[5])?, finite(p[6])?)),
                        color: parse_color(p[7])?,
                    });
                }
                _ => return Err("invalid graph record".into()),
            }
            if doc.nodes.len() > 50000 || doc.connections.len() > 200000 {
                return Err("graph exceeds safety limits".into());
            }
        }
        doc.validate()?;
        Ok(doc)
    }

    /// An approachable step/card presentation of the exact same GraphDocument.
    /// No second copy of the graph, no workflow execution embedded in the framework.
    pub fn render_guided(&self, buf: &mut CommandBuffer, bounds: Rect,
                         scroll_y: f32, selected_id: Option<&str>,
                         pal: &ThemePalette) -> Result<(), String> {
        let steps = self.guided_steps()?;
        let row_h = 94.0;
        let x = bounds.min.x + 20.0;
        let w = (bounds.width() - 40.0).max(120.0);
        buf.push(DrawCommand::ClipPush { rect: bounds });
        for (i, (id, title)) in steps.iter().enumerate() {
            let y = bounds.min.y + 18.0 + i as f32 * row_h - scroll_y;
            let card = Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, 76.0));
            if !card.intersects(bounds) { continue; }
            let selected = selected_id == Some(id.as_str());
            let node = self.nodes.iter().find(|n| &n.id == id);
            buf.push(DrawCommand::Rect {
                rect: card,
                fill: pal.surface,
                radius: CornerRadius::same(8.0),
                stroke: Stroke::new(if selected { 2.0 } else { 1.0 },
                    if selected { pal.accent } else { pal.border }),
            });
            buf.push(DrawCommand::Text {
                text: format!("{}. {}", i + 1, title),
                pos: card.min + Vec2::new(15.0, 13.0), size: 14.0,
                color: pal.text, strong: true,
            });
            if let Some(n) = node {
                buf.push(DrawCommand::Text {
                    text: format!("{}  |  {} inputs  |  {} outputs",
                        n.category, n.inputs.len(), n.outputs.len()),
                    pos: card.min + Vec2::new(15.0, 43.0), size: 11.0,
                    color: pal.text_muted, strong: false,
                });
            }
        }
        buf.push(DrawCommand::ClipPop);
        Ok(())
    }

    /// Select a guided card by point, returning the same stable ID as the graph canvas.
    pub fn guided_hit_test(&self, bounds: Rect, pos: Vec2,
                           scroll_y: f32) -> Result<Option<String>, String> {
        if !bounds.contains(pos) { return Ok(None); }
        let i = ((pos.y - bounds.min.y + scroll_y - 18.0) / 94.0).floor() as isize;
        if i < 0 { return Ok(None); }
        let x = bounds.min.x + 20.0;
        let w = (bounds.width() - 40.0).max(120.0);
        let card = Rect::from_min_size(
            Vec2::new(x, bounds.min.y + 18.0 + i as f32 * 94.0 - scroll_y),
            Vec2::new(w, 76.0));
        if !card.contains(pos) { return Ok(None); }
        Ok(self.guided_steps()?.get(i as usize).map(|(id, _)| id.clone()))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        std::fs::write(path, self.serialize()?).map_err(|e| e.to_string())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::deserialize(&text)
    }
}

fn hex(v: &str) -> String {
    let mut out = String::with_capacity(v.len() * 2);
    for byte in v.bytes() { out.push_str(&format!("{byte:02x}")); }
    out
}
fn unhex(v: &str) -> Result<String, String> {
    if v.len() % 2 != 0 { return Err("invalid encoded string".into()); }
    let mut bytes = Vec::with_capacity(v.len() / 2);
    for pair in v.as_bytes().chunks(2) {
        let h = std::str::from_utf8(pair).map_err(|e| e.to_string())?;
        bytes.push(u8::from_str_radix(h, 16).map_err(|e| e.to_string())?);
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
fn finite(v: &str) -> Result<f32, String> {
    let n: f32 = v.parse::<f32>().map_err(|e| e.to_string())?;
    if !n.is_finite() || n.abs() > 1000000.0 { return Err("invalid graph number".into()); }
    Ok(n)
}
fn bit(v: &str) -> Result<bool, String> {
    match v { "0" => Ok(false), "1" => Ok(true), _ => Err("invalid bool".into()) }
}
fn rgba(c: Color) -> String { format!("{:02x}{:02x}{:02x}{:02x}", c.r,c.g,c.b,c.a) }
fn parse_color(v: &str) -> Result<Color, String> {
    if v.len() != 8 { return Err("invalid graph color".into()); }
    let part = |n: usize| u8::from_str_radix(&v[n..n+2], 16).map_err(|e| e.to_string());
    Ok(Color::rgba(part(0)?, part(2)?, part(4)?, part(6)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_round_trip() {
        let original = GraphDocument::from_editor(&NodeEditor::default());
        let text = original.serialize().unwrap();
        let copy = GraphDocument::deserialize(&text).unwrap();
        assert_eq!(original, copy);
        assert_eq!(copy.guided_steps().unwrap().len(), 5);
        let mut editor = NodeEditor::default();
        copy.apply_to(&mut editor).unwrap();
    }

    #[test]
    fn reject_bad_types_and_cycles() {
        let mut doc = GraphDocument::from_editor(&NodeEditor::default());
        let edge = doc.connections[0].clone();
        doc.connections.push(NodeConnection {
            from_node: edge.to_node,
            from_pin: "out_rgb".into(), to_node: edge.from_node,
            to_pin: edge.from_pin,
        });
        assert!(doc.validate().is_err());
    }
}
