//! Stable UI inspection primitives allowing coding agents and test suites
//! to query element hierarchies, bounds, focus states, and interaction properties.

use crate::math::Rect;

/// A node within the UI inspector tree.
#[derive(Debug, Clone, PartialEq)]
pub struct UiInspectorNode {
    pub id: String,
    pub role: String,
    pub bounds: Rect,
    pub focused: bool,
    pub active: bool,
    pub label: Option<String>,
    pub children: Vec<UiInspectorNode>,
}

impl UiInspectorNode {
    pub fn new(id: impl Into<String>, role: impl Into<String>, bounds: Rect) -> Self {
        Self {
            id: id.into(),
            role: role.into(),
            bounds,
            focused: false,
            active: false,
            label: None,
            children: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_focus(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn add_child(&mut self, child: UiInspectorNode) {
        self.children.push(child);
    }

    /// Recursively finds a node by its ID.
    pub fn find_by_id(&self, target_id: &str) -> Option<&UiInspectorNode> {
        if self.id == target_id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(target_id) {
                return Some(found);
            }
        }
        None
    }

    /// Recursively finds all nodes matching a specific role.
    pub fn find_by_role(&self, target_role: &str) -> Vec<&UiInspectorNode> {
        let mut results = Vec::new();
        self.collect_by_role(target_role, &mut results);
        results
    }

    fn collect_by_role<'a>(&'a self, target_role: &str, out: &mut Vec<&'a UiInspectorNode>) {
        if self.role == target_role {
            out.push(self);
        }
        for child in &self.children {
            child.collect_by_role(target_role, out);
        }
    }
}

/// A complete inspector snapshot of a running workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct UiInspectorSnapshot {
    pub root: UiInspectorNode,
    pub window_size: (u32, u32),
    pub focused_element: Option<String>,
}

impl UiInspectorSnapshot {
    pub fn new(root: UiInspectorNode, window_size: (u32, u32), focused_element: Option<String>) -> Self {
        Self {
            root,
            window_size,
            focused_element,
        }
    }

    pub fn find(&self, id: &str) -> Option<&UiInspectorNode> {
        self.root.find_by_id(id)
    }

    /// Generates a human-readable text dump of the entire UI tree for agent inspection.
    pub fn dump_tree(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("UI Tree Snapshot ({}x{}):\n", self.window_size.0, self.window_size.1));
        Self::dump_node(&self.root, 0, &mut s);
        s
    }

    fn dump_node(node: &UiInspectorNode, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        let label_str = node.label.as_deref().map_or("".to_string(), |l| format!(" \"{}\"", l));
        let focus_str = if node.focused { " [FOCUSED]" } else { "" };
        let active_str = if node.active { " [ACTIVE]" } else { "" };

        out.push_str(&format!(
            "{}- <{}> #{}{} [x:{:.0}, y:{:.0}, w:{:.0}, h:{:.0}]{}{}\n",
            indent,
            node.role,
            node.id,
            label_str,
            node.bounds.min.x,
            node.bounds.min.y,
            node.bounds.width(),
            node.bounds.height(),
            focus_str,
            active_str,
        ));

        for child in &node.children {
            Self::dump_node(child, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec2;

    #[test]
    fn test_inspector_tree_query() {
        let mut root = UiInspectorNode::new("window", "Window", Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0)));
        let mut canvas = UiInspectorNode::new("canvas_host", "CanvasHost", Rect::from_min_size(Vec2::new(200.0, 40.0), Vec2::new(800.0, 680.0)))
            .with_active(true);
        canvas.add_child(UiInspectorNode::new("layer_1", "Layer", Rect::from_min_size(Vec2::new(220.0, 60.0), Vec2::new(100.0, 100.0))).with_label("Hero"));

        root.add_child(canvas);

        let snapshot = UiInspectorSnapshot::new(root, (1280, 720), Some("canvas_host".into()));
        assert!(snapshot.find("canvas_host").is_some());
        assert!(snapshot.find("layer_1").is_some());
        assert_eq!(snapshot.find("layer_1").unwrap().label.as_deref(), Some("Hero"));

        let layers = snapshot.root.find_by_role("Layer");
        assert_eq!(layers.len(), 1);

        let dump = snapshot.dump_tree();
        assert!(dump.contains("<CanvasHost> #canvas_host"));
        assert!(dump.contains("<Layer> #layer_1 \"Hero\""));
    }
}
