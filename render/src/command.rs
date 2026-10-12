//! Drawing commands, vertex structures, and render batching primitives.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};

#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    Rect {
        rect: Rect,
        fill: Color,
        radius: CornerRadius,
        stroke: Stroke,
    },
    Line {
        start: Vec2,
        end: Vec2,
        stroke: Stroke,
    },
    CubicBezier {
        start: Vec2,
        ctrl1: Vec2,
        ctrl2: Vec2,
        end: Vec2,
        stroke: Stroke,
    },
    Text {
        text: String,
        pos: Vec2,
        size: f32,
        color: Color,
        strong: bool,
    },
    ClipPush {
        rect: Rect,
    },
    ClipPop,
    /// Monochrome SVG icon (e.g. Tabler) rasterized by the backend and tinted with `color`.
    Icon {
        id: &'static str,
        svg: &'static str,
        rect: Rect,
        color: Color,
    },
}

#[derive(Debug, Clone, Default)]
pub struct CommandBuffer {
    commands: Vec<DrawCommand>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn push(&mut self, cmd: DrawCommand) {
        self.commands.push(cmd);
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
