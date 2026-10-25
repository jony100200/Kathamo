//! Layout solver primitives: directional stacking, flex sizing, and responsive breakpoints.

use crate::math::{Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Breakpoint {
    Compact, // < 900px
    #[default]
    Standard, // 900px - 1440px
    Wide,    // > 1440px
}

impl Breakpoint {
    pub fn from_width(width: f32) -> Self {
        if width < 900.0 {
            Self::Compact
        } else if width <= 1440.0 {
            Self::Standard
        } else {
            Self::Wide
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SizeConstraint {
    #[default]
    Auto,
    Fixed(f32),
    Flex(f32),
    Clamped {
        min: f32,
        preferred: f32,
        max: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutItem {
    pub size: SizeConstraint,
    pub cross_align: Alignment,
}

/// Infallible linear box layout solver.
pub fn resolve_stack_layout(
    bounds: Rect,
    direction: Direction,
    gap: f32,
    items: &[SizeConstraint],
) -> Vec<Rect> {
    if items.is_empty() {
        return Vec::new();
    }

    let total_main = match direction {
        Direction::Horizontal => bounds.width(),
        Direction::Vertical => bounds.height(),
    };
    let cross_size = match direction {
        Direction::Horizontal => bounds.height(),
        Direction::Vertical => bounds.width(),
    };

    let gap_total = gap * (items.len().saturating_sub(1) as f32);
    let mut fixed_sum = 0.0_f32;
    let mut flex_weight_sum = 0.0_f32;

    for item in items {
        match *item {
            SizeConstraint::Fixed(s) => fixed_sum += s.max(0.0),
            SizeConstraint::Clamped { preferred, .. } => fixed_sum += preferred.max(0.0),
            SizeConstraint::Flex(w) => flex_weight_sum += w.max(0.0),
            SizeConstraint::Auto => fixed_sum += 32.0, // sensible default
        }
    }

    let remaining_main = (total_main - fixed_sum - gap_total).max(0.0);
    let mut current_offset = 0.0_f32;
    let mut results = Vec::with_capacity(items.len());

    for item in items {
        let item_main_size = match *item {
            SizeConstraint::Fixed(s) => s.max(0.0),
            SizeConstraint::Clamped {
                min,
                preferred,
                max,
            } => preferred.clamp(min.max(0.0), max.max(min)),
            SizeConstraint::Flex(w) => {
                if flex_weight_sum > 0.0 {
                    (remaining_main * (w.max(0.0) / flex_weight_sum)).max(0.0)
                } else {
                    0.0
                }
            }
            SizeConstraint::Auto => 32.0,
        };

        let rect = match direction {
            Direction::Horizontal => Rect::from_min_size(
                Vec2::new(bounds.min.x + current_offset, bounds.min.y),
                Vec2::new(item_main_size, cross_size),
            ),
            Direction::Vertical => Rect::from_min_size(
                Vec2::new(bounds.min.x, bounds.min.y + current_offset),
                Vec2::new(cross_size, item_main_size),
            ),
        };

        results.push(rect);
        current_offset += item_main_size + gap;
    }

    results
}

/// Grid geometry independent of the renderer. Rows accept the same constraints as
/// stacked layouts, and incomplete final rows remain aligned with earlier rows.
pub fn resolve_grid_layout(
    bounds: Rect,
    columns: usize,
    column_gap: f32,
    row_gap: f32,
    row_sizes: &[SizeConstraint],
    item_count: usize,
) -> Vec<Rect> {
    if item_count == 0 {
        return Vec::new();
    }
    let cols = columns.max(1).min(item_count);
    let rows = (item_count + cols - 1) / cols;
    let gx = if column_gap.is_finite() {
        column_gap.max(0.0)
    } else {
        0.0
    };
    let gy = if row_gap.is_finite() {
        row_gap.max(0.0)
    } else {
        0.0
    };
    let width = ((bounds.width().max(0.0) - gx * (cols - 1) as f32) / cols as f32).max(0.0);
    let mut sizes = Vec::with_capacity(rows);
    for i in 0..rows {
        sizes.push(
            row_sizes
                .get(i)
                .copied()
                .unwrap_or(SizeConstraint::Flex(1.0)),
        );
    }
    let rows = resolve_stack_layout(bounds, Direction::Vertical, gy, &sizes);
    let mut cells = Vec::with_capacity(item_count);
    for i in 0..item_count {
        let col = i % cols;
        let row = i / cols;
        cells.push(Rect::from_min_size(
            Vec2::new(bounds.min.x + col as f32 * (width + gx), rows[row].min.y),
            Vec2::new(width, rows[row].height()),
        ));
    }
    cells
}

/// Wrap items into as many visual rows as needed without silently squeezing controls.
/// Suitable for resizable toolbars, command palettes, or adaptive form sections.
pub fn resolve_wrap_layout(
    bounds: Rect,
    sizes: &[Vec2],
    column_gap: f32,
    row_gap: f32,
) -> Vec<Rect> {
    let gap_x = if column_gap.is_finite() {
        column_gap.max(0.0)
    } else {
        0.0
    };
    let gap_y = if row_gap.is_finite() {
        row_gap.max(0.0)
    } else {
        0.0
    };
    let mut results = Vec::with_capacity(sizes.len());
    let mut x = bounds.min.x;
    let mut y = bounds.min.y;
    let mut row_height = 0.0_f32;
    for size in sizes {
        let width = if size.x.is_finite() {
            size.x.max(0.0)
        } else {
            0.0
        };
        let height = if size.y.is_finite() {
            size.y.max(0.0)
        } else {
            0.0
        };
        if x > bounds.min.x && x + width > bounds.max.x {
            x = bounds.min.x;
            y += row_height + gap_y;
            row_height = 0.0;
        }
        results.push(Rect::from_min_size(
            Vec2::new(x, y),
            Vec2::new(width.min(bounds.width().max(0.0)), height),
        ));
        x += width + gap_x;
        row_height = row_height.max(height);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_layout_respects_constraints_and_gaps() {
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 100.0));
        let items = [
            SizeConstraint::Fixed(50.0),
            SizeConstraint::Flex(1.0),
            SizeConstraint::Fixed(50.0),
        ];
        let rects = resolve_stack_layout(bounds, Direction::Horizontal, 10.0, &items);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].width(), 50.0);
        // remaining = 300 - 50 - 50 - (2 * 10) = 180
        assert_eq!(rects[1].width(), 180.0);
        assert_eq!(rects[2].width(), 50.0);
    }
    #[test]
    fn grid_and_wrap_respect_resizing() {
        let b = Rect::from_min_size(Vec2::ZERO, Vec2::new(320.0, 200.0));
        let cells = resolve_grid_layout(
            b,
            2,
            10.0,
            10.0,
            &[SizeConstraint::Flex(1.0), SizeConstraint::Flex(1.0)],
            3,
        );
        assert_eq!(cells.len(), 3);
        assert!((cells[0].width() - 155.0).abs() < 0.01);
        assert_eq!(cells[2].min.x, 0.0);
        let wrapped = resolve_wrap_layout(
            b,
            &[Vec2::new(190.0, 40.0), Vec2::new(190.0, 40.0)],
            8.0,
            5.0,
        );
        assert_eq!(wrapped[1].min.y, 45.0);
    }
}
