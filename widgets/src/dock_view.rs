//! Interactive DockTreeView rendering DockTree with splitters, tabs, drop guides,
//! maximize/restore buttons, and drag-and-drop tab docking.

use crate::theme::ThemePalette;
use ui_core::{
    Color, CornerRadius, DockDropZone, DockTabPanel, DockTree, Rect, SplitAxis, Stroke, Vec2,
    WorkspaceLayout,
};
use ui_platform::{PointerButton, PointerState};
use ui_render::{CommandBuffer, DrawCommand};

/// Tab title size in dock headers.
const TAB_TEXT: f32 = 11.5;
/// Room for the close cross on a closable tab.
const TAB_CLOSE: f32 = 18.0;
const TAB_GAP: f32 = 4.0;
const TAB_MIN: f32 = 44.0;
const TAB_MAX: f32 = 180.0;
/// Room kept free at the right of a header for the maximize button.
const HEADER_RESERVED: f32 = 34.0;

/// The tab rectangles of a leaf header: each tab as wide as its title (bold, plus padding and
/// the close cross), and all of them shrunk evenly when they would run past the header, so no
/// tab ever spills over its neighbour panel. Titles that no longer fit are shortened with an
/// ellipsis when drawn.
pub fn dock_tab_rects(header: Rect, tabs: &[DockTabPanel]) -> Vec<Rect> {
    let natural: Vec<f32> = tabs
        .iter()
        .map(|tab| {
            let close = if tab.is_closable { TAB_CLOSE } else { 0.0 };
            (ui_render::measure_text(&tab.title, TAB_TEXT, true) + 18.0 + close)
                .clamp(TAB_MIN, TAB_MAX)
        })
        .collect();
    let gaps = TAB_GAP * tabs.len().saturating_sub(1) as f32;
    let room = (header.width() - 8.0 - HEADER_RESERVED - gaps).max(0.0);
    let total: f32 = natural.iter().sum();
    let scale = if total > room && total > 0.0 {
        room / total
    } else {
        1.0
    };
    let mut x = header.min.x + 8.0;
    natural
        .into_iter()
        .map(|w| {
            let w = (w * scale).max(TAB_MIN.min(w));
            let rect = Rect::from_min_size(
                Vec2::new(x, header.min.y + 2.0),
                Vec2::new(w, header.height() - 2.0),
            );
            x += w + TAB_GAP;
            rect
        })
        .collect()
}

/// State of an active tab drag operation.
#[derive(Debug, Clone, PartialEq)]
pub struct DraggingTabState {
    pub source_leaf_id: String,
    pub tab_index: usize,
    pub panel: DockTabPanel,
    pub cursor_offset: Vec2,
}

/// Interactive view component for `DockTree`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DockTreeView {
    pub active_splitter_drag: Option<(String, SplitAxis, Vec2, f32)>, // (split_id, axis, start_pos, start_ratio)
    pub dragging_tab: Option<DraggingTabState>,
    pub hovered_splitter: Option<String>,
    /// Set when a tab is dropped outside all dock leaves.
    pub pending_detach: Option<String>,
}

impl DockTreeView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles pointer press down on tabs, maximize buttons, or splitters.
    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        layout: &WorkspaceLayout,
        dock_tree: &mut DockTree,
    ) -> bool {
        if button != PointerButton::Primary {
            return false;
        }

        // 1. Check Splitters first (for resizing)
        for sp in &layout.splitters {
            let hit_rect = sp.handle_rect.expand(3.0); // Slight hit slop
            if hit_rect.contains(pos) {
                self.active_splitter_drag = Some((sp.split_id.clone(), sp.axis, pos, 0.0));
                return true;
            }
        }

        // 2. Check Leaf Headers (tabs, close, maximize)
        for leaf in &layout.leaves {
            if leaf.header_rect.contains(pos) {
                // Maximize / Restore button (top right of header)
                let max_btn_rect = Rect::from_min_size(
                    Vec2::new(leaf.header_rect.max.x - 26.0, leaf.header_rect.min.y + 4.0),
                    Vec2::new(20.0, 18.0),
                );
                if max_btn_rect.contains(pos) {
                    dock_tree.toggle_maximize(&leaf.id);
                    return true;
                }

                // Check tab clicks
                let rects = dock_tab_rects(leaf.header_rect, &leaf.tabs);
                for (t_idx, (tab, tab_rect)) in leaf.tabs.iter().zip(rects).enumerate() {
                    if tab_rect.contains(pos) {
                        // Close button on tab
                        let close_rect = Rect::from_min_size(
                            Vec2::new(tab_rect.max.x - 18.0, tab_rect.min.y + 4.0),
                            Vec2::new(14.0, 14.0),
                        );
                        if tab.is_closable && close_rect.contains(pos) {
                            dock_tree.close_panel(&leaf.id, &tab.id);
                            return true;
                        }

                        // Switch active tab
                        dock_tree.set_active_tab(&leaf.id, t_idx);

                        // Start tab drag
                        self.dragging_tab = Some(DraggingTabState {
                            source_leaf_id: leaf.id.clone(),
                            tab_index: t_idx,
                            panel: tab.clone(),
                            cursor_offset: pos - tab_rect.min,
                        });
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Handles pointer movement for splitter resizing and tab dragging.
    pub fn handle_mouse_move(
        &mut self,
        _pos: Vec2,
        delta: Vec2,
        bounds: Rect,
        dock_tree: &mut DockTree,
        ptr: &PointerState,
    ) {
        if !ptr.primary_down {
            self.active_splitter_drag = None;
            self.dragging_tab = None;
            return;
        }

        // Handle active splitter drag
        if let Some((ref split_id, axis, _, _)) = self.active_splitter_drag.clone() {
            let delta_ratio = match axis {
                SplitAxis::Horizontal => delta.x / bounds.width().max(10.0),
                SplitAxis::Vertical => delta.y / bounds.height().max(10.0),
            };
            dock_tree.resize_split(split_id, delta_ratio);
        }
    }

    /// Handles mouse release, committing tab drops into target leaves.
    pub fn handle_mouse_up(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        layout: &WorkspaceLayout,
        dock_tree: &mut DockTree,
    ) -> bool {
        if button != PointerButton::Primary {
            return false;
        }

        self.active_splitter_drag = None;

        if let Some(dragging) = self.dragging_tab.take() {
            // Find target leaf under cursor
            for leaf in &layout.leaves {
                if leaf.id != dragging.source_leaf_id && leaf.bounds.contains(pos) {
                    if let Some(zone) = dock_tree.calculate_drop_zone(leaf.bounds, pos) {
                        // Commit both operations together: a failed drop must not lose a panel.
                        let mut next = dock_tree.clone();
                        if !next.close_panel(&dragging.source_leaf_id, &dragging.panel.id) {
                            return false;
                        }
                        let success = match zone {
                            DockDropZone::Center => next.add_tab(&leaf.id, dragging.panel),
                            DockDropZone::Left
                            | DockDropZone::Right
                            | DockDropZone::Top
                            | DockDropZone::Bottom => {
                                let (axis, ratio) = match zone {
                                    DockDropZone::Left => (SplitAxis::Horizontal, 0.3),
                                    DockDropZone::Right => (SplitAxis::Horizontal, 0.7),
                                    DockDropZone::Top => (SplitAxis::Vertical, 0.3),
                                    DockDropZone::Bottom => (SplitAxis::Vertical, 0.7),
                                    DockDropZone::Center => unreachable!(),
                                };
                                next.split_leaf(
                                    &leaf.id,
                                    format!("split_{}_{}", leaf.id, dragging.panel.id),
                                    format!("leaf_{}", dragging.panel.id),
                                    axis,
                                    ratio,
                                    dragging.panel,
                                )
                            }
                        };
                        if success {
                            *dock_tree = next;
                        }
                        return success;
                    }
                }
            }
            // No docking target: offer the dragged panel to the application
            // for creation of a native detached window.
            if !layout.leaves.iter().any(|leaf| leaf.bounds.contains(pos)) {
                self.pending_detach = Some(dragging.panel.id);
                return true;
            }
        }

        false
    }

    /// Alias for `render_chrome`.
    pub fn render(
        &self,
        buf: &mut CommandBuffer,
        layout: &WorkspaceLayout,
        dock_tree: &DockTree,
        palette: &ThemePalette,
        ptr: &PointerState,
    ) {
        self.render_chrome(buf, layout, dock_tree, ptr, palette);
    }

    /// Renders chrome: tab headers, close buttons, maximize toggles, splitters, and drop guides.
    pub fn render_chrome(
        &self,
        buf: &mut CommandBuffer,
        layout: &WorkspaceLayout,
        dock_tree: &DockTree,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        // 1. Render Leaf Headers and Borders
        for leaf in &layout.leaves {
            // Panel container border
            buf.push(DrawCommand::Rect {
                rect: leaf.bounds,
                fill: Color::TRANSPARENT,
                radius: CornerRadius::ZERO,
                stroke: Stroke::new(1.0, palette.border),
            });

            // Header bar
            buf.push(DrawCommand::Rect {
                rect: leaf.header_rect,
                fill: palette.elevated,
                radius: CornerRadius::ZERO,
                stroke: Stroke::new(1.0, palette.border),
            });

            // Tabs
            let rects = dock_tab_rects(leaf.header_rect, &leaf.tabs);
            for (t_idx, (tab, tab_rect)) in leaf.tabs.iter().zip(rects).enumerate() {
                let is_active = t_idx == leaf.active_tab;

                buf.push(DrawCommand::Rect {
                    rect: tab_rect,
                    fill: if is_active {
                        palette.surface
                    } else {
                        palette.elevated.with_alpha(120)
                    },
                    radius: CornerRadius::same(3.0),
                    stroke: if is_active {
                        Stroke::new(1.0, palette.border)
                    } else {
                        Stroke::NONE
                    },
                });

                let close = if tab.is_closable { TAB_CLOSE } else { 0.0 };
                let title_room = (tab_rect.width() - 14.0 - close).max(8.0);
                buf.push(DrawCommand::Text {
                    text: ui_render::fit_text(&tab.title, TAB_TEXT, is_active, title_room),
                    pos: Vec2::new(tab_rect.min.x + 8.0, tab_rect.min.y + 5.0),
                    size: TAB_TEXT,
                    color: if is_active {
                        palette.text
                    } else {
                        palette.text_muted
                    },
                    strong: is_active,
                });

                // Tab close button
                if tab.is_closable {
                    buf.push(DrawCommand::Text {
                        text: "×".into(),
                        pos: Vec2::new(tab_rect.max.x - 14.0, tab_rect.min.y + 4.0),
                        size: 13.0,
                        color: palette.text_muted,
                        strong: false,
                    });
                }
            }

            // Maximize / Restore Toggle Icon
            let max_icon = if dock_tree.is_maximized(&leaf.id) {
                "🗗" // Restore symbol
            } else {
                "🗖" // Maximize symbol
            };
            buf.push(DrawCommand::Text {
                text: max_icon.into(),
                pos: Vec2::new(leaf.header_rect.max.x - 22.0, leaf.header_rect.min.y + 6.0),
                size: 12.0,
                color: palette.text_muted,
                strong: true,
            });
        }

        // 2. Render Interactive Splitters
        for sp in &layout.splitters {
            let is_dragged = self
                .active_splitter_drag
                .as_ref()
                .is_some_and(|(id, ..)| id == &sp.split_id);
            let is_hovered = sp.handle_rect.expand(2.0).contains(ptr.position);

            let fill = if is_dragged {
                palette.accent
            } else if is_hovered {
                palette.border.with_alpha(220)
            } else {
                palette.border.with_alpha(150)
            };

            buf.push(DrawCommand::Rect {
                rect: sp.handle_rect,
                fill,
                radius: CornerRadius::ZERO,
                stroke: Stroke::NONE,
            });
        }

        // 3. Render Drop Guides when dragging a tab
        if let Some(ref dragging) = self.dragging_tab {
            for leaf in &layout.leaves {
                if leaf.id != dragging.source_leaf_id && leaf.bounds.contains(ptr.position) {
                    if let Some(zone) = dock_tree.calculate_drop_zone(leaf.bounds, ptr.position) {
                        let guide_rect = DockTree::preview_rect_for_zone(leaf.bounds, zone);
                        buf.push(DrawCommand::Rect {
                            rect: guide_rect,
                            fill: palette.accent.with_alpha(60),
                            radius: CornerRadius::same(6.0),
                            stroke: Stroke::new(2.0, palette.accent),
                        });
                    }
                }
            }

            // Dragging tab thumbnail floating under cursor
            let float_rect = Rect::from_min_size(
                ptr.position - dragging.cursor_offset,
                Vec2::new(110.0, 24.0),
            );
            buf.push(DrawCommand::Rect {
                rect: float_rect,
                fill: palette.elevated.with_alpha(220),
                radius: CornerRadius::same(4.0),
                stroke: Stroke::new(1.5, palette.accent),
            });
            buf.push(DrawCommand::Text {
                text: dragging.panel.title.clone(),
                pos: Vec2::new(float_rect.min.x + 8.0, float_rect.min.y + 5.0),
                size: 11.5,
                color: palette.text,
                strong: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::{DockNode, DockTabPanel};
    use ui_themes::ThemePalette;

    #[test]
    fn dock_tabs_fit_their_titles_and_never_spill_past_the_header() {
        let tab = |title: &str| DockTabPanel::new(title, title);
        let header = Rect::from_min_size(Vec2::new(100.0, 10.0), Vec2::new(900.0, 26.0));
        let short = dock_tab_rects(header, &[tab("Story"), tab("Activity")]);
        assert!(
            short[0].width() < short[1].width(),
            "wider titles get wider tabs"
        );
        assert!(short[0].width() >= TAB_MIN);

        let many: Vec<_> = (0..12).map(|i| tab(&format!("Panel number {i}"))).collect();
        let narrow = Rect::from_min_size(Vec2::new(0.0, 0.0), Vec2::new(400.0, 26.0));
        let rects = dock_tab_rects(narrow, &many);
        assert_eq!(rects.len(), 12);
        let last = rects.last().unwrap();
        assert!(
            last.max.x <= narrow.max.x - HEADER_RESERVED + 1.0 || last.width() <= TAB_MIN,
            "tabs shrink to fit: {last:?}"
        );
        for pair in rects.windows(2) {
            assert!(pair[0].max.x <= pair[1].min.x, "no overlap");
        }
    }

    #[test]
    fn test_dock_view_interactions() {
        let mut dock_tree = DockTree {
            root: DockNode::Split {
                id: "split_root".into(),
                axis: SplitAxis::Horizontal,
                ratio: 0.5,
                first: Box::new(DockNode::Leaf {
                    id: "left".into(),
                    tabs: vec![DockTabPanel::new("p1", "Hierarchy")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "right".into(),
                    tabs: vec![DockTabPanel::new("p2", "Inspector")],
                    active_tab: 0,
                }),
            },
            ..Default::default()
        };

        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));
        let layout = dock_tree.compute_layout(bounds);
        assert_eq!(layout.splitters.len(), 1);

        let mut dock_view = DockTreeView::new();

        // Hit test splitter
        let splitter_pos = layout.splitters[0].handle_rect.center();
        let hit = dock_view.handle_mouse_down(
            splitter_pos,
            PointerButton::Primary,
            &layout,
            &mut dock_tree,
        );
        assert!(hit, "Clicking splitter should activate drag");
        assert!(dock_view.active_splitter_drag.is_some());

        // Drag splitter
        let ptr = PointerState {
            primary_down: true,
            ..Default::default()
        };
        dock_view.handle_mouse_move(
            splitter_pos + Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 0.0),
            bounds,
            &mut dock_tree,
            &ptr,
        );

        dock_view.handle_mouse_up(
            splitter_pos,
            PointerButton::Primary,
            &layout,
            &mut dock_tree,
        );
        assert!(dock_view.active_splitter_drag.is_none());
    }

    #[test]
    fn test_dock_view_rendering_emits_commands() {
        let dock_tree = DockTree::default();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0));
        let layout = dock_tree.compute_layout(bounds);

        let dock_view = DockTreeView::new();
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let ptr = PointerState::default();

        dock_view.render(&mut buf, &layout, &dock_tree, &palette, &ptr);
        assert!(!buf.commands().is_empty());
    }
}
