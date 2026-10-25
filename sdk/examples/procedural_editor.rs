//! Interface Demonstration 3: Procedural Node Graph Editor
//!
//! Built exclusively using public Kathamo SDK APIs (`use kathamo::prelude::*;`).
//! Demonstrates:
//! - Visual node editor with draggable nodes, typed pins (Image, Mask, PBR, Float),
//!   bezier links, and group backdrop enclosers
//! - Metadata-driven node parameter inspector
//! - Asset browser for reusable subgraphs and procedural generators
//! - MenuBar, DialogHost (Export Shader modal), and NotificationHost

use kathamo::prelude::*;

pub struct ProceduralGraphEditorApp {
    pub workspace: WorkspaceHost,
    pub menu_bar: MenuBar,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
    pub inspector_snapshot: Option<UiInspectorSnapshot>,
}

impl ProceduralGraphEditorApp {
    pub fn new() -> Self {
        let mut registry = EditorRegistry::new();

        // 1. Procedural Node Graph Editor with default preset nodes
        registry.register_factory("node_graph", "Procedural Flow Graph", "Node Logic", || {
            Box::new(NodeGraphEditor::default())
        });

        // 2. Node Parameter Inspector
        registry.register_factory("property_inspector", "Node Parameters", "Inspectors", || {
            let inspector = MetadataPropertyInspector::new(
                "Selected Node: Make Seamless",
                vec![
                    PropertyItem::enum_choice(
                        "seam_algorithm",
                        "Algorithm",
                        0,
                        vec![
                            "Autocorrelation Signal Fitting".into(),
                            "Poisson Gradient Blending".into(),
                            "Hermite Patch Match".into(),
                        ],
                    ),
                    PropertyItem::float("seam_overlap", "Border Overlap (%)", 0.18, 0.05, 0.50, 0.01),
                    PropertyItem::float("seam_falloff", "Falloff Softness", 0.65, 0.1, 1.0, 0.05),
                    PropertyItem::boolean("preserve_details", "Preserve High Frequency Details", true),
                    PropertyItem::integer("octaves", "Perlin Octaves", 4, 1, 8),
                    PropertyItem::float("roughness_scale", "PBR Roughness Factor", 0.42, 0.0, 1.0, 0.02),
                ],
            );
            Box::new(InspectorEditor { inspector })
        });

        // 3. Subgraph Library Browser
        registry.register_factory("asset_browser", "Node Library", "Data", || {
            let browser = AssetBrowser {
                items: vec![
                    AssetItem::new("gen_simplex", "Simplex Noise 2D", "Generators"),
                    AssetItem::new("gen_voronoi", "Voronoi Cellular F1", "Generators"),
                    AssetItem::new("filter_blur", "Directional Gaussian Blur", "Filters"),
                    AssetItem::new("filter_normal", "Height to Normal (Sobel)", "Filters"),
                    AssetItem::new("mat_pbr", "Standard PBR Surface", "Outputs"),
                    AssetItem::new("mat_toon", "Toon Cel Shader Ramp", "Outputs"),
                ],
                ..Default::default()
            };
            Box::new(AssetBrowserEditor { browser })
        });

        let mut workspace = WorkspaceHost::with_registry(registry);
        workspace.switch_workspace(2); // ProceduralGraph preset

        // Application Menu Bar
        let menu_bar = MenuBar::new(vec![
            Menu::new(
                "File",
                vec![
                    MenuItem::action("file_new_graph", "New Flow Graph", Some("Ctrl+N")),
                    MenuItem::action("file_open_graph", "Open Graph...", Some("Ctrl+O")),
                    MenuItem::separator(),
                    MenuItem::action("file_save_graph", "Save Graph", Some("Ctrl+S")),
                    MenuItem::action("file_export_shader", "Export GLSL Shader...", Some("Ctrl+E")),
                ],
            ),
            Menu::new(
                "Edit",
                vec![
                    MenuItem::action("edit_undo", "Undo Wire Connection", Some("Ctrl+Z")),
                    MenuItem::action("edit_redo", "Redo", Some("Ctrl+Y")),
                    MenuItem::separator(),
                    MenuItem::action("edit_group", "Group Selected Nodes", Some("Ctrl+G")),
                    MenuItem::action("edit_delete", "Delete Selected", Some("Del")),
                ],
            ),
            Menu::new(
                "Graph",
                vec![
                    MenuItem::action("graph_auto_layout", "Auto-Arrange Nodes", Some("Ctrl+Shift+L")),
                    MenuItem::action("graph_zoom_fit", "Frame All Nodes", Some("Home")),
                    MenuItem::action("graph_clean_wires", "Remove Unused Links", None),
                ],
            ),
        ]);

        let mut notifications = NotificationHost::new();
        notifications.info("Graph Ready", "5 procedural nodes active. Pin type checking enforced.");

        Self {
            workspace,
            menu_bar,
            notifications,
            dialogs: DialogHost::new(),
            inspector_snapshot: None,
        }
    }

    pub fn trigger_export_dialog(&mut self) {
        self.dialogs.open(
            "shader_export_modal",
            "Export Procedural Shader",
            "Compile visual node graph to optimized standalone GLSL/Vulkan compute kernel?",
        );
    }
}

impl Default for ProceduralGraphEditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl KathamoApp for ProceduralGraphEditorApp {
    fn title(&self) -> &str {
        "Kathamo Procedural Flow Graph Workstation"
    }

    fn default_size(&self) -> (u32, u32) {
        (1600, 950)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let palette = ctx.theme;
        let bounds = ctx.bounds();

        // 1. Top Menu Bar
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar.render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // 2. Docked Procedural Workspace
        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.render(&mut ctx.cmd_buf, workspace_rect, &ctx.pointer, &palette);

        // 3. Dialogs & Notifications
        self.dialogs.render(&mut ctx.cmd_buf, bounds, &ctx.pointer, &palette);
        self.notifications.render(&mut ctx.cmd_buf, bounds, &palette);

        // 4. UI Inspector Snapshot
        if self.inspector_snapshot.is_none() {
            let mut snap = InterfaceInspector::new_snapshot("procedural_graph_root", bounds);
            InterfaceInspector::add_dock_tab(&mut snap.root, "menu_bar", "MenuBar", menu_rect, true);
            InterfaceInspector::add_editor(&mut snap.root, "workspace_host", "WorkspaceHost", workspace_rect);
            self.inspector_snapshot = Some(snap);
        }
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        let bounds = ctx.bounds();
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));

        if menu_rect.contains(pos) {
            if let Some(action_id) = self.menu_bar.handle_mouse_down(pos, button, menu_rect) {
                if action_id == "file_export_shader" {
                    self.trigger_export_dialog();
                } else if action_id == "file_save_graph" {
                    self.notifications.success("Graph Saved", "Procedural graph definition saved.");
                }
                return true;
            }
        }

        if self.dialogs.is_open {
            if let Some((_id, action)) = self.dialogs.handle_mouse_down(pos, button, bounds) {
                if action == DialogAction::Confirm {
                    self.notifications.success("Export Complete", "GLSL shader code emitted to disk.");
                }
                return true;
            }
        }

        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.handle_mouse_down(pos, button, workspace_rect, &ctx.pointer)
    }
}

fn main() {
    println!("=== Kathamo Demonstration 3: Procedural Node Graph Editor ===");
    let app = ProceduralGraphEditorApp::new();
    let mut runner = HeadlessAppRunner::new(app);

    let mut last_cmd_count = 0;
    for frame_idx in 0..10 {
        last_cmd_count = runner.step_frame();
        assert!(last_cmd_count > 0, "Procedural node graph must render commands");
        if frame_idx == 4 {
            runner.app.trigger_export_dialog();
        }
    }

    assert!(runner.app.dialogs.is_open, "Export shader dialog should be open");
    assert!(runner.app.inspector_snapshot.is_some(), "Inspector snapshot captured");

    println!("✓ Procedural Node Graph Editor simulated successfully.");
    println!("✓ Total commands rendered: {}", last_cmd_count);
    println!("✓ Workspaces active: {}", runner.app.workspace.workspaces.len());
}
