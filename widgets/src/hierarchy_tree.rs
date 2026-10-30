//! Reusable HierarchyTree for layers, scene graphs, objects, and tracks.
//! Supports nesting, visibility/lock toggles, reordering, inline renaming, and selection.

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, TextEditBuffer, Vec2};
use ui_platform::{PointerButton, PointerState};
use ui_render::{CommandBuffer, DrawCommand};

/// A single item within the hierarchy tree.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyItem {
    pub id: String,
    pub name: String,
    pub depth: usize,
    pub visible: bool,
    pub locked: bool,
    pub selected: bool,
    pub is_expanded: bool,
    pub has_children: bool,
    pub opacity: f32, // 0.0 .. 1.0
    pub blend_mode: String,
}

impl HierarchyItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            depth: 0,
            visible: true,
            locked: false,
            selected: false,
            is_expanded: false,
            has_children: false,
            opacity: 1.0,
            blend_mode: "Normal".into(),
        }
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }
}

/// Events emitted by interactive hierarchy tree actions.
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchyEvent {
    Selected(String),
    ToggleVisibility(String),
    ToggleLocked(String),
    ToggleExpanded(String),
    Renamed { id: String, new_name: String },
    Reordered { from_idx: usize, to_idx: usize },
    AddLayer,
    DeleteSelected,
}

/// Reusable HierarchyTree widget.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HierarchyTree {
    pub items: Vec<HierarchyItem>,
    pub selected_id: Option<String>,
    pub renaming_id: Option<String>,
    pub rename_buffer: TextEditBuffer,
    /// Toolbar title; empty shows "LAYERS". A 3D editor sets "Scene Hierarchy".
    pub title: String,
}

impl HierarchyTree {
    pub fn new() -> Self {
        Self::with_items(vec![
            HierarchyItem::new("layer_base", "Background Canvas"),
            HierarchyItem::new("layer_sketch", "Rough Sketch"),
        ])
    }

    pub fn with_items(items: Vec<HierarchyItem>) -> Self {
        let selected_id = items.iter().find(|i| i.selected).map(|i| i.id.clone());
        Self {
            items,
            selected_id,
            renaming_id: None,
            rename_buffer: TextEditBuffer::new(""),
            title: String::new(),
        }
    }

    pub fn select(&mut self, id: &str) {
        self.selected_id = Some(id.to_string());
        for item in &mut self.items {
            item.selected = item.id == id;
        }
    }

    pub fn toggle_visibility(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.visible = !item.visible;
        }
    }

    pub fn toggle_lock(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.locked = !item.locked;
        }
    }

    pub fn start_rename(&mut self, id: &str) {
        if let Some(item) = self.items.iter().find(|i| i.id == id) {
            self.renaming_id = Some(id.to_string());
            self.rename_buffer = TextEditBuffer::new(&item.name);
        }
    }

    pub fn cancel_rename(&mut self) {
        self.renaming_id = None;
    }

    pub fn selected_item(&self) -> Option<&HierarchyItem> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.items.iter().find(|i| &i.id == id))
    }

    pub fn selected_item_mut(&mut self) -> Option<&mut HierarchyItem> {
        let target = self.selected_id.clone()?;
        self.items.iter_mut().find(|i| i.id == target)
    }

    pub fn add_item(&mut self, name: impl Into<String>) -> String {
        let id = format!("layer_{}", self.items.len() + 1);
        let mut item = HierarchyItem::new(id.clone(), name);
        item.selected = true;
        for existing in &mut self.items {
            existing.selected = false;
        }
        self.items.insert(0, item);
        self.selected_id = Some(id.clone());
        id
    }

    pub fn delete_selected(&mut self) -> Option<String> {
        let sel = self.selected_id.clone()?;
        if let Some(pos) = self.items.iter().position(|i| i.id == sel) {
            self.items.remove(pos);
            self.selected_id = self
                .items
                .get(pos.min(self.items.len().saturating_sub(1)))
                .map(|i| i.id.clone());
            if let Some(ref new_sel) = self.selected_id {
                for item in &mut self.items {
                    item.selected = &item.id == new_sel;
                }
            }
            Some(sel)
        } else {
            None
        }
    }

    pub fn move_selected_up(&mut self) -> bool {
        let sel = match self.selected_id.as_ref() {
            Some(id) => id.clone(),
            None => return false,
        };
        let pos = match self.items.iter().position(|i| i.id == sel) {
            Some(p) => p,
            None => return false,
        };
        if pos > 0 {
            self.items.swap(pos, pos - 1);
            true
        } else {
            false
        }
    }

    pub fn move_selected_down(&mut self) -> bool {
        let sel = match self.selected_id.as_ref() {
            Some(id) => id.clone(),
            None => return false,
        };
        let pos = match self.items.iter().position(|i| i.id == sel) {
            Some(p) => p,
            None => return false,
        };
        if pos + 1 < self.items.len() {
            self.items.swap(pos, pos + 1);
            true
        } else {
            false
        }
    }

    /// Handles pointer clicks and interaction. Returns any emitted event.
    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        viewport: Rect,
        ptr: &PointerState,
    ) -> Option<HierarchyEvent> {
        if !viewport.contains(pos) || button != PointerButton::Primary {
            return None;
        }

        // Toolbar buttons hit test (at top of panel)
        let toolbar_h = 32.0;
        let toolbar_rect =
            Rect::from_min_size(viewport.min, Vec2::new(viewport.width(), toolbar_h));
        if toolbar_rect.contains(pos) {
            let add_btn = Rect::from_min_size(
                Vec2::new(viewport.max.x - 70.0, viewport.min.y + 4.0),
                Vec2::new(28.0, 24.0),
            );
            let del_btn = Rect::from_min_size(
                Vec2::new(viewport.max.x - 36.0, viewport.min.y + 4.0),
                Vec2::new(28.0, 24.0),
            );

            if add_btn.contains(pos) {
                self.add_item("New Layer");
                return Some(HierarchyEvent::AddLayer);
            }
            if del_btn.contains(pos) {
                if let Some(_id) = self.delete_selected() {
                    return Some(HierarchyEvent::DeleteSelected);
                }
            }
            return None;
        }

        // Items list hit test
        let mut y = viewport.min.y + toolbar_h + 4.0;
        let row_h = 28.0;

        for (idx, item) in self.items.iter_mut().enumerate() {
            let row_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + 4.0, y),
                Vec2::new(viewport.width() - 8.0, row_h),
            );
            if row_rect.contains(pos) {
                // Check eye visibility icon
                let eye_rect = Rect::from_min_size(
                    Vec2::new(row_rect.min.x + 4.0, row_rect.min.y + 4.0),
                    Vec2::new(20.0, 20.0),
                );
                if eye_rect.contains(pos) {
                    item.visible = !item.visible;
                    return Some(HierarchyEvent::ToggleVisibility(item.id.clone()));
                }

                // Check lock icon
                let lock_rect = Rect::from_min_size(
                    Vec2::new(row_rect.min.x + 28.0, row_rect.min.y + 4.0),
                    Vec2::new(20.0, 20.0),
                );
                if lock_rect.contains(pos) {
                    item.locked = !item.locked;
                    return Some(HierarchyEvent::ToggleLocked(item.id.clone()));
                }

                // Double click on name triggers inline renaming
                if ptr.double_clicked {
                    self.renaming_id = Some(item.id.clone());
                    self.rename_buffer = TextEditBuffer::new(&item.name);
                    self.rename_buffer.set_focused(true);
                    return None;
                }

                let id = item.id.clone();
                self.selected_id = Some(id.clone());
                for (i, it) in self.items.iter_mut().enumerate() {
                    it.selected = i == idx;
                }
                return Some(HierarchyEvent::Selected(id));
            }
            y += row_h + 2.0;
        }

        None
    }

    /// Commits active inline renaming.
    pub fn commit_rename(&mut self) -> Option<HierarchyEvent> {
        if let Some(target_id) = self.renaming_id.take() {
            let new_name = self.rename_buffer.as_str().trim().to_string();
            if !new_name.is_empty() {
                if let Some(item) = self.items.iter_mut().find(|i| i.id == target_id) {
                    item.name = new_name.clone();
                    return Some(HierarchyEvent::Renamed {
                        id: target_id,
                        new_name,
                    });
                }
            }
        }
        None
    }

    /// Renders the hierarchy tree into the command buffer.
    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Background
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // Toolbar header
        let toolbar_h = 32.0;
        let toolbar_rect =
            Rect::from_min_size(viewport.min, Vec2::new(viewport.width(), toolbar_h));
        buf.push(DrawCommand::Rect {
            rect: toolbar_rect,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        buf.push(DrawCommand::Text {
            text: format!(
                "{} ({})",
                if self.title.is_empty() {
                    "LAYERS"
                } else {
                    &self.title
                },
                self.items.len()
            ),
            pos: Vec2::new(toolbar_rect.min.x + 10.0, toolbar_rect.min.y + 9.0),
            size: 11.5,
            color: palette.text,
            strong: true,
        });

        // Add layer button
        let add_btn = Rect::from_min_size(
            Vec2::new(viewport.max.x - 70.0, viewport.min.y + 4.0),
            Vec2::new(28.0, 24.0),
        );
        buf.push(DrawCommand::Rect {
            rect: add_btn,
            fill: palette.surface,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        buf.push(DrawCommand::Text {
            text: "+".into(),
            pos: Vec2::new(add_btn.min.x + 10.0, add_btn.min.y + 5.0),
            size: 13.0,
            color: palette.text,
            strong: true,
        });

        // Delete layer button
        let del_btn = Rect::from_min_size(
            Vec2::new(viewport.max.x - 36.0, viewport.min.y + 4.0),
            Vec2::new(28.0, 24.0),
        );
        buf.push(DrawCommand::Rect {
            rect: del_btn,
            fill: palette.surface,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        buf.push(DrawCommand::Text {
            text: "-".into(),
            pos: Vec2::new(del_btn.min.x + 11.0, del_btn.min.y + 5.0),
            size: 14.0,
            color: palette.danger,
            strong: true,
        });

        // Layer rows
        let mut y = viewport.min.y + toolbar_h + 4.0;
        let row_h = 28.0;

        for item in &self.items {
            let row_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + 4.0, y),
                Vec2::new(viewport.width() - 8.0, row_h),
            );
            let is_sel = item.selected;

            buf.push(DrawCommand::Rect {
                rect: row_rect,
                fill: if is_sel {
                    palette.accent.with_alpha(50)
                } else {
                    palette.canvas.with_alpha(100)
                },
                radius: CornerRadius::same(4.0),
                stroke: if is_sel {
                    Stroke::new(1.0, palette.accent)
                } else {
                    Stroke::new(1.0, palette.border.with_alpha(50))
                },
            });

            // Visibility Eye indicator
            let eye_text = if item.visible { "👁" } else { "⊘" };
            buf.push(DrawCommand::Text {
                text: eye_text.into(),
                pos: Vec2::new(row_rect.min.x + 8.0, row_rect.min.y + 6.0),
                size: 12.0,
                color: if item.visible {
                    palette.text
                } else {
                    palette.text_muted.with_alpha(100)
                },
                strong: false,
            });

            // Lock indicator
            let lock_text = if item.locked { "🔒" } else { "🔓" };
            buf.push(DrawCommand::Text {
                text: lock_text.into(),
                pos: Vec2::new(row_rect.min.x + 30.0, row_rect.min.y + 6.0),
                size: 11.0,
                color: if item.locked {
                    palette.warning
                } else {
                    palette.text_muted.with_alpha(80)
                },
                strong: false,
            });

            // Layer Name (or inline editor if renaming)
            let indent = item.depth as f32 * 14.0;
            let name_x = row_rect.min.x + 52.0 + indent;

            if self.renaming_id.as_deref() == Some(&item.id) {
                let edit_rect = Rect::from_min_size(
                    Vec2::new(name_x, row_rect.min.y + 2.0),
                    Vec2::new(row_rect.width() - 60.0 - indent, row_h - 4.0),
                );
                buf.push(DrawCommand::Rect {
                    rect: edit_rect,
                    fill: palette.elevated,
                    radius: CornerRadius::same(3.0),
                    stroke: Stroke::new(1.0, palette.accent),
                });
                buf.push(DrawCommand::Text {
                    text: format!("{}_", self.rename_buffer.as_str()),
                    pos: Vec2::new(edit_rect.min.x + 6.0, edit_rect.min.y + 5.0),
                    size: 12.0,
                    color: palette.text,
                    strong: true,
                });
            } else {
                buf.push(DrawCommand::Text {
                    text: item.name.clone(),
                    pos: Vec2::new(name_x, row_rect.min.y + 7.0),
                    size: 12.0,
                    color: if item.visible {
                        palette.text
                    } else {
                        palette.text_muted
                    },
                    strong: is_sel,
                });
            }

            // Opacity readout on right
            let opacity_text = format!("{:.0}%", item.opacity * 100.0);
            buf.push(DrawCommand::Text {
                text: opacity_text,
                pos: Vec2::new(row_rect.max.x - 42.0, row_rect.min.y + 8.0),
                size: 10.5,
                color: palette.text_muted,
                strong: false,
            });

            y += row_h + 2.0;
        }

        buf.push(DrawCommand::ClipPop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_themes::ThemePalette;

    #[test]
    fn test_hierarchy_tree_layer_management() {
        let mut tree = HierarchyTree::new();
        assert_eq!(
            tree.items.len(),
            2,
            "Default hierarchy should contain 2 sample layers"
        );

        let new_id = tree.add_item("Foreground Lineart");
        assert_eq!(tree.items.len(), 3);
        assert_eq!(tree.selected_id.as_deref(), Some(new_id.as_str()));

        tree.toggle_visibility(&new_id);
        let item = tree.items.iter().find(|i| i.id == new_id).unwrap();
        assert!(!item.visible);

        tree.toggle_lock(&new_id);
        let item = tree.items.iter().find(|i| i.id == new_id).unwrap();
        assert!(item.locked);

        // Reordering
        assert_eq!(tree.items[0].id, new_id);
        tree.move_selected_down();
        assert_eq!(tree.items[1].id, new_id);
        tree.move_selected_up();
        assert_eq!(tree.items[0].id, new_id);

        // Deletion
        let deleted = tree.delete_selected();
        assert_eq!(deleted, Some(new_id));
        assert_eq!(tree.items.len(), 2);
    }

    #[test]
    fn test_hierarchy_inline_renaming() {
        let mut tree = HierarchyTree::new();
        let target_id = tree.items[0].id.clone();

        tree.start_rename(&target_id);
        assert_eq!(tree.renaming_id.as_deref(), Some(target_id.as_str()));

        tree.rename_buffer.clear();
        tree.rename_buffer.insert_str("Ink Inpainting");
        tree.commit_rename();

        assert_eq!(tree.renaming_id, None);
        assert_eq!(tree.items[0].name, "Ink Inpainting");
    }

    #[test]
    fn test_hierarchy_rendering_emits_commands() {
        let tree = HierarchyTree::new();
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 500.0));

        tree.render(&mut buf, viewport, &palette);
        assert!(!buf.commands().is_empty());
    }
}
