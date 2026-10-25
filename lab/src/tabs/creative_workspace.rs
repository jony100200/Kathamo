//! Creative Workstation Studio Demonstration Tab.
//! Showcases the complete P0 and P1 professional UI/UX foundation:
//! - Presentation modes: Create / Split / Graph
//! - DockTree with interactive splitters, tabs, maximize/restore, and drop guides
//! - CanvasHost with 2D pan/zoom, checkerboard, adaptive grid, and stroke drawing
//! - HierarchyTree with layer visibility, lock, reorder, inline rename, and selection
//! - Metadata-driven PropertyInspector with numeric scrubbers and typed parameters
//! - NodeEditor with typed ports, bezier splines, DAG cycle check, and compact nodes
//! - Generic adapters for tool config, model selection, task progress, and service health

use crate::app::UiLabApp;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_platform::PointerButton;
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::canvas_host::CanvasTool;
use ui_widgets::workspace_host::PresentationMode;

pub fn render_creative_workspace_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    // 1. Studio Top Control Bar (Toolbar + Presentation Switcher + Model Adapter)
    let top_bar_h = 44.0;
    let top_bar_rect =
        Rect::from_min_size(content_rect.min, Vec2::new(content_rect.width(), top_bar_h));

    cmd_buf.push(DrawCommand::Rect {
        rect: top_bar_rect,
        fill: p.surface,
        radius: CornerRadius::ZERO,
        stroke: Stroke::new(1.0, p.border),
    });

    let mut cur_x = top_bar_rect.min.x + 12.0;
    let btn_y = top_bar_rect.min.y + 7.0;

    // A. Presentation Mode Selector Pills: [Create] [Split] [Graph]
    let modes = [
        (PresentationMode::Create, "Create"),
        (PresentationMode::Split, "Split"),
        (PresentationMode::Graph, "Graph"),
    ];

    for (mode, label) in modes {
        let is_active = app.creative_workspace.document.mode == mode;
        let mode_rect = Rect::from_min_size(Vec2::new(cur_x, btn_y), Vec2::new(68.0, 28.0));
        let is_hover = mode_rect.contains(app.pointer.position);

        let fill = if is_active {
            p.accent
        } else if is_hover {
            p.elevated
        } else {
            p.surface
        };

        cmd_buf.push(DrawCommand::Rect {
            rect: mode_rect,
            fill,
            radius: CornerRadius::same(5.0),
            stroke: Stroke::new(1.0, if is_active { p.accent } else { p.border }),
        });

        cmd_buf.push(DrawCommand::Text {
            text: label.into(),
            pos: Vec2::new(mode_rect.min.x + 14.0, mode_rect.min.y + 7.0),
            size: 11.5,
            color: if is_active { p.canvas } else { p.text },
            strong: is_active,
        });

        cur_x += 74.0;
    }

    cur_x += 16.0;

    // Divider line
    cmd_buf.push(DrawCommand::Line {
        start: Vec2::new(cur_x, top_bar_rect.min.y + 8.0),
        end: Vec2::new(cur_x, top_bar_rect.max.y - 8.0),
        stroke: Stroke::new(1.0, p.border),
    });
    cur_x += 16.0;

    // B. Canvas Tools: Select, Pan, Brush, Eraser
    let tools = [
        (CanvasTool::Select, "Select"),
        (CanvasTool::Pan, "Pan"),
        (CanvasTool::Brush, "Brush"),
        (CanvasTool::Eraser, "Eraser"),
    ];

    for (tool, label) in tools {
        let is_active = app.creative_workspace.document.canvas.active_tool == tool;
        let tool_rect = Rect::from_min_size(Vec2::new(cur_x, btn_y), Vec2::new(60.0, 28.0));
        let is_hover = tool_rect.contains(app.pointer.position);

        cmd_buf.push(DrawCommand::Rect {
            rect: tool_rect,
            fill: if is_active {
                p.accent.with_alpha(180)
            } else if is_hover {
                p.elevated
            } else {
                p.surface
            },
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, if is_active { p.accent } else { p.border }),
        });

        cmd_buf.push(DrawCommand::Text {
            text: label.into(),
            pos: Vec2::new(tool_rect.min.x + 12.0, tool_rect.min.y + 7.0),
            size: 11.0,
            color: if is_active { Color::WHITE } else { p.text },
            strong: is_active,
        });

        cur_x += 66.0;
    }

    cur_x += 12.0;

    // Brush Size Scrubber readout
    let brush_size = app.creative_workspace.document.canvas.brush_size;
    let size_rect = Rect::from_min_size(Vec2::new(cur_x, btn_y), Vec2::new(82.0, 28.0));
    cmd_buf.push(DrawCommand::Rect {
        rect: size_rect,
        fill: p.elevated,
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, p.border),
    });
    cmd_buf.push(DrawCommand::Text {
        text: format!("Size: {:.0}px", brush_size),
        pos: Vec2::new(size_rect.min.x + 8.0, size_rect.min.y + 7.0),
        size: 11.0,
        color: p.text,
        strong: false,
    });

    // C. Model Selector Adapter pill (right-aligned area)
    let model_rect = Rect::from_min_size(
        Vec2::new(top_bar_rect.max.x - 420.0, btn_y),
        Vec2::new(220.0, 28.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: model_rect,
        fill: p.elevated,
        radius: CornerRadius::same(5.0),
        stroke: Stroke::new(1.0, p.border),
    });
    let cur_model = app.model_adapter.selected_model().unwrap_or("Studio Model");
    cmd_buf.push(DrawCommand::Text {
        text: format!("Pipeline: {} ▾", cur_model),
        pos: Vec2::new(model_rect.min.x + 10.0, model_rect.min.y + 7.0),
        size: 11.0,
        color: p.accent,
        strong: true,
    });

    // D. Service Status Adapter pill (far right)
    let service_rect = Rect::from_min_size(
        Vec2::new(top_bar_rect.max.x - 180.0, btn_y),
        Vec2::new(168.0, 28.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: service_rect,
        fill: p.elevated,
        radius: CornerRadius::same(5.0),
        stroke: Stroke::new(1.0, p.border),
    });
    let dot_rect = Rect::from_min_size(
        Vec2::new(service_rect.min.x + 8.0, service_rect.min.y + 9.0),
        Vec2::new(10.0, 10.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: dot_rect,
        fill: if app.service_adapter.is_connected {
            p.success
        } else {
            p.danger
        },
        radius: CornerRadius::same(5.0),
        stroke: Stroke::NONE,
    });
    cmd_buf.push(DrawCommand::Text {
        text: format!("Worker ({}ms)", app.service_adapter.latency_ms),
        pos: Vec2::new(service_rect.min.x + 24.0, service_rect.min.y + 7.0),
        size: 11.0,
        color: p.text,
        strong: false,
    });

    // 2. Main Workstation Area: delegated to WorkspaceHost
    let workspace_rect = Rect::from_min_size(
        Vec2::new(content_rect.min.x, content_rect.min.y + top_bar_h),
        Vec2::new(
            content_rect.width(),
            (content_rect.height() - top_bar_h).max(50.0),
        ),
    );

    app.creative_workspace
        .render(cmd_buf, workspace_rect, &app.pointer, p);
}

/// Handles mouse down interactions for top control bar in Creative Workspace tab.
pub fn handle_creative_workspace_mouse_down(
    app: &mut UiLabApp,
    pos: Vec2,
    button: PointerButton,
    content_rect: Rect,
) -> bool {
    let top_bar_h = 44.0;
    let top_bar_rect =
        Rect::from_min_size(content_rect.min, Vec2::new(content_rect.width(), top_bar_h));

    if top_bar_rect.contains(pos) && button == PointerButton::Primary {
        let mut cur_x = top_bar_rect.min.x + 12.0;
        let btn_y = top_bar_rect.min.y + 7.0;

        // Presentation mode pill clicks
        let modes = [
            (PresentationMode::Create, "Create"),
            (PresentationMode::Split, "Split"),
            (PresentationMode::Graph, "Graph"),
        ];

        for (mode, _) in modes {
            let mode_rect = Rect::from_min_size(Vec2::new(cur_x, btn_y), Vec2::new(68.0, 28.0));
            if mode_rect.contains(pos) {
                app.creative_workspace.switch_mode(mode);
                return true;
            }
            cur_x += 74.0;
        }

        cur_x += 32.0;

        // Canvas tools clicks
        let tools = [
            CanvasTool::Select,
            CanvasTool::Pan,
            CanvasTool::Brush,
            CanvasTool::Eraser,
        ];

        for tool in tools {
            let tool_rect = Rect::from_min_size(Vec2::new(cur_x, btn_y), Vec2::new(60.0, 28.0));
            if tool_rect.contains(pos) {
                app.creative_workspace.document.canvas.active_tool = tool;
                app.tool_adapter.active_tool = tool;
                return true;
            }
            cur_x += 66.0;
        }

        // Model selector click
        let model_rect = Rect::from_min_size(
            Vec2::new(top_bar_rect.max.x - 420.0, btn_y),
            Vec2::new(220.0, 28.0),
        );
        if model_rect.contains(pos) {
            app.model_adapter.handle_mouse_down(pos, model_rect);
            return true;
        }

        return true;
    }

    // Delegate into WorkspaceHost
    let workspace_rect = Rect::from_min_size(
        Vec2::new(content_rect.min.x, content_rect.min.y + top_bar_h),
        Vec2::new(
            content_rect.width(),
            (content_rect.height() - top_bar_h).max(50.0),
        ),
    );

    if workspace_rect.contains(pos) {
        let ptr_copy = app.pointer.clone();
        app.creative_workspace
            .handle_mouse_down(pos, button, workspace_rect, &ptr_copy);
        return true;
    }

    false
}
