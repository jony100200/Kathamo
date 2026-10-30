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
    /// Gradient surface with a subtle top-edge lighting treatment, rendered in one SDF quad.
    Surface {
        rect: Rect,
        fill_top: Color,
        fill_bottom: Color,
        radius: CornerRadius,
        stroke: Stroke,
        rim_strength: f32,
    },
    /// Analytical feathered shadow. No intermediate shadow bitmap is required.
    SoftShadow {
        rect: Rect,
        radius: CornerRadius,
        color: Color,
        offset: Vec2,
        blur: f32,
        spread: f32,
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
    /// Monospace text for code, logs and aligned numbers.
    Code {
        text: String,
        pos: Vec2,
        size: f32,
        color: Color,
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
    /// Textured quad / image blit (for 2D raster layers, video frames, or 3D viewport FBOs).
    Image {
        rect: Rect,
        texture_id: u64,
        uv_rect: Rect,
        tint: Color,
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

    pub fn insert(&mut self, index: usize, cmd: DrawCommand) {
        if index <= self.commands.len() {
            self.commands.insert(index, cmd);
        } else {
            self.commands.push(cmd);
        }
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

    /// Emits high-contrast text with an infallible subpixel outline stroke (ideal for subtitles and manga captions).
    pub fn push_text_with_outline(
        &mut self,
        text: impl Into<String>,
        pos: Vec2,
        size: f32,
        color: Color,
        outline_color: Color,
        outline_width: f32,
        strong: bool,
    ) {
        let text_str = text.into();
        let w = outline_width.max(0.5);
        let offsets = [
            Vec2::new(-w, 0.0),
            Vec2::new(w, 0.0),
            Vec2::new(0.0, -w),
            Vec2::new(0.0, w),
            Vec2::new(-w * 0.707, -w * 0.707),
            Vec2::new(w * 0.707, -w * 0.707),
            Vec2::new(-w * 0.707, w * 0.707),
            Vec2::new(w * 0.707, w * 0.707),
        ];
        for off in offsets {
            self.push(DrawCommand::Text {
                text: text_str.clone(),
                pos: pos + off,
                size,
                color: outline_color,
                strong,
            });
        }
        self.push(DrawCommand::Text {
            text: text_str,
            pos,
            size,
            color,
            strong,
        });
    }

    /// Emits text with a cinematic soft drop shadow.
    pub fn push_text_with_shadow(
        &mut self,
        text: impl Into<String>,
        pos: Vec2,
        size: f32,
        color: Color,
        shadow_color: Color,
        shadow_offset: Vec2,
        strong: bool,
    ) {
        let text_str = text.into();
        self.push(DrawCommand::Text {
            text: text_str.clone(),
            pos: pos + shadow_offset,
            size,
            color: shadow_color,
            strong,
        });
        self.push(DrawCommand::Text {
            text: text_str,
            pos,
            size,
            color,
            strong,
        });
    }
}
