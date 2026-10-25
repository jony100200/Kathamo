//! Recursive binary split DockTree workstation layout engine.
//!
//! Provides a recursive binary split tree supporting horizontal and vertical
//! splits, tabbed panel stacks, interactive splitter resizing, collapse/merge,
//! and deterministic layout geometry calculation.

use crate::math::{Rect, Vec2};

/// Split orientation for container nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal, // Splits left and right (vertical divider bar)
    Vertical,   // Splits top and bottom (horizontal divider bar)
}

/// A panel housed within a tabbed dock leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockTabPanel {
    /// Unique pane instance ID within a workspace.
    pub id: String,
    pub title: String,
    pub is_closable: bool,
    /// Registered editor factory ID. None uses the pane ID for backward compatibility.
    pub factory_id: Option<String>,
}

impl DockTabPanel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            is_closable: true,
            factory_id: None,
        }
    }

    /// Open another independent pane using an already registered editor type.
    pub fn with_factory(mut self, factory_id: impl Into<String>) -> Self {
        self.factory_id = Some(factory_id.into());
        self
    }

    pub fn non_closable(mut self) -> Self {
        self.is_closable = false;
        self
    }
}

/// Node within the binary dock hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub enum DockNode {
    Leaf {
        id: String,
        tabs: Vec<DockTabPanel>,
        active_tab: usize,
    },
    Split {
        id: String,
        axis: SplitAxis,
        ratio: f32, // 0.0 .. 1.0 (clamped between 0.05 and 0.95)
        first: Box<DockNode>,
        second: Box<DockNode>,
    },
}

/// Computed geometry layout for a leaf dock node.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedLeafLayout {
    pub id: String,
    pub bounds: Rect,
    pub header_rect: Rect,
    pub content_rect: Rect,
    pub tabs: Vec<DockTabPanel>,
    pub active_tab: usize,
}

/// Computed geometry for an interactive divider splitter handle.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedSplitterLayout {
    pub split_id: String,
    pub axis: SplitAxis,
    pub handle_rect: Rect,
}

/// Complete evaluated workspace geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkspaceLayout {
    pub leaves: Vec<ComputedLeafLayout>,
    pub splitters: Vec<ComputedSplitterLayout>,
}

/// Drop zone locations for interactive panel docking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockDropZone {
    Left,
    Right,
    Top,
    Bottom,
    Center, // Tabs with the leaf
}

/// High-density workstation dock manager.
#[derive(Debug, Clone, PartialEq)]
pub struct DockTree {
    pub root: DockNode,
    pub splitter_thickness: f32,
    pub tab_header_height: f32,
    pub maximized_leaf: Option<String>,
}

impl Default for DockTree {
    fn default() -> Self {
        Self::new("main", DockTabPanel::new("tab1", "Workspace"))
    }
}

impl DockTree {
    pub fn new(root_leaf_id: impl Into<String>, initial_panel: DockTabPanel) -> Self {
        Self {
            root: DockNode::Leaf {
                id: root_leaf_id.into(),
                tabs: vec![initial_panel],
                active_tab: 0,
            },
            splitter_thickness: 4.0,
            tab_header_height: 26.0,
            maximized_leaf: None,
        }
    }

    pub fn with_root(root: DockNode) -> Self {
        Self {
            root,
            splitter_thickness: 4.0,
            tab_header_height: 26.0,
            maximized_leaf: None,
        }
    }

    /// Splits a target leaf into two children along the specified axis.
    pub fn split_leaf(
        &mut self,
        target_leaf_id: &str,
        new_split_id: impl Into<String>,
        new_leaf_id: impl Into<String>,
        axis: SplitAxis,
        ratio: f32,
        new_panel: DockTabPanel,
    ) -> bool {
        Self::split_node_recursive(
            &mut self.root,
            target_leaf_id,
            new_split_id.into(),
            new_leaf_id.into(),
            axis,
            ratio.clamp(0.05, 0.95),
            new_panel,
        )
    }

    fn split_node_recursive(
        node: &mut DockNode,
        target_id: &str,
        new_split_id: String,
        new_leaf_id: String,
        axis: SplitAxis,
        ratio: f32,
        new_panel: DockTabPanel,
    ) -> bool {
        match node {
            DockNode::Leaf { id, .. } if id == target_id => {
                let existing_leaf = node.clone();
                let new_leaf = DockNode::Leaf {
                    id: new_leaf_id,
                    tabs: vec![new_panel],
                    active_tab: 0,
                };
                *node = DockNode::Split {
                    id: new_split_id,
                    axis,
                    ratio,
                    first: Box::new(existing_leaf),
                    second: Box::new(new_leaf),
                };
                true
            }
            DockNode::Split { first, second, .. } => {
                Self::split_node_recursive(
                    first,
                    target_id,
                    new_split_id.clone(),
                    new_leaf_id.clone(),
                    axis,
                    ratio,
                    new_panel.clone(),
                ) || Self::split_node_recursive(
                    second,
                    target_id,
                    new_split_id,
                    new_leaf_id,
                    axis,
                    ratio,
                    new_panel,
                )
            }
            _ => false,
        }
    }

    /// Adjusts splitter ratio by a delta relative to the container dimension.
    pub fn resize_split(&mut self, split_id: &str, delta_ratio: f32) -> bool {
        Self::resize_split_recursive(&mut self.root, split_id, delta_ratio)
    }

    fn resize_split_recursive(node: &mut DockNode, target_id: &str, delta_ratio: f32) -> bool {
        match node {
            DockNode::Split {
                id,
                ratio,
                first,
                second,
                ..
            } => {
                if id == target_id {
                    *ratio = (*ratio + delta_ratio).clamp(0.05, 0.95);
                    true
                } else {
                    Self::resize_split_recursive(first, target_id, delta_ratio)
                        || Self::resize_split_recursive(second, target_id, delta_ratio)
                }
            }
            DockNode::Leaf { .. } => false,
        }
    }

    /// Closes a panel inside a leaf. If the leaf has no remaining panels,
    /// collapses the parent split to merge into the surviving sibling.
    pub fn close_panel(&mut self, leaf_id: &str, panel_id: &str) -> bool {
        let (removed, should_prune) =
            Self::close_panel_recursive(&mut self.root, leaf_id, panel_id);
        if should_prune {
            Self::prune_empty_leaves(&mut self.root);
        }
        removed
    }

    fn close_panel_recursive(
        node: &mut DockNode,
        target_leaf_id: &str,
        panel_id: &str,
    ) -> (bool, bool) {
        match node {
            DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } if id == target_leaf_id => {
                if let Some(pos) = tabs.iter().position(|p| p.id == panel_id) {
                    if tabs[pos].is_closable {
                        tabs.remove(pos);
                        if *active_tab >= tabs.len() && !tabs.is_empty() {
                            *active_tab = tabs.len() - 1;
                        }
                        (true, tabs.is_empty())
                    } else {
                        (false, false)
                    }
                } else {
                    (false, false)
                }
            }
            DockNode::Split { first, second, .. } => {
                let (rem1, prune1) = Self::close_panel_recursive(first, target_leaf_id, panel_id);
                if rem1 {
                    (true, prune1)
                } else {
                    Self::close_panel_recursive(second, target_leaf_id, panel_id)
                }
            }
            _ => (false, false),
        }
    }

    /// Sets active tab index in target leaf.
    pub fn set_active_tab(&mut self, leaf_id: &str, tab_idx: usize) -> bool {
        Self::set_active_tab_recursive(&mut self.root, leaf_id, tab_idx)
    }

    fn set_active_tab_recursive(node: &mut DockNode, target_id: &str, tab_idx: usize) -> bool {
        match node {
            DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } if id == target_id => {
                if tab_idx < tabs.len() {
                    *active_tab = tab_idx;
                    true
                } else {
                    false
                }
            }
            DockNode::Split { first, second, .. } => {
                Self::set_active_tab_recursive(first, target_id, tab_idx)
                    || Self::set_active_tab_recursive(second, target_id, tab_idx)
            }
            _ => false,
        }
    }

    /// Adds a tab panel into a target leaf.
    pub fn add_tab(&mut self, leaf_id: &str, panel: DockTabPanel) -> bool {
        Self::add_tab_recursive(&mut self.root, leaf_id, panel)
    }

    fn add_tab_recursive(node: &mut DockNode, target_id: &str, panel: DockTabPanel) -> bool {
        match node {
            DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } if id == target_id => {
                tabs.push(panel);
                *active_tab = tabs.len() - 1;
                true
            }
            DockNode::Split { first, second, .. } => {
                Self::add_tab_recursive(first, target_id, panel.clone())
                    || Self::add_tab_recursive(second, target_id, panel)
            }
            _ => false,
        }
    }

    fn prune_empty_leaves(node: &mut DockNode) {
        if let DockNode::Split { first, second, .. } = node {
            Self::prune_empty_leaves(first);
            Self::prune_empty_leaves(second);

            if let DockNode::Leaf { tabs, .. } = &**first {
                if tabs.is_empty() {
                    *node = (**second).clone();
                    return;
                }
            }
            if let DockNode::Leaf { tabs, .. } = &**second {
                if tabs.is_empty() {
                    *node = (**first).clone();
                }
            }
        }
    }

    /// Toggles maximization of a leaf panel.
    pub fn toggle_maximize(&mut self, leaf_id: &str) {
        if self.maximized_leaf.as_deref() == Some(leaf_id) {
            self.maximized_leaf = None;
        } else {
            self.maximized_leaf = Some(leaf_id.to_string());
        }
    }

    /// Checks if a specific leaf is currently maximized.
    pub fn is_maximized(&self, leaf_id: &str) -> bool {
        self.maximized_leaf.as_deref() == Some(leaf_id)
    }

    /// Finds tabs for a given leaf ID.
    pub fn find_leaf_tabs(node: &DockNode, target_id: &str) -> Option<(Vec<DockTabPanel>, usize)> {
        match node {
            DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } if id == target_id => Some((tabs.clone(), *active_tab)),
            DockNode::Split { first, second, .. } => Self::find_leaf_tabs(first, target_id)
                .or_else(|| Self::find_leaf_tabs(second, target_id)),
            _ => None,
        }
    }

    /// Determines which drop zone a point falls into within a leaf's bounds.
    pub fn calculate_drop_zone(
        &self,
        leaf_bounds: Rect,
        pointer_pos: Vec2,
    ) -> Option<DockDropZone> {
        if !leaf_bounds.contains(pointer_pos) {
            return None;
        }
        let w = leaf_bounds.width();
        let h = leaf_bounds.height();
        let rel_x = (pointer_pos.x - leaf_bounds.min.x) / w.max(1.0);
        let rel_y = (pointer_pos.y - leaf_bounds.min.y) / h.max(1.0);

        if rel_x < 0.22 {
            Some(DockDropZone::Left)
        } else if rel_x > 0.78 {
            Some(DockDropZone::Right)
        } else if rel_y < 0.22 {
            Some(DockDropZone::Top)
        } else if rel_y > 0.78 {
            Some(DockDropZone::Bottom)
        } else {
            Some(DockDropZone::Center)
        }
    }

    /// Computes preview rectangle for a drop guide.
    pub fn preview_rect_for_zone(leaf_bounds: Rect, zone: DockDropZone) -> Rect {
        match zone {
            DockDropZone::Left => Rect::from_min_size(
                leaf_bounds.min,
                Vec2::new(leaf_bounds.width() * 0.5, leaf_bounds.height()),
            ),
            DockDropZone::Right => Rect::from_min_size(
                Vec2::new(
                    leaf_bounds.min.x + leaf_bounds.width() * 0.5,
                    leaf_bounds.min.y,
                ),
                Vec2::new(leaf_bounds.width() * 0.5, leaf_bounds.height()),
            ),
            DockDropZone::Top => Rect::from_min_size(
                leaf_bounds.min,
                Vec2::new(leaf_bounds.width(), leaf_bounds.height() * 0.5),
            ),
            DockDropZone::Bottom => Rect::from_min_size(
                Vec2::new(
                    leaf_bounds.min.x,
                    leaf_bounds.min.y + leaf_bounds.height() * 0.5,
                ),
                Vec2::new(leaf_bounds.width(), leaf_bounds.height() * 0.5),
            ),
            DockDropZone::Center => leaf_bounds,
        }
    }

    /// Reorders a tab within a leaf panel.
    pub fn reorder_tab(&mut self, leaf_id: &str, old_idx: usize, new_idx: usize) -> bool {
        Self::reorder_tab_recursive(&mut self.root, leaf_id, old_idx, new_idx)
    }

    fn reorder_tab_recursive(
        node: &mut DockNode,
        target_id: &str,
        old_idx: usize,
        new_idx: usize,
    ) -> bool {
        match node {
            DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } if id == target_id => {
                if old_idx < tabs.len() && new_idx < tabs.len() && old_idx != new_idx {
                    let tab = tabs.remove(old_idx);
                    tabs.insert(new_idx, tab);
                    *active_tab = new_idx;
                    true
                } else {
                    false
                }
            }
            DockNode::Split { first, second, .. } => {
                Self::reorder_tab_recursive(first, target_id, old_idx, new_idx)
                    || Self::reorder_tab_recursive(second, target_id, old_idx, new_idx)
            }
            _ => false,
        }
    }

    /// Recursively computes geometry for all leaves and splitter divider bars.
    pub fn compute_layout(&self, bounds: Rect) -> WorkspaceLayout {
        if let Some(ref max_id) = self.maximized_leaf {
            if let Some((tabs, active_tab)) = Self::find_leaf_tabs(&self.root, max_id) {
                let header_height = self.tab_header_height;
                let header_rect =
                    Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), header_height));
                let content_rect = Rect::from_min_size(
                    Vec2::new(bounds.min.x, bounds.min.y + header_height),
                    Vec2::new(bounds.width(), (bounds.height() - header_height).max(0.0)),
                );
                return WorkspaceLayout {
                    leaves: vec![ComputedLeafLayout {
                        id: max_id.clone(),
                        bounds,
                        header_rect,
                        content_rect,
                        tabs,
                        active_tab,
                    }],
                    splitters: Vec::new(),
                };
            }
        }

        let mut layout = WorkspaceLayout::default();
        self.compute_node_layout(&self.root, bounds, &mut layout);
        layout
    }

    fn compute_node_layout(&self, node: &DockNode, bounds: Rect, layout: &mut WorkspaceLayout) {
        match node {
            DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } => {
                let header_height = if tabs.len() > 1 || !tabs.is_empty() {
                    self.tab_header_height
                } else {
                    0.0
                };
                let header_rect =
                    Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), header_height));
                let content_rect = Rect::from_min_size(
                    Vec2::new(bounds.min.x, bounds.min.y + header_height),
                    Vec2::new(bounds.width(), (bounds.height() - header_height).max(0.0)),
                );
                layout.leaves.push(ComputedLeafLayout {
                    id: id.clone(),
                    bounds,
                    header_rect,
                    content_rect,
                    tabs: tabs.clone(),
                    active_tab: *active_tab,
                });
            }
            DockNode::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                let _half_thick = self.splitter_thickness * 0.5;
                match axis {
                    SplitAxis::Horizontal => {
                        let total_w = bounds.width() - self.splitter_thickness;
                        let first_w = (total_w * ratio).max(10.0);
                        let second_w = (total_w - first_w).max(10.0);

                        let first_bounds =
                            Rect::from_min_size(bounds.min, Vec2::new(first_w, bounds.height()));
                        let splitter_rect = Rect::from_min_size(
                            Vec2::new(bounds.min.x + first_w, bounds.min.y),
                            Vec2::new(self.splitter_thickness, bounds.height()),
                        );
                        let second_bounds = Rect::from_min_size(
                            Vec2::new(
                                bounds.min.x + first_w + self.splitter_thickness,
                                bounds.min.y,
                            ),
                            Vec2::new(second_w, bounds.height()),
                        );

                        layout.splitters.push(ComputedSplitterLayout {
                            split_id: id.clone(),
                            axis: *axis,
                            handle_rect: splitter_rect,
                        });

                        self.compute_node_layout(first, first_bounds, layout);
                        self.compute_node_layout(second, second_bounds, layout);
                    }
                    SplitAxis::Vertical => {
                        let total_h = bounds.height() - self.splitter_thickness;
                        let first_h = (total_h * ratio).max(10.0);
                        let second_h = (total_h - first_h).max(10.0);

                        let first_bounds =
                            Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), first_h));
                        let splitter_rect = Rect::from_min_size(
                            Vec2::new(bounds.min.x, bounds.min.y + first_h),
                            Vec2::new(bounds.width(), self.splitter_thickness),
                        );
                        let second_bounds = Rect::from_min_size(
                            Vec2::new(
                                bounds.min.x,
                                bounds.min.y + first_h + self.splitter_thickness,
                            ),
                            Vec2::new(bounds.width(), second_h),
                        );

                        layout.splitters.push(ComputedSplitterLayout {
                            split_id: id.clone(),
                            axis: *axis,
                            handle_rect: splitter_rect,
                        });

                        self.compute_node_layout(first, first_bounds, layout);
                        self.compute_node_layout(second, second_bounds, layout);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_tree_split_and_layout() {
        let mut dock = DockTree::new("main_leaf", DockTabPanel::new("viewport", "Viewport 3D"));
        assert_eq!(
            dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0)))
                .leaves
                .len(),
            1
        );

        // Split horizontally into Viewport on left and Inspector on right
        let split_ok = dock.split_leaf(
            "main_leaf",
            "split_main",
            "side_leaf",
            SplitAxis::Horizontal,
            0.7,
            DockTabPanel::new("inspector", "Inspector"),
        );
        assert!(split_ok);

        let layout = dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0)));
        assert_eq!(layout.leaves.len(), 2);
        assert_eq!(layout.splitters.len(), 1);

        let left = layout.leaves.iter().find(|l| l.id == "main_leaf").unwrap();
        let right = layout.leaves.iter().find(|l| l.id == "side_leaf").unwrap();

        assert!(left.bounds.width() > right.bounds.width());
        assert_eq!(left.bounds.height(), 800.0);
        assert_eq!(right.bounds.height(), 800.0);
    }

    #[test]
    fn test_dock_tree_resize_split() {
        let mut dock = DockTree::new("leaf_1", DockTabPanel::new("p1", "Panel 1"));
        dock.split_leaf(
            "leaf_1",
            "split_1",
            "leaf_2",
            SplitAxis::Vertical,
            0.5,
            DockTabPanel::new("p2", "Panel 2"),
        );

        assert!(dock.resize_split("split_1", 0.1));

        if let DockNode::Split { ratio, .. } = &dock.root {
            assert!((*ratio - 0.6).abs() < 0.001);
        } else {
            panic!("Root must be a split");
        }
    }

    #[test]
    fn test_dock_tree_close_and_merge() {
        let mut dock = DockTree::new("leaf_1", DockTabPanel::new("p1", "Panel 1"));
        dock.split_leaf(
            "leaf_1",
            "split_1",
            "leaf_2",
            SplitAxis::Horizontal,
            0.5,
            DockTabPanel::new("p2", "Panel 2"),
        );

        // Close Panel 2; leaf_2 should become empty and root should merge back to single leaf_1
        assert!(dock.close_panel("leaf_2", "p2"));

        let layout = dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0)));
        assert_eq!(layout.leaves.len(), 1);
        assert_eq!(layout.leaves[0].id, "leaf_1");
        assert_eq!(layout.splitters.len(), 0);
    }

    #[test]
    fn test_dock_tree_maximize_and_restore() {
        let mut dock = DockTree::new("leaf_1", DockTabPanel::new("p1", "Panel 1"));
        dock.split_leaf(
            "leaf_1",
            "split_1",
            "leaf_2",
            SplitAxis::Horizontal,
            0.5,
            DockTabPanel::new("p2", "Panel 2"),
        );

        assert_eq!(
            dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0)))
                .leaves
                .len(),
            2
        );

        // Maximize leaf_2
        dock.toggle_maximize("leaf_2");
        assert!(dock.is_maximized("leaf_2"));
        let max_layout =
            dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0)));
        assert_eq!(max_layout.leaves.len(), 1);
        assert_eq!(max_layout.leaves[0].id, "leaf_2");
        assert_eq!(max_layout.leaves[0].bounds.width(), 800.0);
        assert_eq!(max_layout.splitters.len(), 0);

        // Restore
        dock.toggle_maximize("leaf_2");
        assert!(!dock.is_maximized("leaf_2"));
        assert_eq!(
            dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0)))
                .leaves
                .len(),
            2
        );
    }

    #[test]
    fn test_dock_tree_drop_zones() {
        let dock = DockTree::new("leaf_1", DockTabPanel::new("p1", "Panel 1"));
        let bounds = Rect::from_min_size(Vec2::new(100.0, 100.0), Vec2::new(200.0, 200.0));

        assert_eq!(
            dock.calculate_drop_zone(bounds, Vec2::new(110.0, 200.0)),
            Some(DockDropZone::Left)
        );
        assert_eq!(
            dock.calculate_drop_zone(bounds, Vec2::new(280.0, 200.0)),
            Some(DockDropZone::Right)
        );
        assert_eq!(
            dock.calculate_drop_zone(bounds, Vec2::new(200.0, 110.0)),
            Some(DockDropZone::Top)
        );
        assert_eq!(
            dock.calculate_drop_zone(bounds, Vec2::new(200.0, 280.0)),
            Some(DockDropZone::Bottom)
        );
        assert_eq!(
            dock.calculate_drop_zone(bounds, Vec2::new(200.0, 200.0)),
            Some(DockDropZone::Center)
        );
        assert_eq!(
            dock.calculate_drop_zone(bounds, Vec2::new(50.0, 50.0)),
            None
        );
    }
}
