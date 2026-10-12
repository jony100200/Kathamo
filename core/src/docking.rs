//! Multi-panel creative studio docking math.
//! Pure data representation of panels, tabs, splitters, and canvas-protection geometry.

use crate::layout::Breakpoint;
use crate::math::{Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSlot {
    Left,
    Right,
    Bottom,
    Center,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockPanel {
    pub id: String,
    pub title: String,
    pub slot: DockSlot,
    pub visible: bool,
    pub preferred_size: f32,
    pub min_size: f32,
    pub max_size: f32,
}

impl DockPanel {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        slot: DockSlot,
        preferred: f32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            slot,
            visible: true,
            preferred_size: preferred,
            min_size: 160.0,
            max_size: 600.0,
        }
    }

    pub fn safe_size(&self, available: f32) -> f32 {
        if !self.visible || !available.is_finite() || available <= 0.0 {
            return 0.0;
        }
        let min = if self.min_size.is_finite() {
            self.min_size.max(0.0)
        } else {
            0.0
        };
        let max = if self.max_size.is_finite() {
            self.max_size.max(min)
        } else {
            min
        };
        let pref = if self.preferred_size.is_finite() {
            self.preferred_size
        } else {
            min
        };
        pref.clamp(min, max).min(available)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDockLayout {
    pub left_rect: Rect,
    pub right_rect: Rect,
    pub bottom_rect: Rect,
    pub canvas_rect: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockWorkspace {
    pub left: Option<DockPanel>,
    pub right: Option<DockPanel>,
    pub bottom: Option<DockPanel>,
    pub min_canvas_width: f32,
    pub min_canvas_height: f32,
}

impl Default for DockWorkspace {
    fn default() -> Self {
        Self {
            left: Some(DockPanel::new(
                "hierarchy",
                "Hierarchy & Tools",
                DockSlot::Left,
                260.0,
            )),
            right: Some(DockPanel::new(
                "inspector",
                "Inspector & Properties",
                DockSlot::Right,
                320.0,
            )),
            bottom: Some(DockPanel::new(
                "timeline",
                "Timeline & Logs",
                DockSlot::Bottom,
                220.0,
            )),
            min_canvas_width: 380.0,
            min_canvas_height: 240.0,
        }
    }
}

impl DockWorkspace {
    /// Resolves panel bounding boxes against window bounds.
    /// Infallible: protects central canvas from being squished to zero by oversized sidebars.
    pub fn resolve(&self, viewport: Rect) -> ResolvedDockLayout {
        let width = viewport.width().max(0.0);
        let height = viewport.height().max(0.0);
        let min_w = self.min_canvas_width.clamp(0.0, width);
        let min_h = self.min_canvas_height.clamp(0.0, height);
        let narrow = Breakpoint::from_width(width) == Breakpoint::Compact;

        // Left panel
        let left_w = if let Some(ref p) = self.left {
            p.safe_size((width - min_w).max(0.0))
        } else {
            0.0
        };

        // Right panel (collapsed if narrow)
        let right_w = if narrow {
            0.0
        } else if let Some(ref p) = self.right {
            p.safe_size((width - min_w - left_w).max(0.0))
        } else {
            0.0
        };

        // Bottom panel
        let bottom_h = if narrow {
            0.0
        } else if let Some(ref p) = self.bottom {
            p.safe_size((height - min_h).max(0.0))
        } else {
            0.0
        };

        let canvas_w = (width - left_w - right_w).max(0.0);
        let canvas_h = (height - bottom_h).max(0.0);

        let left_rect = Rect::from_min_size(viewport.min, Vec2::new(left_w, canvas_h));
        let canvas_rect = Rect::from_min_size(
            Vec2::new(viewport.min.x + left_w, viewport.min.y),
            Vec2::new(canvas_w, canvas_h),
        );
        let right_rect = Rect::from_min_size(
            Vec2::new(viewport.min.x + left_w + canvas_w, viewport.min.y),
            Vec2::new(right_w, canvas_h),
        );
        let bottom_rect = Rect::from_min_size(
            Vec2::new(viewport.min.x, viewport.min.y + canvas_h),
            Vec2::new(width, bottom_h),
        );

        ResolvedDockLayout {
            left_rect,
            right_rect,
            bottom_rect,
            canvas_rect,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_resolver_protects_canvas_minimum_area() {
        let ws = DockWorkspace::default();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0));
        let layout = ws.resolve(viewport);

        assert!(layout.canvas_rect.width() >= ws.min_canvas_width);
        assert!(layout.canvas_rect.height() >= ws.min_canvas_height);
        assert!(layout.left_rect.width() > 0.0);
        assert!(layout.right_rect.width() > 0.0);
        assert!(layout.bottom_rect.height() > 0.0);
    }

    #[test]
    fn narrow_viewport_automatically_collapses_secondary_panels() {
        let ws = DockWorkspace::default();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 400.0));
        let layout = ws.resolve(viewport);

        assert_eq!(layout.right_rect.width(), 0.0);
        assert_eq!(layout.bottom_rect.height(), 0.0);
        assert!(layout.canvas_rect.width() >= ws.min_canvas_width);
    }
}
