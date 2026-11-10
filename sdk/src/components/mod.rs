//! Standard reusable creative and application components for Kathamo SDK.

use std::time::{Duration, Instant};

use ui_core::motion;
use ui_core::*;
use ui_platform::{Key, PointerButton, PointerState};
use ui_render::command::DrawCommand;
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

// -----------------------------------------------------------------------------
// 1. MENU SYSTEM & TOOLBAR
// -----------------------------------------------------------------------------

const MENU_TITLE_PX: f32 = 12.0;

/// Item within a dropdown or context menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub separator_after: bool,
    /// `Some(checked)` makes this a check item. `None` (the default) is a plain item.
    pub checked: Option<bool>,
    /// Nested items. The submenu opens on hover, click or ArrowRight and activates nothing itself.
    pub submenu: Option<Vec<MenuItem>>,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            enabled: true,
            separator_after: false,
            checked: None,
            submenu: None,
        }
    }

    /// A check item, shown with a box that is filled when `checked`.
    pub fn check(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self::new(id, label).with_checked(checked)
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Makes this item open a nested dropdown with `items`.
    pub fn with_submenu(mut self, items: Vec<MenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }

    /// True for the divider items made by [`MenuItem::separator`].
    pub fn is_separator(&self) -> bool {
        self.id.is_empty() && self.label.is_empty() && self.submenu.is_none()
    }

    pub fn action(id: impl Into<String>, label: impl Into<String>, shortcut: Option<&str>) -> Self {
        let mut item = Self::new(id, label);
        if let Some(sc) = shortcut {
            item = item.with_shortcut(sc);
        }
        item
    }

    /// A divider line between items. It takes no pointer or keyboard focus.
    pub fn separator() -> Self {
        Self::new("", "")
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
///
/// A menu bar is a row of titles. Clicking a title opens its dropdown. Items with a
/// [`MenuItem::with_submenu`] open a nested dropdown on hover, click or ArrowRight. The keyboard
/// moves the highlight with the arrows, ArrowLeft and Escape close one level, and Enter
/// activates the highlighted item. Feed keys to [`MenuBar::handle_key_down`] while
/// [`MenuBar::is_open`] is true.
#[derive(Debug, Clone, Default)]
pub struct MenuBar {
    pub menus: Vec<Menu>,
    pub open_menu_idx: Option<usize>,
    /// Item indices of the open submenus, outermost first. Each names an item with a submenu in
    /// the level above it.
    pub submenu_path: Vec<usize>,
    /// Keyboard or hover highlight, an index into the deepest open level.
    pub highlight: Option<usize>,
}

/// One open dropdown: its panel rectangle, its items, and each item's (top, height).
struct MenuLevel<'a> {
    rect: Rect,
    items: &'a [MenuItem],
    tops: Vec<(f32, f32)>,
}

/// Where a pointer position lands in the open menus.
enum MenuHit {
    Item { depth: usize, index: usize },
    Panel,
    Nothing,
}

const MENU_ROW_H: f32 = 26.0;
const MENU_SEP_H: f32 = 9.0;
const MENU_PAD: f32 = 4.0;

/// (top, height) of each item in a dropdown whose panel starts at `panel_top`.
fn item_tops(items: &[MenuItem], panel_top: f32) -> Vec<(f32, f32)> {
    let mut y = panel_top + MENU_PAD;
    let mut tops = Vec::with_capacity(items.len());
    for item in items {
        let h = if item.is_separator() {
            MENU_SEP_H
        } else {
            MENU_ROW_H
        };
        tops.push((y, h));
        y += h;
        if item.separator_after && !item.is_separator() {
            y += MENU_SEP_H;
        }
    }
    tops
}

fn panel_height(items: &[MenuItem]) -> f32 {
    let rows: f32 = item_tops(items, 0.0).last().map_or(0.0, |&(y, h)| y + h);
    rows + MENU_PAD
}

fn panel_width(items: &[MenuItem]) -> f32 {
    let widest = items
        .iter()
        .filter(|it| !it.is_separator())
        .map(|it| {
            let shortcut = it
                .shortcut
                .as_deref()
                .map_or(0.0, |sc| ui_render::measure_text(sc, 10.5, false) + 24.0);
            let arrow = if it.submenu.is_some() { 16.0 } else { 0.0 };
            ui_render::measure_text(&it.label, 11.5, false) + shortcut + arrow + 24.0
        })
        .fold(180.0f32, f32::max);
    let check_col = if items.iter().any(|it| it.checked.is_some()) {
        14.0
    } else {
        0.0
    };
    widest + check_col
}

impl MenuBar {
    pub fn new(menus: Vec<Menu>) -> Self {
        Self {
            menus,
            open_menu_idx: None,
            submenu_path: Vec::new(),
            highlight: None,
        }
    }

    /// True while a dropdown is open. Keys go to the menu bar first while it is open.
    pub fn is_open(&self) -> bool {
        self.open_menu_idx.is_some()
    }

    /// Opens the menu at `idx`. Does nothing if there is no such menu.
    pub fn open_menu(&mut self, idx: usize) {
        if idx < self.menus.len() {
            self.open_menu_idx = Some(idx);
            self.submenu_path.clear();
            self.highlight = None;
        }
    }

    /// Closes every open dropdown.
    pub fn close(&mut self) {
        self.open_menu_idx = None;
        self.submenu_path.clear();
        self.highlight = None;
    }

    /// Title hit/draw rectangles, sized from the measured title text.
    /// Rendering and hit-testing share this so they can never disagree.
    pub fn title_rects(&self, bounds: Rect) -> Vec<Rect> {
        let mut x = bounds.min.x + 4.0;
        self.menus
            .iter()
            .map(|menu| {
                let w = ui_render::measure_text(&menu.title, MENU_TITLE_PX, false) + 20.0;
                let r = Rect::from_min_size(
                    Vec2::new(x, bounds.min.y + 2.0),
                    Vec2::new(w, bounds.height() - 4.0),
                );
                x += w + 2.0;
                r
            })
            .collect()
    }

    /// The items of the open level `depth` (0 is the open top-level menu).
    fn items_at(&self, depth: usize) -> Option<&[MenuItem]> {
        let mut items: &[MenuItem] = &self.menus.get(self.open_menu_idx?)?.items;
        for &idx in self.submenu_path.iter().take(depth) {
            items = items.get(idx)?.submenu.as_deref()?;
        }
        Some(items)
    }

    /// Every open dropdown, outermost first, with its geometry.
    fn open_levels(&self, bounds: Rect) -> Vec<MenuLevel<'_>> {
        let Some(top) = self.open_menu_idx else {
            return Vec::new();
        };
        let Some(menu) = self.menus.get(top) else {
            return Vec::new();
        };
        let title = self.title_rects(bounds)[top];
        let mut rect = Rect::from_min_size(
            Vec2::new(title.min.x, bounds.max.y + 2.0),
            Vec2::new(panel_width(&menu.items), panel_height(&menu.items)),
        );
        let mut items: &[MenuItem] = &menu.items;
        let mut levels = Vec::new();
        for depth in 0..=self.submenu_path.len() {
            let tops = item_tops(items, rect.min.y);
            let parent_row = tops.clone();
            levels.push(MenuLevel { rect, items, tops });
            let Some(&idx) = self.submenu_path.get(depth) else {
                break;
            };
            let (Some(sub), Some(&(y, _))) = (
                items.get(idx).and_then(|it| it.submenu.as_deref()),
                parent_row.get(idx),
            ) else {
                break;
            };
            rect = Rect::from_min_size(
                Vec2::new(rect.max.x - 2.0, y - MENU_PAD),
                Vec2::new(panel_width(sub), panel_height(sub)),
            );
            items = sub;
        }
        levels
    }

    /// What is under `pos`: an item (depth, index), a dropdown panel, or nothing.
    fn hit(&self, pos: Vec2, bounds: Rect) -> MenuHit {
        let levels = self.open_levels(bounds);
        for (depth, level) in levels.iter().enumerate().rev() {
            if !level.rect.contains(pos) {
                continue;
            }
            return match level
                .tops
                .iter()
                .position(|&(y, h)| pos.y >= y && pos.y < y + h)
            {
                Some(index) if !level.items[index].is_separator() => MenuHit::Item { depth, index },
                _ => MenuHit::Panel,
            };
        }
        MenuHit::Nothing
    }

    /// Opens the item's submenu, or makes `index` the highlight at `depth`.
    fn select_at(&mut self, depth: usize, index: usize) {
        self.submenu_path.truncate(depth);
        let has_submenu = self
            .items_at(depth)
            .and_then(|items| items.get(index))
            .is_some_and(|it| it.submenu.is_some() && it.enabled);
        if has_submenu {
            self.submenu_path.push(index);
            self.highlight = None;
        } else {
            self.highlight = Some(index);
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
            if let Some(idx) = self
                .title_rects(bounds)
                .iter()
                .position(|r| r.contains(pos))
            {
                if self.open_menu_idx == Some(idx) {
                    self.close();
                } else {
                    self.open_menu(idx);
                }
                return None;
            }
        }

        match self.hit(pos, bounds) {
            MenuHit::Item { depth, index } => {
                let item = self.items_at(depth)?.get(index)?;
                if !item.enabled {
                    return None;
                }
                if item.submenu.is_some() {
                    self.select_at(depth, index);
                    return None;
                }
                let id = item.id.clone();
                self.close();
                Some(id)
            }
            MenuHit::Panel => None,
            MenuHit::Nothing => {
                self.close();
                None
            }
        }
    }

    /// Hover: switches an open menu to the title under the pointer, and highlights or opens the
    /// item under the pointer. Returns true when a dropdown is open.
    pub fn handle_mouse_move(&mut self, pos: Vec2, bounds: Rect) -> bool {
        if !self.is_open() {
            return false;
        }
        if let Some(idx) = self
            .title_rects(bounds)
            .iter()
            .position(|r| r.contains(pos))
        {
            if self.open_menu_idx != Some(idx) {
                self.open_menu(idx);
            }
            return true;
        }
        if let MenuHit::Item { depth, index } = self.hit(pos, bounds) {
            let enabled = self
                .items_at(depth)
                .and_then(|items| items.get(index))
                .is_some_and(|it| it.enabled);
            if enabled {
                self.select_at(depth, index);
            }
        }
        true
    }

    /// Moves the highlight to the next (`step` > 0) or previous selectable item at the deepest
    /// open level. Separators and disabled items are skipped.
    fn step_highlight(&mut self, step: isize) {
        let Some(items) = self.items_at(self.submenu_path.len()) else {
            return;
        };
        let n = items.len() as isize;
        let mut i = match self.highlight {
            Some(h) => h as isize + step,
            None if step > 0 => 0,
            None => n - 1,
        };
        while (0..n).contains(&i) {
            let it = &items[i as usize];
            if !it.is_separator() && it.enabled {
                self.highlight = Some(i as usize);
                return;
            }
            i += step;
        }
    }

    /// Keyboard navigation. Returns the id of an activated item and closes the menus. Call
    /// only while [`MenuBar::is_open`] is true.
    pub fn handle_key_down(&mut self, key: &Key) -> Option<String> {
        let top_count = self.menus.len();
        match key {
            Key::Escape => {
                if self.submenu_path.pop().is_some() {
                    self.highlight = None;
                } else {
                    self.close();
                }
            }
            Key::ArrowDown => self.step_highlight(1),
            Key::ArrowUp => self.step_highlight(-1),
            Key::ArrowRight => {
                let highlighted = self.highlight.and_then(|h| {
                    self.items_at(self.submenu_path.len())
                        .and_then(|items| items.get(h))
                        .map(|it| (h, it.submenu.is_some()))
                });
                match highlighted {
                    Some((h, true)) => {
                        self.submenu_path.push(h);
                        self.highlight = None;
                        self.step_highlight(1);
                    }
                    _ if self.submenu_path.is_empty() && top_count > 0 => {
                        let next = (self.open_menu_idx.unwrap_or(0) + 1) % top_count;
                        self.open_menu(next);
                    }
                    _ => {}
                }
            }
            Key::ArrowLeft => {
                if let Some(parent) = self.submenu_path.pop() {
                    self.highlight = Some(parent);
                } else if top_count > 0 {
                    let prev = (self.open_menu_idx.unwrap_or(0) + top_count - 1) % top_count;
                    self.open_menu(prev);
                }
            }
            Key::Enter => {
                let depth = self.submenu_path.len();
                let h = self.highlight?;
                let item = self.items_at(depth)?.get(h)?;
                if !item.enabled {
                    return None;
                }
                if item.submenu.is_some() {
                    self.submenu_path.push(h);
                    self.highlight = None;
                    self.step_highlight(1);
                    return None;
                }
                let id = item.id.clone();
                self.close();
                return Some(id);
            }
            _ => {}
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

        let text_y = bounds.min.y + (bounds.height() - MENU_TITLE_PX) * 0.5 - 1.0;
        for (idx, (menu, item_rect)) in self.menus.iter().zip(self.title_rects(bounds)).enumerate()
        {
            let is_open = self.open_menu_idx == Some(idx);
            let is_hovered = item_rect.contains(ptr.position);
            let t = motion::animate_bool(
                motion::rect_key("menubar", item_rect),
                is_open || is_hovered,
                motion::HOVER,
            );
            if t > 0.0 {
                let fill = if is_open {
                    palette.elevated_high
                } else {
                    palette.elevated
                };
                cmd_buf.push(DrawCommand::Rect {
                    rect: item_rect,
                    fill: fill.with_alpha((fill.a as f32 * t) as u8),
                    radius: CornerRadius::same(4.0),
                    stroke: Stroke::NONE,
                });
            }

            cmd_buf.push(DrawCommand::Text {
                text: menu.title.clone(),
                pos: Vec2::new(item_rect.min.x + 10.0, text_y),
                size: MENU_TITLE_PX,
                color: palette.text_muted.lerp(palette.text, t.max(0.6)),
                strong: false,
            });
        }

        // Open dropdowns, outermost first so each submenu paints over its parent.
        let deepest = self.submenu_path.len();
        for (depth, level) in self.open_levels(bounds).iter().enumerate() {
            self.render_level(cmd_buf, level, depth, deepest, ptr, palette);
        }
    }

    fn render_level(
        &self,
        cmd_buf: &mut CommandBuffer,
        level: &MenuLevel<'_>,
        depth: usize,
        deepest: usize,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        let panel = level.rect;
        cmd_buf.push(DrawCommand::SoftShadow {
            rect: panel,
            radius: CornerRadius::same(5.0),
            color: Color::rgba(0, 0, 0, 110),
            offset: Vec2::new(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
        });
        // Frosted glass: blurred backdrop + translucent tint, then the hairline border.
        cmd_buf.push(DrawCommand::Backdrop {
            rect: panel,
            radius: CornerRadius::same(5.0),
            tint: palette.elevated.with_alpha(200),
        });
        cmd_buf.push(DrawCommand::Rect {
            rect: panel,
            fill: Color::TRANSPARENT,
            radius: CornerRadius::same(5.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        let has_checks = level.items.iter().any(|it| it.checked.is_some());
        let check_col = if has_checks { 14.0 } else { 0.0 };
        let parent_row = self.submenu_path.get(depth).copied();
        for (index, (item, &(top, height))) in level.items.iter().zip(&level.tops).enumerate() {
            if item.is_separator() {
                cmd_buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(panel.min.x + 8.0, top + height * 0.5),
                        Vec2::new(panel.width() - 16.0, 1.0),
                    ),
                    fill: palette.border.with_alpha(160),
                    radius: CornerRadius::ZERO,
                    stroke: Stroke::NONE,
                });
                continue;
            }

            let r = Rect::from_min_size(
                Vec2::new(panel.min.x + 4.0, top),
                Vec2::new(panel.width() - 8.0, MENU_ROW_H),
            );
            let highlighted =
                (depth == deepest && self.highlight == Some(index)) || parent_row == Some(index);
            let is_hov = r.contains(ptr.position) && item.enabled;
            if highlighted || is_hov {
                cmd_buf.push(DrawCommand::Rect {
                    rect: r,
                    fill: palette.elevated_high,
                    radius: CornerRadius::same(3.0),
                    stroke: Stroke::NONE,
                });
            }

            if let Some(checked) = item.checked {
                cmd_buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(r.min.x + 6.0, r.center().y - 4.0),
                        Vec2::new(8.0, 8.0),
                    ),
                    fill: if checked {
                        palette.accent
                    } else {
                        Color::TRANSPARENT
                    },
                    radius: CornerRadius::same(2.0),
                    stroke: Stroke::new(1.0, palette.text_muted),
                });
            }

            cmd_buf.push(DrawCommand::Text {
                text: item.label.clone(),
                pos: Vec2::new(r.min.x + 8.0 + check_col, r.min.y + 5.0),
                size: 11.5,
                color: if item.enabled {
                    palette.text
                } else {
                    palette.text_muted.with_alpha(120)
                },
                strong: false,
            });

            if let Some(sc) = &item.shortcut {
                let sc_w = ui_render::measure_text(sc, 10.5, false);
                cmd_buf.push(DrawCommand::Text {
                    text: sc.clone(),
                    pos: Vec2::new(r.max.x - 10.0 - sc_w, r.min.y + 6.0),
                    size: 10.5,
                    color: palette.text_muted,
                    strong: false,
                });
            }

            if item.submenu.is_some() {
                cmd_buf.push(DrawCommand::Text {
                    text: ">".to_string(),
                    pos: Vec2::new(r.max.x - 12.0, r.min.y + 5.0),
                    size: 11.5,
                    color: palette.text_muted,
                    strong: false,
                });
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
        cmd_buf.push(DrawCommand::SoftShadow {
            rect: b,
            radius: CornerRadius::same(5.0),
            color: Color::rgba(0, 0, 0, 110),
            offset: Vec2::new(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
        });
        // Frosted glass: blurred backdrop + translucent tint, then the hairline border.
        cmd_buf.push(DrawCommand::Backdrop {
            rect: b,
            radius: CornerRadius::same(5.0),
            tint: palette.elevated.with_alpha(200),
        });
        cmd_buf.push(DrawCommand::Rect {
            rect: b,
            fill: Color::TRANSPARENT,
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
    pub created: Instant,
}

/// How long a toast stays on screen; the last part fades out.
const TOAST_LIFETIME: Duration = Duration::from_secs(5);
const TOAST_FADE_OUT: f32 = 0.3;

/// Toast stack, bottom-right like modern web apps, so toolbars stay uncovered.
/// Lifetime is wall-clock time, so toasts expire even while the user is idle.
#[derive(Debug, Clone, Default)]
pub struct NotificationHost {
    pub notifications: Vec<NotificationToast>,
    next_id: u64,
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
            created: Instant::now(),
        });
    }

    fn push_auto(&mut self, title: String, message: String, level: NotificationLevel) {
        self.next_id += 1;
        let id = format!("toast_{}", self.next_id);
        self.push(id, title, message, level);
    }

    pub fn info(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push_auto(title.into(), message.into(), NotificationLevel::Info);
    }

    pub fn success(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push_auto(title.into(), message.into(), NotificationLevel::Success);
    }

    pub fn warning(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push_auto(title.into(), message.into(), NotificationLevel::Warning);
    }

    pub fn error(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push_auto(title.into(), message.into(), NotificationLevel::Error);
    }

    pub fn dismiss(&mut self, id: &str) {
        self.notifications.retain(|n| n.id != id);
    }

    /// Drops expired toasts. Safe to call every frame.
    pub fn tick(&mut self) {
        self.notifications
            .retain(|n| n.created.elapsed() < TOAST_LIFETIME);
    }

    pub fn render(&self, cmd_buf: &mut CommandBuffer, screen_bounds: Rect, palette: &ThemePalette) {
        if self.notifications.is_empty() {
            return;
        }

        let toast_w = 320.0;
        let toast_h = 58.0;
        // Clear a typical 24 px status bar at the bottom edge.
        let mut y = screen_bounds.max.y - 40.0 - toast_h;

        for n in self.notifications.iter().rev().take(4) {
            let key = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                ("toast", &n.id).hash(&mut h);
                h.finish()
            };
            let enter = motion::animate_from(key, 0.0, 1.0, motion::SLIDE);
            let remaining = TOAST_LIFETIME.as_secs_f32() - n.created.elapsed().as_secs_f32();
            let alpha = enter.min((remaining / TOAST_FADE_OUT).clamp(0.0, 1.0));
            if remaining <= TOAST_FADE_OUT {
                motion::request_frame();
            } else {
                // Sleep until the fade-out begins.
                motion::wake_at(
                    n.created + TOAST_LIFETIME - Duration::from_secs_f32(TOAST_FADE_OUT),
                );
            }
            let fade = |c: Color| c.with_alpha((c.a as f32 * alpha) as u8);

            let toast_rect = Rect::from_min_size(
                Vec2::new(
                    screen_bounds.max.x - toast_w - 16.0 + (1.0 - enter) * 24.0,
                    y,
                ),
                Vec2::new(toast_w, toast_h),
            );

            cmd_buf.push(DrawCommand::SoftShadow {
                rect: toast_rect,
                radius: CornerRadius::same(8.0),
                color: fade(Color::rgba(0, 0, 0, 120)),
                offset: Vec2::new(0.0, 8.0),
                blur: 24.0,
                spread: 0.0,
            });
            // Frosted glass while fully shown; the blur has no opacity, so enter and
            // fade-out use the fading solid card instead of popping.
            let glass = alpha >= 0.999;
            if glass {
                cmd_buf.push(DrawCommand::Backdrop {
                    rect: toast_rect,
                    radius: CornerRadius::same(8.0),
                    tint: palette.elevated.with_alpha(200),
                });
            }
            cmd_buf.push(DrawCommand::Rect {
                rect: toast_rect,
                fill: if glass {
                    Color::TRANSPARENT
                } else {
                    fade(palette.elevated)
                },
                radius: CornerRadius::same(8.0),
                stroke: Stroke::new(1.0, fade(palette.border)),
            });

            let indicator_color = match n.level {
                NotificationLevel::Info => palette.accent,
                NotificationLevel::Success => palette.success,
                NotificationLevel::Warning => palette.warning,
                NotificationLevel::Error => palette.danger,
            };
            // Status dot instead of a hard edge bar: reads as a modern, quiet signal.
            cmd_buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(
                    Vec2::new(toast_rect.min.x + 14.0, toast_rect.min.y + 15.0),
                    Vec2::new(8.0, 8.0),
                ),
                fill: fade(indicator_color),
                radius: CornerRadius::same(4.0),
                stroke: Stroke::NONE,
            });

            let text_x = toast_rect.min.x + 30.0;
            let text_w = toast_rect.max.x - 14.0 - text_x;
            cmd_buf.push(DrawCommand::Text {
                text: ui_render::fit_text(&n.title, 12.5, true, text_w),
                pos: Vec2::new(text_x, toast_rect.min.y + 10.0),
                size: 12.5,
                color: fade(palette.text),
                strong: true,
            });
            cmd_buf.push(DrawCommand::Text {
                text: ui_render::fit_text(&n.message, 11.5, false, text_w),
                pos: Vec2::new(text_x, toast_rect.min.y + 31.0),
                size: 11.5,
                color: fade(palette.text_muted),
                strong: false,
            });

            y -= toast_h + 8.0;
        }
    }
}

/// File-type icon for an asset without a rendered preview.
fn asset_icon(name: &str) -> ui_widgets::StudioIcon {
    use ui_widgets::StudioIcon;
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "exr" | "tga" => StudioIcon::Image,
        "cs" | "rs" | "lua" | "py" | "js" => StudioIcon::Code,
        "graph" => StudioIcon::GitBranch,
        "scene" | "level" => StudioIcon::LayoutGrid,
        "fbx" | "obj" | "gltf" | "glb" | "blend" => StudioIcon::Layers,
        "mat" | "ksmat" => StudioIcon::Palette,
        "wav" | "ogg" | "mp3" => StudioIcon::Waveform,
        _ => StudioIcon::File,
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
                    let hover = motion::animate_bool(
                        motion::rect_key("asset_card", r),
                        r.contains(ptr.position),
                        motion::HOVER,
                    );

                    // Card body
                    cmd_buf.push(DrawCommand::Rect {
                        rect: r,
                        fill: if is_sel {
                            palette.accent.with_alpha(40)
                        } else {
                            palette.surface.lerp(palette.elevated, hover)
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
                        // No preview yet: show the file type, never a fake picture.
                        let icon = asset_icon(&item.name);
                        let c = thumb_r.center();
                        cmd_buf.push(DrawCommand::Icon {
                            id: icon.name(),
                            svg: icon.svg_str(),
                            rect: Rect::from_min_size(
                                Vec2::new(c.x - 14.0, c.y - 14.0),
                                Vec2::new(28.0, 28.0),
                            ),
                            color: palette.text_muted.with_alpha(150),
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

#[cfg(test)]
mod menu_bar_tests {
    use super::*;

    const BOUNDS: Rect = Rect {
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(800.0, 30.0),
    };

    fn sample_bar() -> MenuBar {
        MenuBar::new(vec![
            Menu::new(
                "File",
                vec![
                    MenuItem::new("file_new", "New").with_shortcut("Ctrl+N"),
                    MenuItem::separator(),
                    MenuItem::new("file_recent", "Recent").with_submenu(vec![
                        MenuItem::new("recent_a", "project_a.kolpona"),
                        MenuItem::new("recent_b", "project_b.kolpona").disabled(),
                        MenuItem::separator(),
                        MenuItem::new("recent_c", "project_c.kolpona"),
                    ]),
                    MenuItem::check("view_grid", "Show grid", true).with_separator(),
                    MenuItem::new("file_close", "Close").disabled(),
                ],
            ),
            Menu::new("Edit", vec![MenuItem::new("edit_undo", "Undo")]),
        ])
    }

    /// Centre of item `index` in open level `depth`, from the same geometry the bar draws.
    fn centre(bar: &MenuBar, depth: usize, index: usize) -> Vec2 {
        let levels = bar.open_levels(BOUNDS);
        let level = &levels[depth];
        let (top, height) = level.tops[index];
        Vec2::new(level.rect.min.x + 30.0, top + height * 0.5)
    }

    #[test]
    fn separators_are_dividers_and_never_activate() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        let levels = bar.open_levels(BOUNDS);
        let (top, height) = levels[0].tops[1];
        assert_eq!(height, MENU_SEP_H, "a divider is a short row");
        let on_divider = Vec2::new(levels[0].rect.min.x + 30.0, top + 2.0);
        assert_eq!(
            bar.handle_mouse_down(on_divider, PointerButton::Primary, BOUNDS),
            None
        );
        assert!(bar.is_open(), "clicking a divider keeps the menu open");
    }

    #[test]
    fn check_item_emits_its_id_and_keeps_its_state() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        assert_eq!(bar.menus[0].items[3].checked, Some(true));
        let p = centre(&bar, 0, 3);
        assert_eq!(
            bar.handle_mouse_down(p, PointerButton::Primary, BOUNDS),
            Some("view_grid".into())
        );
    }

    #[test]
    fn disabled_items_do_nothing_on_click() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        let p = centre(&bar, 0, 4);
        assert_eq!(
            bar.handle_mouse_down(p, PointerButton::Primary, BOUNDS),
            None
        );
        assert!(bar.is_open());
    }

    #[test]
    fn hover_opens_a_submenu_and_a_click_inside_it_activates() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        assert!(bar.handle_mouse_move(centre(&bar, 0, 2), BOUNDS));
        assert_eq!(
            bar.submenu_path,
            vec![2],
            "hovering Recent opens its submenu"
        );
        assert_eq!(bar.open_levels(BOUNDS).len(), 2);

        // The submenu sits to the right of its parent and does not overlap it.
        let levels = bar.open_levels(BOUNDS);
        assert!(levels[1].rect.min.x >= levels[0].rect.max.x - 2.0);

        // Hovering the disabled entry leaves the highlight alone.
        bar.handle_mouse_move(centre(&bar, 1, 1), BOUNDS);
        assert_eq!(bar.highlight, None);
        assert_eq!(
            bar.handle_mouse_down(centre(&bar, 1, 0), PointerButton::Primary, BOUNDS),
            Some("recent_a".into())
        );
        assert!(!bar.is_open(), "activating an item closes every level");
        assert!(bar.submenu_path.is_empty());
    }

    #[test]
    fn click_on_a_submenu_parent_opens_it_without_activating() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        assert_eq!(
            bar.handle_mouse_down(centre(&bar, 0, 2), PointerButton::Primary, BOUNDS),
            None
        );
        assert_eq!(bar.submenu_path, vec![2]);
        assert!(bar.is_open());
    }

    #[test]
    fn keyboard_walks_the_menu_skipping_dividers_and_disabled_items() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        bar.handle_key_down(&Key::ArrowDown);
        assert_eq!(bar.highlight, Some(0), "first selectable item");
        bar.handle_key_down(&Key::ArrowDown);
        assert_eq!(bar.highlight, Some(2), "skips the divider at index 1");
        bar.handle_key_down(&Key::ArrowDown);
        assert_eq!(bar.highlight, Some(3), "the check item follows Recent");
        bar.handle_key_down(&Key::ArrowDown);
        assert_eq!(
            bar.highlight,
            Some(3),
            "the disabled Close is skipped at the end"
        );

        bar.handle_key_down(&Key::ArrowUp);
        assert_eq!(bar.highlight, Some(2));
        // ArrowRight opens the highlighted submenu and highlights its first selectable item.
        assert_eq!(bar.handle_key_down(&Key::ArrowRight), None);
        assert_eq!(bar.submenu_path, vec![2]);
        assert_eq!(bar.highlight, Some(0));
        bar.handle_key_down(&Key::ArrowDown);
        assert_eq!(
            bar.highlight,
            Some(3),
            "skips the disabled entry and the divider"
        );
        assert_eq!(
            bar.handle_key_down(&Key::Enter),
            Some("recent_c".into()),
            "Enter activates the highlighted submenu item"
        );
        assert!(!bar.is_open());
    }

    #[test]
    fn escape_and_arrow_left_close_one_level_at_a_time() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        bar.handle_key_down(&Key::ArrowDown);
        bar.handle_key_down(&Key::ArrowDown);
        bar.handle_key_down(&Key::ArrowRight);
        assert_eq!(bar.submenu_path, vec![2]);

        bar.handle_key_down(&Key::Escape);
        assert!(
            bar.submenu_path.is_empty(),
            "Escape closes the submenu first"
        );
        assert!(bar.is_open());
        assert_eq!(bar.highlight, None);

        bar.handle_key_down(&Key::ArrowDown);
        bar.handle_key_down(&Key::ArrowDown);
        bar.handle_key_down(&Key::ArrowRight);
        assert_eq!(bar.submenu_path, vec![2]);
        bar.handle_key_down(&Key::ArrowLeft);
        assert!(bar.submenu_path.is_empty(), "ArrowLeft closes the submenu");
        assert_eq!(
            bar.highlight,
            Some(2),
            "and returns the highlight to its parent"
        );

        bar.handle_key_down(&Key::Escape);
        assert!(!bar.is_open(), "Escape with no submenu closes the menu");
    }

    #[test]
    fn arrows_at_the_top_level_switch_menus() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        bar.handle_key_down(&Key::ArrowRight);
        assert_eq!(bar.open_menu_idx, Some(1));
        bar.handle_key_down(&Key::ArrowLeft);
        assert_eq!(bar.open_menu_idx, Some(0));
    }

    #[test]
    fn hovering_another_title_while_open_switches_menus() {
        let mut bar = sample_bar();
        bar.open_menu(0);
        let edit_title = bar.title_rects(BOUNDS)[1].center();
        bar.handle_mouse_move(edit_title, BOUNDS);
        assert_eq!(bar.open_menu_idx, Some(1));
    }

    #[test]
    fn title_click_toggles_and_outside_click_closes() {
        let mut bar = sample_bar();
        let title = bar.title_rects(BOUNDS)[0].center();
        bar.handle_mouse_down(title, PointerButton::Primary, BOUNDS);
        assert_eq!(bar.open_menu_idx, Some(0));
        bar.handle_mouse_down(title, PointerButton::Primary, BOUNDS);
        assert_eq!(bar.open_menu_idx, None, "clicking an open title closes it");
        bar.open_menu(0);
        bar.handle_mouse_down(Vec2::new(700.0, 500.0), PointerButton::Primary, BOUNDS);
        assert!(!bar.is_open());
    }
}
