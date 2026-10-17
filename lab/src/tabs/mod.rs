//! Lab Tabs Module: Enumerates and dispatches all showcase screens in Kolpona UI Lab.

pub mod design;
pub mod ojon;
pub mod showcases;

use crate::app::UiLabApp;
use ui_core::Rect;
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabTab {
    OjonWorkstation,
    DockingAndOverview,
    DesignSystemAndIcons,
    GodotAnchorsAndMaterial3,
    InfiniteCanvas,
    NodeEditor,
    Timeline,
    VirtualizedList100k,
    AsyncWorkers,
    RendererRecovery,
}

pub fn render_tab(
    app: &mut UiLabApp,
    tab: LabTab,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    palette: &ThemePalette,
) {
    match tab {
        LabTab::OjonWorkstation => {
            ojon::render_ojon_workstation_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::DockingAndOverview => {
            showcases::render_docking_overview_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::DesignSystemAndIcons => {
            design::render_design_system_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::GodotAnchorsAndMaterial3 => {
            showcases::render_anchors_and_material3_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::InfiniteCanvas => {
            showcases::render_infinite_canvas_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::NodeEditor => {
            showcases::render_node_editor_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::Timeline => {
            showcases::render_timeline_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::VirtualizedList100k => {
            showcases::render_virtualized_list_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::AsyncWorkers => {
            showcases::render_async_workers_tab(app, cmd_buf, content_rect, palette);
        }
        LabTab::RendererRecovery => {
            showcases::render_renderer_recovery_tab(app, cmd_buf, content_rect, palette);
        }
    }
}
