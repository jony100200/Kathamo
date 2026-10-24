//! Agent Inspection APIs for Kathamo SDK.

use ui_core::{Rect, UiInspectorNode, UiInspectorSnapshot};

/// Helper for constructing inspectable snapshots of Kathamo interfaces.
pub struct InterfaceInspector;

impl InterfaceInspector {
    /// Create a new snapshot with root element.
    pub fn new_snapshot(root_id: &str, viewport: Rect) -> UiInspectorSnapshot {
        let root = UiInspectorNode::new(root_id, "WindowRoot", viewport);
        let win_size = (viewport.width() as u32, viewport.height() as u32);
        UiInspectorSnapshot::new(root, win_size, None)
    }

    /// Add a button child to a root node.
    pub fn add_button(parent: &mut UiInspectorNode, id: &str, label: &str, bounds: Rect, enabled: bool) {
        let node = UiInspectorNode::new(id, "Button", bounds)
            .with_label(label)
            .with_active(enabled);
        parent.add_child(node);
    }

    /// Add an editor viewport child.
    pub fn add_editor(parent: &mut UiInspectorNode, id: &str, title: &str, bounds: Rect) {
        let node = UiInspectorNode::new(id, "EditorViewport", bounds).with_label(title);
        parent.add_child(node);
    }

    /// Add a dock tab child.
    pub fn add_dock_tab(parent: &mut UiInspectorNode, id: &str, title: &str, bounds: Rect, active: bool) {
        let node = UiInspectorNode::new(id, "DockTab", bounds)
            .with_label(title)
            .with_active(active);
        parent.add_child(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::Vec2;

    #[test]
    fn test_interface_inspector_queries() {
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1440.0, 900.0));
        let mut snap = InterfaceInspector::new_snapshot("main_win", viewport);

        InterfaceInspector::add_dock_tab(&mut snap.root, "tab_canvas", "2D Canvas", Rect::from_min_size(Vec2::ZERO, Vec2::new(120.0, 28.0)), true);
        InterfaceInspector::add_editor(&mut snap.root, "main_canvas", "2D Canvas Host", Rect::from_min_size(Vec2::new(0.0, 28.0), Vec2::new(1000.0, 872.0)));
        InterfaceInspector::add_button(&mut snap.root, "btn_brush", "Brush Tool", Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::new(32.0, 32.0)), true);

        assert!(snap.find("tab_canvas").is_some());
        assert_eq!(snap.root.find_by_role("Button").len(), 1);
        assert_eq!(snap.root.find_by_role("EditorViewport").len(), 1);
    }
}
