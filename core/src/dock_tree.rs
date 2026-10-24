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
    pub id: String,
    pub title: String,
    pub is_closable: bool,
}

impl DockTabPanel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            is_closable: true,
        }
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

/// High-density workstation dock manager.
#[derive(Debug, Clone, PartialEq)]
pub struct DockTree {
    pub root: DockNode,
    pub splitter_thickness: f32,
    pub tab_header_height: f32,
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
        }
    }

    pub fn with_root(root: DockNode) -> Self {
        Self {
            root,
            splitter_thickness: 4.0,
            tab_header_height: 26.0,
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
                Self::split_node_recursive(first, target_id, new_split_id.clone(), new_leaf_id.clone(), axis, ratio, new_panel.clone())
                    || Self::split_node_recursive(second, target_id, new_split_id, new_leaf_id, axis, ratio, new_panel)
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
            DockNode::Split { id, ratio, first, second, .. } => {
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
        let (removed, should_prune) = Self::close_panel_recursive(&mut self.root, leaf_id, panel_id);
        if should_prune {
            Self::prune_empty_leaves(&mut self.root);
        }
        removed
    }

    fn close_panel_recursive(node: &mut DockNode, target_leaf_id: &str, panel_id: &str) -> (bool, bool) {
        match node {
            DockNode::Leaf { id, tabs, active_tab } if id == target_leaf_id => {
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

    /// Recursively computes geometry for all leaves and splitter divider bars.
    pub fn compute_layout(&self, bounds: Rect) -> WorkspaceLayout {
        let mut layout = WorkspaceLayout::default();
        self.compute_node_layout(&self.root, bounds, &mut layout);
        layout
    }

    fn compute_node_layout(&self, node: &DockNode, bounds: Rect, layout: &mut WorkspaceLayout) {
        match node {
            DockNode::Leaf { id, tabs, active_tab } => {
                let header_height = if tabs.len() > 1 || !tabs.is_empty() {
                    self.tab_header_height
                } else {
                    0.0
                };
                let header_rect = Rect::from_min_size(
                    bounds.min,
                    Vec2::new(bounds.width(), header_height),
                );
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
            DockNode::Split { id, axis, ratio, first, second } => {
                let _half_thick = self.splitter_thickness * 0.5;
                match axis {
                    SplitAxis::Horizontal => {
                        let total_w = bounds.width() - self.splitter_thickness;
                        let first_w = (total_w * ratio).max(10.0);
                        let second_w = (total_w - first_w).max(10.0);

                        let first_bounds = Rect::from_min_size(
                            bounds.min,
                            Vec2::new(first_w, bounds.height()),
                        );
                        let splitter_rect = Rect::from_min_size(
                            Vec2::new(bounds.min.x + first_w, bounds.min.y),
                            Vec2::new(self.splitter_thickness, bounds.height()),
                        );
                        let second_bounds = Rect::from_min_size(
                            Vec2::new(bounds.min.x + first_w + self.splitter_thickness, bounds.min.y),
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

                        let first_bounds = Rect::from_min_size(
                            bounds.min,
                            Vec2::new(bounds.width(), first_h),
                        );
                        let splitter_rect = Rect::from_min_size(
                            Vec2::new(bounds.min.x, bounds.min.y + first_h),
                            Vec2::new(bounds.width(), self.splitter_thickness),
                        );
                        let second_bounds = Rect::from_min_size(
                            Vec2::new(bounds.min.x, bounds.min.y + first_h + self.splitter_thickness),
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
        assert_eq!(dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0))).leaves.len(), 1);

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
        dock.split_leaf("leaf_1", "split_1", "leaf_2", SplitAxis::Vertical, 0.5, DockTabPanel::new("p2", "Panel 2"));

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
        dock.split_leaf("leaf_1", "split_1", "leaf_2", SplitAxis::Horizontal, 0.5, DockTabPanel::new("p2", "Panel 2"));

        // Close Panel 2; leaf_2 should become empty and root should merge back to single leaf_1
        assert!(dock.close_panel("leaf_2", "p2"));

        let layout = dock.compute_layout(Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0)));
        assert_eq!(layout.leaves.len(), 1);
        assert_eq!(layout.leaves[0].id, "leaf_1");
        assert_eq!(layout.splitters.len(), 0);
    }
}
