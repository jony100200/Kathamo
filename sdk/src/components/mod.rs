//! Standard reusable creative and application components for Kathamo SDK.

use ui_core::*;
use ui_platform::{PointerButton, PointerState};
use ui_render::command::DrawCommand;
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

// -----------------------------------------------------------------------------
// 1. MENU SYSTEM & TOOLBAR
// -----------------------------------------------------------------------------

/// Item within a dropdown or context menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub separator_after: bool,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            enabled: true,
            separator_after: false,
        }
    }

    pub fn action(id: impl Into<String>, label: impl Into<String>, shortcut: Option<&str>) -> Self {
        let mut item = Self::new(id, label);
        if let Some(sc) = shortcut {
            item = item.with_shortcut(sc);
        }
        item
    }

    pub fn separator() -> Self {
        Self::new("sep", "").with_separator()
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_separator(mut self) -> Self {
        self.separator_after = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Menu containing a list of menu items.
#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(title: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            title: title.into(),
            items,
        }
    }
}

/// Top application MenuBar.
#[derive(Debug, Clone, Default)]
pub struct MenuBar {
    pub menus: Vec<Menu>,
    pub open_menu_idx: Option<usize>,
}

impl MenuBar {
    pub fn new(menus: Vec<Menu>) -> Self {
        Self {
            menus,
            open_menu_idx: None,
        }
    }

    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        bounds: Rect,
    ) -> Option<String> {
        if button != PointerButton::Primary {
            return None;
        }

        // Check if clicked in menubar header
        if bounds.contains(pos) {
            let mut x = bounds.min.x + 8.0;
            let item_width = 54.0;
            for (idx, _menu) in self.menus.iter().enumerate() {
                let rect = Rect::from_min_size(
                    Vec2::new(x, bounds.min.y),
                    Vec2::new(item_width, bounds.height()),
                );
                if rect.contains(pos) {
                    if self.open_menu_idx == Some(idx) {
                        self.open_menu_idx = None;
                    } else {
                        self.open_menu_idx = Some(idx);
                    }
                    return None;
                }
                x += item_width + 4.0;
            }
        }

        // Check if clicked inside open dropdown
        if let Some(idx) = self.open_menu_idx {
            if let Some(menu) = self.menus.get(idx) {
                let menu_x = bounds.min.x + 8.0 + (idx as f32 * 58.0);
                let dropdown_rect = Rect::from_min_size(
                    Vec2::new(menu_x, bounds.max.y),
                    Vec2::new(180.0, menu.items.len() as f32 * 26.0 + 8.0),
                );

                if dropdown_rect.contains(pos) {
                    let rel_y = pos.y - (dropdown_rect.min.y + 4.0);
                    let item_idx = (rel_y / 26.0).floor() as usize;
                    if let Some(item) = menu.items.get(item_idx) {
                        if item.enabled {
                            self.open_menu_idx = None;
                            return Some(item.id.clone());
                        }
                    }
                } else {
                    self.open_menu_idx = None;
                }
            }
        }

        None
    }

    pub fn render(
        &self,
        cmd_buf: &mut CommandBuffer,
        bounds: Rect,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        // Render bar background
        cmd_buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border.with_alpha(100)),
        });

        let mut x = bounds.min.x + 8.0;
        let item_width = 54.0;

        for (idx, menu) in self.menus.iter().enumerate() {
            let item_rect = Rect::from_min_size(
                Vec2::new(x, bounds.min.y + 2.0),
                Vec2::new(item_width, bounds.height() - 4.0),
            );
            let is_open = self.open_menu_idx == Some(idx);
            let is_hovered = item_rect.contains(ptr.position);

            if is_open || is_hovered {
                cmd_buf.push(DrawCommand::Rect {
                    rect: item_rect,
                    fill: if is_open {
                        palette.elevated_high
                    } else {
                        palette.elevated
                    },
                    radius: CornerRadius::same(3.0),
                    stroke: Stroke::NONE,
                });
            }

            cmd_buf.push(DrawCommand::Text {
                text: menu.title.clone(),
                pos: Vec2::new(x + 10.0, bounds.min.y + 6.0),
                size: 12.0,
                color: palette.text,
                strong: is_open,
            });

            x += item_width + 4.0;
        }

        // Render open dropdown if active
        if let Some(idx) = self.open_menu_idx {
            if let Some(menu) = self.menus.get(idx) {
                let menu_x = bounds.min.x + 8.0 + (idx as f32 * 58.0);
                let dropdown_rect = Rect::from_min_size(
                    Vec2::new(menu_x, bounds.max.y),
                    Vec2::new(190.0, menu.items.len() as f32 * 26.0 + 8.0),
                );

                // Dropdown shadow & card
                cmd_buf.push(DrawCommand::Rect {
                    rect: dropdown_rect.expand(2.0),
                    fill: Color::rgba(0, 0, 0, 80),
                    radius: CornerRadius::same(6.0),
                    stroke: Stroke::NONE,
                });
                cmd_buf.push(DrawCommand::Rect {
                    rect: dropdown_rect,
                    fill: palette.elevated,
                    radius: CornerRadius::same(5.0),
                    stroke: Stroke::new(1.0, palette.border),
                });

                let mut item_y = dropdown_rect.min.y + 4.0;
                for item in &menu.items {
                    let r = Rect::from_min_size(
                        Vec2::new(dropdown_rect.min.x + 4.0, item_y),
                        Vec2::new(dropdown_rect.width() - 8.0, 24.0),
                    );
                    let is_hov = r.contains(ptr.position) && item.enabled;

                    if is_hov {
                        cmd_buf.push(DrawCommand::Rect {
                            rect: r,
                            fill: palette.elevated_high,
                            radius: CornerRadius::same(3.0),
                            stroke: Stroke::NONE,
                        });
                    }

                    cmd_buf.push(DrawCommand::Text {
                        text: item.label.clone(),
                        pos: Vec2::new(r.min.x + 8.0, r.min.y + 5.0),
                        size: 11.5,
                        color: if item.enabled {
                            palette.text
                        } else {
                            palette.text_muted.with_alpha(120)
                        },
                        strong: false,
                    });

                    if let Some(sc) = &item.shortcut {
                        cmd_buf.push(DrawCommand::Text {
                            text: sc.clone(),
                            pos: Vec2::new(r.max.x - 45.0, r.min.y + 5.0),
                            size: 10.5,
                            color: palette.text_muted,
                            strong: false,
                        });
                    }

                    item_y += 26.0;
                }
            }
        }
    }
}

/// Floating context menu triggered by right-click.
#[derive(Debug, Clone, Default)]
pub struct ContextMenu {
    pub position: Vec2,
    pub items: Vec<MenuItem>,
    pub is_open: bool,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_at(&mut self, pos: Vec2, items: Vec<MenuItem>) {
        self.position = pos;
        self.items = items;
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.items.clear();
    }

    pub fn bounds(&self) -> Rect {
        if !self.is_open {
            return Rect::ZERO;
        }
        Rect::from_min_size(
            self.position,
            Vec2::new(180.0, self.items.len() as f32 * 26.0 + 8.0),
        )
    }

    pub fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton) -> Option<String> {
        if !self.is_open {
            return None;
        }
        if button != PointerButton::Primary {
            self.close();
            return None;
        }

        let b = self.bounds();
        if b.contains(pos) {
            let rel_y = pos.y - (b.min.y + 4.0);
            let item_idx = (rel_y / 26.0).floor() as usize;
            if let Some(item) = self.items.get(item_idx) {
                if item.enabled {
                    let id = item.id.clone();
                    self.close();
                    return Some(id);
                }
            }
        } else {
            self.close();
        }
        None
    }

    pub fn render(&self, cmd_buf: &mut CommandBuffer, ptr: &PointerState, palette: &ThemePalette) {
        if !self.is_open || self.items.is_empty() {
            return;
        }

        let b = self.bounds();
        cmd_buf.push(DrawCommand::Rect {
            rect: b.expand(3.0),
            fill: Color::rgba(0, 0, 0, 90),
            radius: CornerRadius::same(6.0),
            stroke: Stroke::NONE,
        });
        cmd_buf.push(DrawCommand::Rect {
            rect: b,
            fill: palette.elevated,
            radius: CornerRadius::same(5.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        let mut item_y = b.min.y + 4.0;
        for item in &self.items {
            let r = Rect::from_min_size(
                Vec2::new(b.min.x + 4.0, item_y),
                Vec2::new(b.width() - 8.0, 24.0),
            );
            let is_hov = r.contains(ptr.position) && item.enabled;

            if is_hov {
                cmd_buf.push(DrawCommand::Rect {
                    rect: r,
                    fill: palette.elevated_high,
                    radius: CornerRadius::same(3.0),
                    stroke: Stroke::NONE,
                });
            }

            cmd_buf.push(DrawCommand::Text {
                text: item.label.clone(),
                pos: Vec2::new(r.min.x + 8.0, r.min.y + 5.0),
                size: 11.5,
                color: if item.enabled {
                    palette.text
                } else {
                    palette.text_muted.with_alpha(120)
                },
                strong: false,
            });

            if let Some(sc) = &item.shortcut {
                cmd_buf.push(DrawCommand::Text {
                    text: sc.clone(),
                    pos: Vec2::new(r.max.x - 45.0, r.min.y + 5.0),
                    size: 10.5,
                    color: palette.text_muted,
                    strong: false,
                });
            }

            item_y += 26.0;
        }
    }
}

// -----------------------------------------------------------------------------
// 2. DIALOG & MODAL SYSTEM
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    Confirm,
    Cancel,
    Custom(u32),
}

/// Modal Dialog Host.
#[derive(Debug, Clone, Default)]
pub struct DialogHost {
    pub is_open: bool,
    pub title: String,
    pub message: String,
    pub confirm_text: String,
    pub cancel_text: String,
    pub dialog_id: String,
}

impl DialogHost {
    pub fn new() -> Self {
        Self {
            is_open: false,
            title: String::new(),
            message: String::new(),
            confirm_text: "Confirm".into(),
            cancel_text: "Cancel".into(),
            dialog_id: String::new(),
        }
    }

    pub fn open(
        &mut self,
        dialog_id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.dialog_id = dialog_id.into();
        self.title = title.into();
        self.message = message.into();
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        screen_bounds: Rect,
    ) -> Option<(String, DialogAction)> {
        if !self.is_open || button != PointerButton::Primary {
            return None;
        }

        let modal_w = 420.0;
        let modal_h = 180.0;
        let center = screen_bounds.center();
        let modal_rect = Rect::from_min_size(
            Vec2::new(center.x - modal_w * 0.5, center.y - modal_h * 0.5),
            Vec2::new(modal_w, modal_h),
        );

        let btn_w = 80.0;
        let btn_h = 28.0;
        let cancel_rect = Rect::from_min_size(
            Vec2::new(modal_rect.max.x - 180.0, modal_rect.max.y - 40.0),
            Vec2::new(btn_w, btn_h),
        );
        let confirm_rect = Rect::from_min_size(
            Vec2::new(modal_rect.max.x - 90.0, modal_rect.max.y - 40.0),
            Vec2::new(btn_w, btn_h),
        );

        if confirm_rect.contains(pos) {
            let id = self.dialog_id.clone();
            self.close();
            return Some((id, DialogAction::Confirm));
        }

        if cancel_rect.contains(pos) {
            let id = self.dialog_id.clone();
            self.close();
            return Some((id, DialogAction::Cancel));
        }

        None
    }

    pub fn render(
        &self,
        cmd_buf: &mut CommandBuffer,
        screen_bounds: Rect,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        if !self.is_open {
            return;
        }

        // 1. Semi-transparent scrim
        cmd_buf.push(DrawCommand::Rect {
            rect: screen_bounds,
            fill: Color::rgba(0, 0, 0, 160),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // 2. Dialog Card
        let modal_w = 420.0;
        let modal_h = 180.0;
        let center = screen_bounds.center();
        let modal_rect = Rect::from_min_size(
            Vec2::new(center.x - modal_w * 0.5, center.y - modal_h * 0.5),
            Vec2::new(modal_w, modal_h),
        );

        cmd_buf.push(DrawCommand::Rect {
            rect: modal_rect,
            fill: palette.elevated,
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Title
        cmd_buf.push(DrawCommand::Text {
            text: self.title.clone(),
            pos: Vec2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 18.0),
            size: 15.0,
            color: palette.text,
            strong: true,
        });

        // Message
        cmd_buf.push(DrawCommand::Text {
            text: self.message.clone(),
            pos: Vec2::new(modal_rect.min.x + 20.0, modal_rect.min.y + 55.0),
            size: 12.5,
            color: palette.text_muted,
            strong: false,
        });

        // Cancel Button
        let btn_w = 80.0;
        let btn_h = 28.0;
        let cancel_rect = Rect::from_min_size(
            Vec2::new(modal_rect.max.x - 180.0, modal_rect.max.y - 40.0),
            Vec2::new(btn_w, btn_h),
        );
        let cancel_hov = cancel_rect.contains(ptr.position);
        cmd_buf.push(DrawCommand::Rect {
            rect: cancel_rect,
            fill: if cancel_hov {
                palette.elevated
            } else {
                palette.surface
            },
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        cmd_buf.push(DrawCommand::Text {
            text: self.cancel_text.clone(),
            pos: Vec2::new(cancel_rect.min.x + 18.0, cancel_rect.min.y + 7.0),
            size: 11.5,
            color: palette.text,
            strong: false,
        });

        // Confirm Button (Accent)
        let confirm_rect = Rect::from_min_size(
            Vec2::new(modal_rect.max.x - 90.0, modal_rect.max.y - 40.0),
            Vec2::new(btn_w, btn_h),
        );
        let confirm_hov = confirm_rect.contains(ptr.position);
        cmd_buf.push(DrawCommand::Rect {
            rect: confirm_rect,
            fill: if confirm_hov {
                palette.accent.with_alpha(220)
            } else {
                palette.accent
            },
            radius: CornerRadius::same(4.0),
            stroke: Stroke::NONE,
        });
        cmd_buf.push(DrawCommand::Text {
            text: self.confirm_text.clone(),
            pos: Vec2::new(confirm_rect.min.x + 16.0, confirm_rect.min.y + 7.0),
            size: 11.5,
            color: Color::WHITE,
            strong: true,
        });
    }
}

// -----------------------------------------------------------------------------
// 3. NOTIFICATION & TOAST SYSTEM
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct NotificationToast {
    pub id: String,
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub created_frame: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationHost {
    pub notifications: Vec<NotificationToast>,
    pub current_frame: u64,
}

impl NotificationHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
        level: NotificationLevel,
    ) {
        self.notifications.push(NotificationToast {
            id: id.into(),
            title: title.into(),
            message: message.into(),
            level,
            created_frame: self.current_frame,
        });
    }

    pub fn info(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast_{}", self.notifications.len());
        self.push(id, title, message, NotificationLevel::Info);
    }

    pub fn success(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast_{}", self.notifications.len());
        self.push(id, title, message, NotificationLevel::Success);
    }

    pub fn warning(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast_{}", self.notifications.len());
        self.push(id, title, message, NotificationLevel::Warning);
    }

    pub fn error(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let id = format!("toast_{}", self.notifications.len());
        self.push(id, title, message, NotificationLevel::Error);
    }

    pub fn dismiss(&mut self, id: &str) {
        self.notifications.retain(|n| n.id != id);
    }

    pub fn tick(&mut self) {
        self.current_frame += 1;
        let cur = self.current_frame;
        // Auto-dismiss after 300 frames (~5 seconds at 60fps)
        self.notifications
            .retain(|n| cur.saturating_sub(n.created_frame) < 300);
    }

    pub fn render(&self, cmd_buf: &mut CommandBuffer, screen_bounds: Rect, palette: &ThemePalette) {
        let toast_w = 300.0;
        let toast_h = 56.0;
        let mut y = screen_bounds.min.y + 40.0;

        for n in self.notifications.iter().rev().take(4) {
            let toast_rect = Rect::from_min_size(
                Vec2::new(screen_bounds.max.x - toast_w - 16.0, y),
                Vec2::new(toast_w, toast_h),
            );

            // Shadow
            cmd_buf.push(DrawCommand::Rect {
                rect: toast_rect.expand(2.0),
                fill: Color::rgba(0, 0, 0, 70),
                radius: CornerRadius::same(6.0),
                stroke: Stroke::NONE,
            });

            // Card
            cmd_buf.push(DrawCommand::Rect {
                rect: toast_rect,
                fill: palette.elevated,
                radius: CornerRadius::same(5.0),
                stroke: Stroke::new(1.0, palette.border),
            });

            // Accent indicator bar
            let indicator_color = match n.level {
                NotificationLevel::Info => palette.accent,
                NotificationLevel::Success => Color::rgb(50, 180, 80),
                NotificationLevel::Warning => Color::rgb(240, 160, 30),
                NotificationLevel::Error => Color::rgb(230, 60, 60),
            };
            cmd_buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(toast_rect.min, Vec2::new(4.0, toast_rect.height())),
                fill: indicator_color,
                radius: CornerRadius::ZERO,
                stroke: Stroke::NONE,
            });

            // Title
            cmd_buf.push(DrawCommand::Text {
                text: n.title.clone(),
                pos: Vec2::new(toast_rect.min.x + 14.0, toast_rect.min.y + 10.0),
                size: 12.0,
                color: palette.text,
                strong: true,
            });

            // Message
            cmd_buf.push(DrawCommand::Text {
                text: n.message.clone(),
                pos: Vec2::new(toast_rect.min.x + 14.0, toast_rect.min.y + 28.0),
                size: 11.0,
                color: palette.text_muted,
                strong: false,
            });

            y += toast_h + 8.0;
        }
    }
}

// -----------------------------------------------------------------------------
// 4. ASSET BROWSER / GALLERY COMPONENT
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AssetItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub meta: String,
    pub texture_id: Option<u64>,
}

impl AssetItem {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            meta: String::new(),
            texture_id: None,
        }
    }

    pub fn with_meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = meta.into();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetViewMode {
    Grid,
    List,
}

#[derive(Debug, Clone)]
pub struct AssetBrowser {
    pub items: Vec<AssetItem>,
    pub selected_id: Option<String>,
    pub active_category: String,
    pub search_query: String,
    pub view_mode: AssetViewMode,
    pub scroll_offset: f32,
}

impl Default for AssetBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetBrowser {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected_id: None,
            active_category: "All".into(),
            search_query: String::new(),
            view_mode: AssetViewMode::Grid,
            scroll_offset: 0.0,
        }
    }

    pub fn add_item(&mut self, item: AssetItem) {
        self.items.push(item);
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats = vec!["All".to_string()];
        for item in &self.items {
            if !cats.contains(&item.category) {
                cats.push(item.category.clone());
            }
        }
        cats
    }

    pub fn filtered_items(&self) -> Vec<&AssetItem> {
        self.items
            .iter()
            .filter(|i| {
                if self.active_category != "All" && i.category != self.active_category {
                    return false;
                }
                if !self.search_query.is_empty()
                    && !i
                        .name
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                {
                    return false;
                }
                true
            })
            .collect()
    }

    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        bounds: Rect,
    ) -> Option<String> {
        if button != PointerButton::Primary {
            return None;
        }

        let header_h = 36.0;
        let content_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + header_h),
            Vec2::new(bounds.width(), bounds.height() - header_h),
        );

        if content_rect.contains(pos) {
            let hit_id = match self.view_mode {
                AssetViewMode::Grid => {
                    let card_w = 110.0;
                    let card_h = 120.0;
                    let padding = 10.0;
                    let cols = ((content_rect.width() - padding) / (card_w + padding))
                        .floor()
                        .max(1.0) as usize;
                    let mut found = None;

                    for (idx, item) in self.filtered_items().iter().enumerate() {
                        let col = idx % cols;
                        let row = idx / cols;
                        let x = content_rect.min.x + padding + (col as f32 * (card_w + padding));
                        let y = content_rect.min.y + padding + (row as f32 * (card_h + padding))
                            - self.scroll_offset;
                        let r = Rect::from_min_size(Vec2::new(x, y), Vec2::new(card_w, card_h));
                        if r.contains(pos) {
                            found = Some(item.id.clone());
                            break;
                        }
                    }
                    found
                }
                AssetViewMode::List => {
                    let row_h = 28.0;
                    let mut found = None;
                    for (idx, item) in self.filtered_items().iter().enumerate() {
                        let y =
                            content_rect.min.y + 4.0 + (idx as f32 * row_h) - self.scroll_offset;
                        let r = Rect::from_min_size(
                            Vec2::new(content_rect.min.x + 8.0, y),
                            Vec2::new(content_rect.width() - 16.0, row_h - 2.0),
                        );
                        if r.contains(pos) {
                            found = Some(item.id.clone());
                            break;
                        }
                    }
                    found
                }
            };

            if let Some(id) = hit_id {
                self.selected_id = Some(id.clone());
                return Some(id);
            }
        }

        None
    }

    pub fn render(
        &self,
        cmd_buf: &mut CommandBuffer,
        bounds: Rect,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        // Container background
        cmd_buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Header toolbar (Filter pills & search status)
        let header_h = 36.0;
        let header_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), header_h));
        cmd_buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border.with_alpha(120)),
        });

        // Render category tabs
        let mut cat_x = header_rect.min.x + 8.0;
        for cat in self.categories() {
            let is_sel = self.active_category == cat;
            let pill_w = cat.len() as f32 * 7.0 + 16.0;
            let pill_r = Rect::from_min_size(
                Vec2::new(cat_x, header_rect.min.y + 6.0),
                Vec2::new(pill_w, 24.0),
            );

            cmd_buf.push(DrawCommand::Rect {
                rect: pill_r,
                fill: if is_sel {
                    palette.accent
                } else {
                    palette.surface
                },
                radius: CornerRadius::same(12.0),
                stroke: if is_sel {
                    Stroke::NONE
                } else {
                    Stroke::new(1.0, palette.border)
                },
            });
            cmd_buf.push(DrawCommand::Text {
                text: cat.clone(),
                pos: Vec2::new(pill_r.min.x + 8.0, pill_r.min.y + 5.0),
                size: 11.0,
                color: if is_sel {
                    Color::WHITE
                } else {
                    palette.text_muted
                },
                strong: is_sel,
            });

            cat_x += pill_w + 6.0;
        }

        // Render Items
        let content_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + header_h),
            Vec2::new(bounds.width(), bounds.height() - header_h),
        );
        let filtered = self.filtered_items();

        match self.view_mode {
            AssetViewMode::Grid => {
                let card_w = 110.0;
                let card_h = 120.0;
                let padding = 10.0;
                let cols = ((content_rect.width() - padding) / (card_w + padding))
                    .floor()
                    .max(1.0) as usize;

                for (idx, item) in filtered.iter().enumerate() {
                    let col = idx % cols;
                    let row = idx / cols;
                    let x = content_rect.min.x + padding + (col as f32 * (card_w + padding));
                    let y = content_rect.min.y + padding + (row as f32 * (card_h + padding))
                        - self.scroll_offset;
                    let r = Rect::from_min_size(Vec2::new(x, y), Vec2::new(card_w, card_h));

                    if r.max.y < content_rect.min.y || r.min.y > content_rect.max.y {
                        continue;
                    }

                    let is_sel = self.selected_id.as_deref() == Some(&item.id);
                    let is_hov = r.contains(ptr.position);

                    // Card body
                    cmd_buf.push(DrawCommand::Rect {
                        rect: r,
                        fill: if is_sel {
                            palette.accent.with_alpha(40)
                        } else if is_hov {
                            palette.elevated
                        } else {
                            palette.surface
                        },
                        radius: CornerRadius::same(6.0),
                        stroke: if is_sel {
                            Stroke::new(1.5, palette.accent)
                        } else {
                            Stroke::new(1.0, palette.border)
                        },
                    });

                    // Thumbnail preview block
                    let thumb_r = Rect::from_min_size(
                        Vec2::new(r.min.x + 6.0, r.min.y + 6.0),
                        Vec2::new(r.width() - 12.0, 70.0),
                    );
                    if let Some(tex_id) = item.texture_id {
                        cmd_buf.push(DrawCommand::Image {
                            rect: thumb_r,
                            texture_id: tex_id,
                            uv_rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(1.0, 1.0)),
                            tint: Color::WHITE,
                        });
                    } else {
                        cmd_buf.push(DrawCommand::Rect {
                            rect: thumb_r,
                            fill: palette.canvas,
                            radius: CornerRadius::same(4.0),
                            stroke: Stroke::NONE,
                        });
                        cmd_buf.push(DrawCommand::Text {
                            text: "[Thumb]".into(),
                            pos: Vec2::new(thumb_r.min.x + 18.0, thumb_r.min.y + 28.0),
                            size: 10.0,
                            color: palette.text_muted.with_alpha(100),
                            strong: false,
                        });
                    }

                    // Item Name
                    cmd_buf.push(DrawCommand::Text {
                        text: item.name.clone(),
                        pos: Vec2::new(r.min.x + 8.0, r.min.y + 82.0),
                        size: 11.0,
                        color: palette.text,
                        strong: is_sel,
                    });

                    // Meta / Category
                    cmd_buf.push(DrawCommand::Text {
                        text: item.category.clone(),
                        pos: Vec2::new(r.min.x + 8.0, r.min.y + 98.0),
                        size: 9.5,
                        color: palette.text_muted,
                        strong: false,
                    });
                }
            }
            AssetViewMode::List => {
                let row_h = 28.0;
                for (idx, item) in filtered.iter().enumerate() {
                    let y = content_rect.min.y + 4.0 + (idx as f32 * row_h) - self.scroll_offset;
                    let r = Rect::from_min_size(
                        Vec2::new(content_rect.min.x + 8.0, y),
                        Vec2::new(content_rect.width() - 16.0, row_h - 2.0),
                    );

                    if r.max.y < content_rect.min.y || r.min.y > content_rect.max.y {
                        continue;
                    }

                    let is_sel = self.selected_id.as_deref() == Some(&item.id);
                    let is_hov = r.contains(ptr.position);

                    cmd_buf.push(DrawCommand::Rect {
                        rect: r,
                        fill: if is_sel {
                            palette.accent.with_alpha(45)
                        } else if is_hov {
                            palette.elevated
                        } else {
                            Color::TRANSPARENT
                        },
                        radius: CornerRadius::same(3.0),
                        stroke: if is_sel {
                            Stroke::new(1.0, palette.accent)
                        } else {
                            Stroke::NONE
                        },
                    });

                    cmd_buf.push(DrawCommand::Text {
                        text: item.name.clone(),
                        pos: Vec2::new(r.min.x + 12.0, r.min.y + 6.0),
                        size: 11.5,
                        color: palette.text,
                        strong: is_sel,
                    });

                    cmd_buf.push(DrawCommand::Text {
                        text: item.category.clone(),
                        pos: Vec2::new(r.max.x - 120.0, r.min.y + 6.0),
                        size: 10.5,
                        color: palette.text_muted,
                        strong: false,
                    });
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// 5. CONSOLE & LOG VIEWER
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

#[derive(Debug, Clone)]
pub struct ConsoleLogEntry {
    pub level: ConsoleLogLevel,
    pub tag: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConsoleView {
    pub entries: Vec<ConsoleLogEntry>,
    pub input_buffer: String,
    pub scroll_offset: f32,
    pub auto_scroll: bool,
}

impl ConsoleView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0.0,
            auto_scroll: true,
        }
    }

    pub fn log(
        &mut self,
        level: ConsoleLogLevel,
        tag: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.entries.push(ConsoleLogEntry {
            level,
            tag: tag.into(),
            message: message.into(),
        });
    }

    pub fn push(&mut self, message: impl Into<String>, level: ConsoleLogLevel) {
        self.log(level, "System", message);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn render(&self, cmd_buf: &mut CommandBuffer, bounds: Rect, palette: &ThemePalette) {
        // Dark terminal background
        cmd_buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.canvas,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        let line_h = 20.0;
        let mut y = bounds.min.y + 8.0;

        for entry in self.entries.iter().rev().take(30).rev() {
            let level_color = match entry.level {
                ConsoleLogLevel::Info => palette.text,
                ConsoleLogLevel::Warning => Color::rgb(240, 180, 50),
                ConsoleLogLevel::Error => Color::rgb(240, 70, 70),
                ConsoleLogLevel::Debug => palette.text_muted,
            };

            let level_tag = match entry.level {
                ConsoleLogLevel::Info => "[INFO]",
                ConsoleLogLevel::Warning => "[WARN]",
                ConsoleLogLevel::Error => "[ERR ]",
                ConsoleLogLevel::Debug => "[DBUG]",
            };

            cmd_buf.push(DrawCommand::Text {
                text: format!("{} [{}] {}", level_tag, entry.tag, entry.message),
                pos: Vec2::new(bounds.min.x + 10.0, y),
                size: 11.0,
                color: level_color,
                strong: entry.level == ConsoleLogLevel::Error,
            });

            y += line_h;
            if y > bounds.max.y - 30.0 {
                break;
            }
        }

        // Bottom prompt separator
        let prompt_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.max.y - 28.0),
            Vec2::new(bounds.width(), 28.0),
        );
        cmd_buf.push(DrawCommand::Rect {
            rect: prompt_rect,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        cmd_buf.push(DrawCommand::Text {
            text: format!(
                "> {}_",
                if self.input_buffer.is_empty() {
                    "Type command..."
                } else {
                    &self.input_buffer
                }
            ),
            pos: Vec2::new(prompt_rect.min.x + 10.0, prompt_rect.min.y + 7.0),
            size: 11.0,
            color: if self.input_buffer.is_empty() {
                palette.text_muted
            } else {
                palette.text
            },
            strong: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_bar_and_context_menu() {
        let menu = Menu::new(
            "File",
            vec![
                MenuItem::new("file_new", "New").with_shortcut("Ctrl+N"),
                MenuItem::new("file_open", "Open").with_shortcut("Ctrl+O"),
            ],
        );
        let mut bar = MenuBar::new(vec![menu]);

        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 30.0));
        // Click on "File"
        let _ = bar.handle_mouse_down(Vec2::new(20.0, 15.0), PointerButton::Primary, bounds);
        assert_eq!(bar.open_menu_idx, Some(0));

        // Click on "New"
        let action = bar.handle_mouse_down(Vec2::new(30.0, 45.0), PointerButton::Primary, bounds);
        assert_eq!(action, Some("file_new".into()));
        assert_eq!(bar.open_menu_idx, None);
    }

    #[test]
    fn test_asset_browser_filtering() {
        let mut browser = AssetBrowser::new();
        browser.add_item(AssetItem::new("a1", "Albedo Texture", "Textures"));
        browser.add_item(AssetItem::new("a2", "Normal Map", "Textures"));
        browser.add_item(AssetItem::new("a3", "Hero Character Mesh", "Models"));

        assert_eq!(browser.categories().len(), 3); // All, Textures, Models
        assert_eq!(browser.filtered_items().len(), 3);

        browser.active_category = "Models".into();
        assert_eq!(browser.filtered_items().len(), 1);
        assert_eq!(browser.filtered_items()[0].name, "Hero Character Mesh");
    }

    #[test]
    fn test_notification_host() {
        let mut host = NotificationHost::new();
        host.push(
            "n1",
            "Build Finished",
            "Artifact ready in 0.4s",
            NotificationLevel::Success,
        );
        assert_eq!(host.notifications.len(), 1);

        host.dismiss("n1");
        assert_eq!(host.notifications.len(), 0);
    }
}
