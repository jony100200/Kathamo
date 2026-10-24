//! Generic UI adapters for tool configuration, model selection, task progress, and service status.
//! Pure UI presentation contracts with zero backend or runtime dependencies.

use crate::canvas_host::CanvasTool;
use crate::theme::ThemePalette;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

/// Generic Tool Configuration Adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolConfigAdapter {
    pub active_tool: CanvasTool,
    pub brush_size: f32,
    pub brush_opacity: f32,
    pub brush_color: Color,
}

impl Default for ToolConfigAdapter {
    fn default() -> Self {
        Self {
            active_tool: CanvasTool::Brush,
            brush_size: 16.0,
            brush_opacity: 1.0,
            brush_color: Color::rgb(70, 140, 245),
        }
    }
}

impl ToolConfigAdapter {
    /// Renders a horizontal tool strip (e.g. pinned under top menu bar).
    pub fn render(&self, buf: &mut CommandBuffer, bounds: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        let tools = [
            (CanvasTool::Select, "Select"),
            (CanvasTool::Brush, "Brush"),
            (CanvasTool::Eraser, "Eraser"),
            (CanvasTool::Pan, "Pan"),
            (CanvasTool::ShapeRect, "Rect"),
            (CanvasTool::Eyedropper, "Pick"),
        ];

        let mut x = bounds.min.x + 8.0;
        let btn_h = bounds.height() - 8.0;
        for (tool, label) in tools {
            let is_sel = self.active_tool == tool;
            let btn_w = 64.0;
            let btn_rect = Rect::from_min_size(Vec2::new(x, bounds.min.y + 4.0), Vec2::new(btn_w, btn_h));

            buf.push(DrawCommand::Rect {
                rect: btn_rect,
                fill: if is_sel {
                    palette.accent
                } else {
                    palette.elevated
                },
                radius: CornerRadius::same(4.0),
                stroke: Stroke::NONE,
            });

            buf.push(DrawCommand::Text {
                text: label.into(),
                pos: Vec2::new(btn_rect.min.x + 12.0, btn_rect.min.y + 6.0),
                size: 11.5,
                color: if is_sel { Color::WHITE } else { palette.text },
                strong: is_sel,
            });

            x += btn_w + 6.0;
        }

        // Size readout
        buf.push(DrawCommand::Text {
            text: format!("Size: {:.0}px", self.brush_size),
            pos: Vec2::new(x + 12.0, bounds.min.y + 10.0),
            size: 11.5,
            color: palette.text_muted,
            strong: false,
        });
    }

    pub fn handle_mouse_down(&mut self, pos: Vec2, bounds: Rect) -> Option<CanvasTool> {
        let tools = [
            (CanvasTool::Select, "Select"),
            (CanvasTool::Brush, "Brush"),
            (CanvasTool::Eraser, "Eraser"),
            (CanvasTool::Pan, "Pan"),
            (CanvasTool::ShapeRect, "Rect"),
            (CanvasTool::Eyedropper, "Pick"),
        ];

        let mut x = bounds.min.x + 8.0;
        let btn_h = bounds.height() - 8.0;
        for (tool, _) in tools {
            let btn_w = 64.0;
            let btn_rect = Rect::from_min_size(Vec2::new(x, bounds.min.y + 4.0), Vec2::new(btn_w, btn_h));
            if btn_rect.contains(pos) {
                self.active_tool = tool;
                return Some(tool);
            }
            x += btn_w + 6.0;
        }
        None
    }
}

/// Generic Model Selector Adapter for generative and offline model pipelines.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSelectorAdapter {
    pub selected_checkpoint: usize,
    pub checkpoints: Vec<String>,
    pub selected_quant: usize,
    pub quant_options: Vec<String>,
    pub lora_name: Option<String>,
    pub lora_strength: f32,
}

impl Default for ModelSelectorAdapter {
    fn default() -> Self {
        Self {
            selected_checkpoint: 0,
            checkpoints: vec![
                "Kolpona-Diffusion-XL-v2.5".into(),
                "Studio-Art-Photoreal-v1".into(),
                "Anime-Cel-Shader-v4".into(),
            ],
            selected_quant: 1,
            quant_options: vec!["FP16 (Native)".into(), "Q8_0 (Balanced)".into(), "Q4_K_M (Fast)".into()],
            lora_name: Some("LineArt-Detail-Enhancer".into()),
            lora_strength: 0.85,
        }
    }
}

impl ModelSelectorAdapter {
    pub fn render(&self, buf: &mut CommandBuffer, bounds: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Header
        buf.push(DrawCommand::Text {
            text: "PIPELINE MODEL CONFIG".into(),
            pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 10.0),
            size: 11.5,
            color: palette.accent,
            strong: true,
        });

        // Checkpoint selector
        let ckpt_rect = Rect::from_min_size(Vec2::new(bounds.min.x + 12.0, bounds.min.y + 32.0), Vec2::new(bounds.width() - 24.0, 26.0));
        buf.push(DrawCommand::Rect {
            rect: ckpt_rect,
            fill: palette.elevated,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        let cur_ckpt = self.checkpoints.get(self.selected_checkpoint).map_or("", |s| s.as_str());
        buf.push(DrawCommand::Text {
            text: format!("Model: {} ▾", cur_ckpt),
            pos: Vec2::new(ckpt_rect.min.x + 8.0, ckpt_rect.min.y + 6.0),
            size: 11.5,
            color: palette.text,
            strong: false,
        });

        // Quantization selector
        let quant_rect = Rect::from_min_size(Vec2::new(bounds.min.x + 12.0, bounds.min.y + 64.0), Vec2::new(bounds.width() - 24.0, 26.0));
        buf.push(DrawCommand::Rect {
            rect: quant_rect,
            fill: palette.elevated,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        let cur_quant = self.quant_options.get(self.selected_quant).map_or("", |s| s.as_str());
        buf.push(DrawCommand::Text {
            text: format!("Precision: {} ▾", cur_quant),
            pos: Vec2::new(quant_rect.min.x + 8.0, quant_rect.min.y + 6.0),
            size: 11.5,
            color: palette.text,
            strong: false,
        });
    }

    pub fn handle_mouse_down(&mut self, pos: Vec2, bounds: Rect) -> bool {
        let ckpt_rect = Rect::from_min_size(Vec2::new(bounds.min.x + 12.0, bounds.min.y + 32.0), Vec2::new(bounds.width() - 24.0, 26.0));
        if ckpt_rect.contains(pos) {
            self.selected_checkpoint = (self.selected_checkpoint + 1) % self.checkpoints.len();
            return true;
        }

        let quant_rect = Rect::from_min_size(Vec2::new(bounds.min.x + 12.0, bounds.min.y + 64.0), Vec2::new(bounds.width() - 24.0, 26.0));
        if quant_rect.contains(pos) {
            self.selected_quant = (self.selected_quant + 1) % self.quant_options.len();
            return true;
        }

        false
    }
}

/// Generic Task Progress Adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskProgressAdapter {
    pub title: String,
    pub progress: f32, // 0.0 .. 1.0
    pub status: String,
    pub is_active: bool,
}

impl Default for TaskProgressAdapter {
    fn default() -> Self {
        Self {
            title: "Asset Cache Baking".into(),
            progress: 0.68,
            status: "Generating PBR normal maps (24/32)...".into(),
            is_active: true,
        }
    }
}

impl TaskProgressAdapter {
    pub fn render(&self, buf: &mut CommandBuffer, bounds: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Title and status
        buf.push(DrawCommand::Text {
            text: self.title.clone(),
            pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 8.0),
            size: 12.0,
            color: palette.text,
            strong: true,
        });

        buf.push(DrawCommand::Text {
            text: self.status.clone(),
            pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 26.0),
            size: 11.0,
            color: palette.text_muted,
            strong: false,
        });

        // Progress bar trough
        let bar_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + 12.0, bounds.min.y + 44.0),
            Vec2::new(bounds.width() - 24.0, 8.0),
        );
        buf.push(DrawCommand::Rect {
            rect: bar_rect,
            fill: palette.elevated,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::NONE,
        });

        // Progress bar fill
        let fill_w = bar_rect.width() * self.progress.clamp(0.0, 1.0);
        let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
        buf.push(DrawCommand::Rect {
            rect: fill_rect,
            fill: palette.accent,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::NONE,
        });
    }
}

/// Generic Service Status Adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceStatusAdapter {
    pub name: String,
    pub is_connected: bool,
    pub latency_ms: u32,
}

impl Default for ServiceStatusAdapter {
    fn default() -> Self {
        Self {
            name: "Local Inference Engine".into(),
            is_connected: true,
            latency_ms: 12,
        }
    }
}

impl ServiceStatusAdapter {
    pub fn render(&self, buf: &mut CommandBuffer, bounds: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Status pill indicator
        let dot_rect = Rect::from_min_size(Vec2::new(bounds.min.x + 12.0, bounds.min.y + 12.0), Vec2::new(10.0, 10.0));
        buf.push(DrawCommand::Rect {
            rect: dot_rect,
            fill: if self.is_connected {
                palette.success
            } else {
                palette.danger
            },
            radius: CornerRadius::same(5.0),
            stroke: Stroke::NONE,
        });

        buf.push(DrawCommand::Text {
            text: self.name.clone(),
            pos: Vec2::new(bounds.min.x + 28.0, bounds.min.y + 9.0),
            size: 11.5,
            color: palette.text,
            strong: true,
        });

        let latency_str = if self.is_connected {
            format!("{} ms", self.latency_ms)
        } else {
            "OFFLINE".into()
        };
        buf.push(DrawCommand::Text {
            text: latency_str,
            pos: Vec2::new(bounds.max.x - 60.0, bounds.min.y + 9.0),
            size: 11.0,
            color: if self.is_connected { palette.success } else { palette.danger },
            strong: false,
        });
    }
}
