//! Accessibility adapter providing semantic UI tree updates compatible with AccessKit.
//!
//! Enables screen readers (Windows Narrator, Orca) to interact with studio controls
//! with zero performance overhead on the visual rendering pipeline.

use std::collections::HashMap;
use ui_core::Rect;

/// Semantic accessibility role matching AccessKit's Role enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    Window,
    Pane,
    Button,
    CheckBox,
    Heading,
    StaticText,
    Slider,
    ComboBox,
    List,
    ListItem,
    Timeline,
}

/// Accessible action that can be triggered by assistive technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityAction {
    Click,
    Focus,
    Toggle,
    SetValue,
}

/// Individual accessible node in the studio visual tree.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    pub id: u64,
    pub role: AccessibilityRole,
    pub name: String,
    pub bounds: Rect,
    pub disabled: bool,
    pub focused: bool,
    pub checked: Option<bool>,
    pub children: Vec<u64>,
}

impl AccessibilityNode {
    pub fn new(id: u64, role: AccessibilityRole, name: impl Into<String>, bounds: Rect) -> Self {
        Self {
            id,
            role,
            name: name.into(),
            bounds,
            disabled: false,
            focused: false,
            checked: None,
            children: Vec::new(),
        }
    }
}

/// Lightweight accessibility tree accumulator for current frame.
#[derive(Debug, Default)]
pub struct AccessibilityTree {
    nodes: HashMap<u64, AccessibilityNode>,
    root_id: u64,
    next_id: u64,
}

impl AccessibilityTree {
    pub fn new(root_name: &str, window_bounds: Rect) -> Self {
        let root_id = 1;
        let mut tree = Self {
            nodes: HashMap::new(),
            root_id,
            next_id: 2,
        };

        let root_node =
            AccessibilityNode::new(root_id, AccessibilityRole::Window, root_name, window_bounds);
        tree.nodes.insert(root_id, root_node);
        tree
    }

    /// Allocates an auto-incrementing accessible ID.
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Registers a child node under the root or specified parent.
    pub fn insert_child(&mut self, parent_id: u64, node: AccessibilityNode) {
        let child_id = node.id;
        self.nodes.insert(child_id, node);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
    }

    pub fn root_id(&self) -> u64 {
        self.root_id
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_node(&self, id: u64) -> Option<&AccessibilityNode> {
        self.nodes.get(&id)
    }

    /// Clears frame nodes while preserving root.
    pub fn clear_frame(&mut self) {
        let root = self.nodes.remove(&self.root_id);
        self.nodes.clear();
        if let Some(mut r) = root {
            r.children.clear();
            self.nodes.insert(self.root_id, r);
        }
        self.next_id = 2;
    }

    /// Converts the frame's semantic accessibility tree into an AccessKit TreeUpdate
    /// for integration with Windows Narrator, Orca (Linux), and VoiceOver.
    pub fn to_accesskit_update(&self) -> accesskit::TreeUpdate {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        let mut focus = accesskit::NodeId(self.root_id);

        for (&id, n) in &self.nodes {
            let role = match n.role {
                AccessibilityRole::Window => accesskit::Role::Window,
                AccessibilityRole::Pane => accesskit::Role::Pane,
                AccessibilityRole::Button => accesskit::Role::Button,
                AccessibilityRole::CheckBox => accesskit::Role::CheckBox,
                AccessibilityRole::Heading => accesskit::Role::Heading,
                AccessibilityRole::StaticText => accesskit::Role::Label,
                AccessibilityRole::Slider => accesskit::Role::Slider,
                AccessibilityRole::ComboBox => accesskit::Role::ComboBox,
                AccessibilityRole::List => accesskit::Role::List,
                AccessibilityRole::ListItem => accesskit::Role::ListItem,
                AccessibilityRole::Timeline => accesskit::Role::Group,
            };

            let mut node = accesskit::NodeBuilder::new(role);
            node.set_name(n.name.clone());
            node.set_bounds(accesskit::Rect {
                x0: n.bounds.min.x as f64,
                y0: n.bounds.min.y as f64,
                x1: n.bounds.max.x as f64,
                y1: n.bounds.max.y as f64,
            });
            for &child_id in &n.children {
                node.push_child(accesskit::NodeId(child_id));
            }
            if n.focused {
                focus = accesskit::NodeId(id);
            }
            nodes.push((accesskit::NodeId(id), node.build()));
        }

        let tree = accesskit::Tree::new(accesskit::NodeId(self.root_id));
        accesskit::TreeUpdate {
            nodes,
            tree: Some(tree),
            focus,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::Vec2;

    #[test]
    fn accessibility_tree_builds_and_traverses_cleanly() {
        let window_bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(1440.0, 900.0));
        let mut tree = AccessibilityTree::new("Kolpona Studio", window_bounds);

        let btn_id = tree.alloc_id();
        let btn_bounds = Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::new(100.0, 32.0));
        let btn_node = AccessibilityNode::new(
            btn_id,
            AccessibilityRole::Button,
            "Generate Image",
            btn_bounds,
        );
        let root_id = tree.root_id();
        tree.insert_child(root_id, btn_node);

        assert_eq!(tree.node_count(), 2);
        let root = tree.get_node(root_id).unwrap();
        assert_eq!(root.children, vec![btn_id]);

        let child = tree.get_node(btn_id).unwrap();
        assert_eq!(child.role, AccessibilityRole::Button);
        assert_eq!(child.name, "Generate Image");

        tree.clear_frame();
        assert_eq!(tree.node_count(), 1);
    }

    #[test]
    fn accesskit_tree_update_converts_correctly() {
        let window_bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(1440.0, 900.0));
        let mut tree = AccessibilityTree::new("Kolpona Studio", window_bounds);
        let btn_id = tree.alloc_id();
        let btn_bounds = Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::new(100.0, 32.0));
        let mut btn_node = AccessibilityNode::new(
            btn_id,
            AccessibilityRole::Button,
            "Generate Image",
            btn_bounds,
        );
        btn_node.focused = true;
        let root_id = tree.root_id();
        tree.insert_child(root_id, btn_node);

        let update = tree.to_accesskit_update();
        assert_eq!(update.nodes.len(), 2);
        assert_eq!(update.focus, accesskit::NodeId(btn_id));
        assert!(update.tree.is_some());
    }
}
