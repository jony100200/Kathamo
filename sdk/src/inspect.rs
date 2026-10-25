//! Agent Inspection APIs for Kathamo SDK.

use ui_core::{Rect, Vec2, UiInspectorNode, UiInspectorSnapshot};
use crate::workspace::WorkspaceHost;

/// Helper for constructing inspectable snapshots of Kathamo interfaces.
pub struct InterfaceInspector;

impl InterfaceInspector {
    /// Create a new snapshot with root element.
    pub fn new_snapshot(root_id: &str, viewport: Rect) -> UiInspectorSnapshot {
        let root = UiInspectorNode::new(root_id, "WindowRoot", viewport);
        let win_size = (viewport.width() as u32, viewport.height() as u32);
        UiInspectorSnapshot::new(root, win_size, None)
    }

    /// Snapshot the actual active dock layout. Nodes use stable panel IDs suitable
    /// for deterministic agent interaction through WorkspaceHost::activate_panel.
    pub fn capture_workspace(host: &WorkspaceHost, bounds: Rect) -> UiInspectorSnapshot {
        let ws = host.active_workspace();
        let layout = ws.dock_tree.compute_layout(bounds);
        let mut snap = Self::new_snapshot(&ws.id, bounds);
        snap.focused_element = host.focused_panel_id.clone();
        for leaf in &layout.leaves {
            let mut group = UiInspectorNode::new(&leaf.id, "DockLeaf", leaf.bounds);
            for (index, tab) in leaf.tabs.iter().enumerate() {
                let tab_bounds = Rect::from_min_size(
                    Vec2::new(leaf.header_rect.min.x + 8.0 + index as f32 * 114.0,
                        leaf.header_rect.min.y + 2.0),
                    Vec2::new(110.0, leaf.header_rect.height() - 2.0),
                );
                group.add_child(
                    UiInspectorNode::new(&tab.id, "DockTab", tab_bounds)
                        .with_label(&tab.title)
                        .with_active(index == leaf.active_tab)
                        .with_focus(host.focused_panel_id.as_deref() == Some(&tab.id)),
                );
            }
            if let Some(active) = leaf.tabs.get(leaf.active_tab) {
                group.add_child(
                    UiInspectorNode::new(format!("editor:{}", active.id), "EditorViewport", leaf.content_rect)
                        .with_label(&active.title)
                        .with_active(true)
                        .with_focus(host.focused_panel_id.as_deref() == Some(&active.id)),
                );
            }
            snap.root.add_child(group);
        }
        snap
    }

    /// Return the deepest element under the pointer, ordered by painted children.
    pub fn hit_test<'a>(snapshot: &'a UiInspectorSnapshot, pos: Vec2) -> Option<&'a UiInspectorNode> {
        fn descend(node: &UiInspectorNode, pos: Vec2) -> Option<&UiInspectorNode> {
            if !node.bounds.contains(pos) { return None; }
            for child in node.children.iter().rev() {
                if let Some(hit) = descend(child, pos) { return Some(hit); }
            }
            Some(node)
        }
        descend(&snapshot.root, pos)
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
    fn live_workspace_snapshot_matches_docking_layout() {
        let host = WorkspaceHost::new();
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(1200.0, 800.0));
        let snap = InterfaceInspector::capture_workspace(&host, rect);
        assert!(snap.find("canvas_2d").is_some());
        let editor = snap.find("editor:canvas_2d").unwrap();
        assert_eq!(editor.role, "EditorViewport");
        assert_eq!(InterfaceInspector::hit_test(&snap, editor.bounds.center()).unwrap().id,
            "editor:canvas_2d");
    }

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
