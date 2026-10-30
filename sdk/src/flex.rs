//! Declarative flexbox for screen layout (CSS flexbox semantics, solved by `taffy`).
//!
//! Describe regions once, solve once per frame, and use the same rectangles for
//! drawing and hit-testing:
//! ```rust,ignore
//! let l = Flex::column()
//!     .child(Flex::fixed(28.0).key("menu"))
//!     .child(Flex::row().grow(1.0)
//!         .child(Flex::fixed(310.0).key("left"))
//!         .child(Flex::grow_by(1.0).key("center"))
//!         .child(Flex::fixed(340.0).key("right")))
//!     .layout(bounds);
//! let center = l.get("center");
//! ```
//! `fixed` sizes along the parent's main axis; the cross axis stretches, as in CSS.

use taffy::prelude::{
    length, AvailableSpace, FlexDirection, NodeId, Size, Style, TaffyTree,
};
use ui_core::{Rect, Vec2};

/// One box in the layout tree.
#[derive(Debug, Clone)]
pub struct Flex {
    style: Style,
    key: Option<&'static str>,
    children: Vec<Flex>,
}

impl Flex {
    fn container(direction: FlexDirection) -> Self {
        Self {
            style: Style {
                flex_direction: direction,
                ..Style::default()
            },
            key: None,
            children: Vec::new(),
        }
    }

    /// Children flow left to right.
    pub fn row() -> Self {
        Self::container(FlexDirection::Row)
    }

    /// Children flow top to bottom.
    pub fn column() -> Self {
        Self::container(FlexDirection::Column)
    }

    /// Exactly `px` along the parent's main axis; never shrinks.
    pub fn fixed(px: f32) -> Self {
        Self::column().basis(px).shrink(0.0)
    }

    /// Takes a `weight` share of the remaining space.
    pub fn grow_by(weight: f32) -> Self {
        Self::column().basis(0.0).grow(weight)
    }

    pub fn key(mut self, key: &'static str) -> Self {
        self.key = Some(key);
        self
    }

    pub fn grow(mut self, weight: f32) -> Self {
        self.style.flex_grow = weight;
        self
    }

    pub fn shrink(mut self, weight: f32) -> Self {
        self.style.flex_shrink = weight;
        self
    }

    pub fn basis(mut self, px: f32) -> Self {
        self.style.flex_basis = length(px);
        self
    }

    /// Smallest size along the main axis, e.g. a centre view that must stay usable.
    pub fn min_main(mut self, px: f32) -> Self {
        self.style.min_size = Size {
            width: length(px),
            height: length(px),
        };
        self
    }

    pub fn gap(mut self, px: f32) -> Self {
        self.style.gap = Size {
            width: length(px),
            height: length(px),
        };
        self
    }

    pub fn padding(mut self, px: f32) -> Self {
        let p = length(px);
        self.style.padding = taffy::Rect {
            left: p,
            right: p,
            top: p,
            bottom: p,
        };
        self
    }

    pub fn child(mut self, child: Flex) -> Self {
        self.children.push(child);
        self
    }

    /// Solves the tree inside `bounds`. Unsolvable input yields an empty layout.
    pub fn layout(self, bounds: Rect) -> FlexLayout {
        let mut out = FlexLayout { rects: Vec::new() };
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let mut keys = Vec::new();
        let Ok(root) = build(&mut tree, self, &mut keys, true, bounds) else {
            return out;
        };
        let space = Size {
            width: AvailableSpace::Definite(bounds.width()),
            height: AvailableSpace::Definite(bounds.height()),
        };
        if tree.compute_layout(root, space).is_err() {
            return out;
        }
        collect(&tree, root, bounds.min, &keys, &mut out.rects);
        out
    }
}

fn build(
    tree: &mut TaffyTree<()>,
    node: Flex,
    keys: &mut Vec<(NodeId, &'static str)>,
    is_root: bool,
    bounds: Rect,
) -> taffy::TaffyResult<NodeId> {
    let mut style = node.style;
    if is_root {
        style.size = Size {
            width: length(bounds.width()),
            height: length(bounds.height()),
        };
    }
    let mut ids = Vec::with_capacity(node.children.len());
    for child in node.children {
        ids.push(build(tree, child, keys, false, bounds)?);
    }
    let id = tree.new_with_children(style, &ids)?;
    if let Some(key) = node.key {
        keys.push((id, key));
    }
    Ok(id)
}

fn collect(
    tree: &TaffyTree<()>,
    id: NodeId,
    parent_origin: Vec2,
    keys: &[(NodeId, &'static str)],
    out: &mut Vec<(&'static str, Rect)>,
) {
    let Ok(l) = tree.layout(id) else {
        return;
    };
    let origin = Vec2::new(parent_origin.x + l.location.x, parent_origin.y + l.location.y);
    if let Some((_, key)) = keys.iter().find(|(n, _)| *n == id) {
        out.push((key, Rect::from_min_size(origin, Vec2::new(l.size.width, l.size.height))));
    }
    if let Ok(children) = tree.children(id) {
        for child in children {
            collect(tree, child, origin, keys, out);
        }
    }
}

/// Solved rectangles by key.
#[derive(Debug, Clone, Default)]
pub struct FlexLayout {
    rects: Vec<(&'static str, Rect)>,
}

impl FlexLayout {
    /// The rectangle for `key`; an empty rectangle if the key was not laid out.
    pub fn get(&self, key: &str) -> Rect {
        self.rects
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(Rect::from_min_size(Vec2::ZERO, Vec2::ZERO), |(_, r)| *r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(bounds: Rect) -> FlexLayout {
        Flex::column()
            .child(Flex::fixed(66.0).key("top"))
            .child(
                Flex::row()
                    .grow(1.0)
                    .key("work")
                    .child(Flex::fixed(310.0).key("left"))
                    .child(Flex::grow_by(1.0).key("center"))
                    .child(Flex::fixed(340.0).key("right")),
            )
            .child(Flex::fixed(24.0).key("status"))
            .layout(bounds)
    }

    #[test]
    fn editor_regions_tile_the_window_exactly() {
        let b = Rect::from_min_size(Vec2::new(10.0, 20.0), Vec2::new(1600.0, 1000.0));
        let l = editor(b);
        assert_eq!(l.get("top").min, b.min);
        assert_eq!(l.get("top").height(), 66.0);
        assert_eq!(l.get("left").width(), 310.0);
        assert_eq!(l.get("right").width(), 340.0);
        assert_eq!(l.get("center").width(), 1600.0 - 310.0 - 340.0);
        assert_eq!(l.get("center").min.x, 10.0 + 310.0);
        assert_eq!(l.get("left").height(), 1000.0 - 66.0 - 24.0);
        assert_eq!(l.get("status").max.y, b.max.y);
    }

    #[test]
    fn gap_padding_and_weights_follow_css_rules() {
        let b = Rect::from_min_size(Vec2::ZERO, Vec2::new(320.0, 100.0));
        let l = Flex::row()
            .padding(10.0)
            .gap(10.0)
            .child(Flex::grow_by(1.0).key("a"))
            .child(Flex::grow_by(2.0).key("b"))
            .layout(b);
        // 320 - 2*10 padding - 10 gap = 290, split 1:2 and rounded to whole pixels.
        assert!((l.get("a").width() - 290.0 / 3.0).abs() <= 1.0);
        assert!((l.get("b").width() - 290.0 * 2.0 / 3.0).abs() <= 1.0);
        assert_eq!(l.get("a").width() + l.get("b").width(), 290.0);
        assert_eq!(l.get("a").min, Vec2::new(10.0, 10.0));
        assert_eq!(l.get("missing").width(), 0.0);
    }
}
