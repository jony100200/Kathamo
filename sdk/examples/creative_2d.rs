//! Interface Demonstration 1: 2D Creative Editor
//!
//! Built exclusively using public Kathamo SDK APIs (`use kathamo::prelude::*;`).
//! Demonstrates:
//! - Interactive 2D canvas with drawing/panning/brush strokes
//! - Multi-layer hierarchy tree with visibility toggles and blend modes
//! - Metadata-driven property inspector for brush size, opacity, and smoothing
//! - MenuBar, DialogHost (Export Canvas modal), and NotificationHost

use kathamo::prelude::*;

pub struct Creative2dEditorApp {
    pub workspace: WorkspaceHost,
    pub menu_bar: MenuBar,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
    pub inspector_snapshot: Option<UiInspectorSnapshot>,
}

impl Creative2dEditorApp {
    pub fn new() -> Self {
        let mut registry = EditorRegistry::new();

        // 1. Customized 2D Canvas Editor
        registry.register_factory("canvas", "2D Canvas", "Viewports", || {
            Box::new(CanvasEditor::new_2d())
        });

        // 2. Customized 2D Layers Hierarchy
        registry.register_factory("hierarchy_tree", "Layers", "Structure", || {
            let mut tree = HierarchyTree::default();
            tree.items = vec![
                HierarchyItem::new("layer_highlights", "Layer 4: Highlights & FX").with_depth(0),
                HierarchyItem::new("layer_color", "Layer 3: Flat Colors & Shading").with_depth(0),
                HierarchyItem::new("layer_inks", "Layer 2: Clean Line Art").with_depth(0),
                HierarchyItem::new("layer_sketch", "Layer 1: Rough Underdrawing").with_depth(0),
                HierarchyItem::new("layer_bg", "Background Canvas").with_depth(0),
            ];
            tree.items[1].selected = true;
            Box::new(HierarchyEditor { tree })
        });

        // 3. Customized 2D Brush & Document Inspector
        registry.register_factory("property_inspector", "Brush & Tool Settings", "Inspectors", || {
            let inspector = MetadataPropertyInspector::new(
                "Inking Brush Properties",
                vec![
                    PropertyItem::float("brush_size", "Size (px)", 18.0, 1.0, 500.0, 1.0),
                    PropertyItem::float("brush_opacity", "Opacity", 1.0, 0.0, 1.0, 0.05),
                    PropertyItem::float("stabilizer", "Streamline Smoothing", 0.65, 0.0, 1.0, 0.05),
                    PropertyItem::boolean("pressure_dynamics", "Pen Pressure Dynamics", true),
                    PropertyItem::enum_choice(
                        "blend_mode",
                        "Blending Mode",
                        0,
                        vec!["Normal".into(), "Multiply".into(), "Screen".into(), "Overlay".into(), "Color Dodge".into()],
                    ),
                    PropertyItem::color("primary_color", "Active Inking Color", Color::rgb(30, 41, 59)),
                ],
            );
            Box::new(InspectorEditor { inspector })
        });

        let mut workspace = WorkspaceHost::with_registry(registry);
        workspace.switch_workspace(0); // Creative2D preset

        // Setup Application Menu Bar
        let menu_bar = MenuBar::new(vec![
            Menu::new(
                "File",
                vec![
                    MenuItem::action("file_new", "New Illustration", Some("Ctrl+N")),
                    MenuItem::action("file_open", "Open Project...", Some("Ctrl+O")),
                    MenuItem::separator(),
                    MenuItem::action("file_save", "Save Artwork", Some("Ctrl+S")),
                    MenuItem::action("file_export", "Export Canvas (PNG/PSD)...", Some("Ctrl+Shift+E")),
                ],
            ),
            Menu::new(
                "Edit",
                vec![
                    MenuItem::action("edit_undo", "Undo Brush Stroke", Some("Ctrl+Z")),
                    MenuItem::action("edit_redo", "Redo", Some("Ctrl+Y")),
                    MenuItem::separator(),
                    MenuItem::action("edit_cut", "Cut Selection", Some("Ctrl+X")),
                    MenuItem::action("edit_copy", "Copy Selection", Some("Ctrl+C")),
                ],
            ),
            Menu::new(
                "Canvas",
                vec![
                    MenuItem::action("canvas_flip_h", "Flip Horizontal", Some("Shift+H")),
                    MenuItem::action("canvas_flip_v", "Flip Vertical", Some("Shift+V")),
                    MenuItem::action("canvas_resize", "Canvas Dimensions...", None),
                ],
            ),
            Menu::new(
                "Workspace",
                vec![
                    MenuItem::action("ws_reset", "Reset 2D Creative Layout", None),
                    MenuItem::action("ws_toggle_hud", "Toggle Distraction-Free Mode", Some("Tab")),
                ],
            ),
        ]);

        let mut notifications = NotificationHost::new();
        notifications.info("Ready", "2D Creative Workstation ready. Resolution: 3840x2160 (600 DPI).");

        Self {
            workspace,
            menu_bar,
            notifications,
            dialogs: DialogHost::new(),
            inspector_snapshot: None,
        }
    }

    /// Trigger the Export Canvas modal dialog.
    pub fn trigger_export_dialog(&mut self) {
        self.dialogs.open(
            "export_modal",
            "Export Canvas",
            "Export high-resolution master asset with full color depth and embedded ICC profile?",
        );
    }
}

impl Default for Creative2dEditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl KathamoApp for Creative2dEditorApp {
    fn title(&self) -> &str {
        "Kathamo 2D Creative Workstation"
    }

    fn default_size(&self) -> (u32, u32) {
        (1440, 900)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let palette = ctx.theme.clone();
        let bounds = ctx.bounds();

        // 1. Top Menu Bar (height: 28px)
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar.render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // 2. Docked Workspace Canvas & Panels
        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.render(&mut ctx.cmd_buf, workspace_rect, &ctx.pointer, &palette);

        // 3. Modal Dialogs (if open)
        self.dialogs.render(&mut ctx.cmd_buf, bounds, &ctx.pointer, &palette);

        // 4. Floating Notifications
        self.notifications.render(&mut ctx.cmd_buf, bounds, &palette);

        // 5. Periodic Inspector Snapshot
        if self.inspector_snapshot.is_none() {
            let mut snap = InterfaceInspector::new_snapshot("app_root", bounds);
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
                if action_id == "file_export" {
                    self.trigger_export_dialog();
                } else if action_id == "file_save" {
                    self.notifications.success("Saved", "Project saved successfully.");
                }
                return true;
            }
        }

        if self.dialogs.is_open {
            if let Some((_id, action)) = self.dialogs.handle_mouse_down(pos, button, bounds) {
                if action == DialogAction::Confirm {
                    self.notifications.success("Export Complete", "Artwork exported as high-resolution PNG.");
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
    println!("=== Kathamo Demonstration 1: 2D Creative Editor ===");
    let app = Creative2dEditorApp::new();
    let mut runner = HeadlessAppRunner::new(app);

    // Step 10 frames to verify layout, rendering, and interaction pipeline
    let mut last_cmd_count = 0;
    for frame_idx in 0..10 {
        last_cmd_count = runner.step_frame();
        assert!(last_cmd_count > 0, "Render commands must be generated");
        if frame_idx == 3 {
            // Simulate clicking Export
            runner.app.trigger_export_dialog();
        }
    }

    assert!(runner.app.dialogs.is_open, "Export modal dialog should be open");
    assert!(runner.app.inspector_snapshot.is_some(), "Inspector snapshot must be captured");

    println!("✓ 2D Creative Editor simulated successfully.");
    println!("✓ Total commands rendered: {}", last_cmd_count);
    println!("✓ Workspaces active: {}", runner.app.workspace.workspaces.len());
}
