//! Interface Demonstration 2: 3D Spatial & DCC Editor
//!
//! Built exclusively using public Kathamo SDK APIs (`use kathamo::prelude::*;`).
//! Demonstrates:
//! - 3D viewport canvas host receiving external GPU render textures (1920x1080)
//! - Scene hierarchy outliner with parent-child nesting, cameras, lights, and meshes
//! - Metadata-driven 3D transform and PBR material inspector (Position, Rotation, Scale, Albedo, Roughness, Metallic)
//! - Asset browser for 3D meshes, textures, and material shaders
//! - MenuBar, DialogHost (Build Settings modal), and NotificationHost

use kathamo::prelude::*;

pub struct Spatial3dEditorApp {
    pub workspace: WorkspaceHost,
    pub menu_bar: MenuBar,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
    pub inspector_snapshot: Option<UiInspectorSnapshot>,
}

impl Spatial3dEditorApp {
    pub fn new() -> Self {
        let mut registry = EditorRegistry::new();

        // 1. Customized 3D Viewport Host (Blits external GPU texture)
        registry.register_factory("viewport_3d", "3D Viewport", "Viewports", || {
            Box::new(CanvasEditor::new_3d_mock())
        });

        // 2. Customized Scene Graph Hierarchy Outliner
        registry.register_factory("hierarchy_tree", "Scene Outliner", "Structure", || {
            let mut hero_item = HierarchyItem::new("actor_hero", "Character_Model_LOD0").with_depth(0);
            hero_item.selected = true;

            let tree = HierarchyTree {
                items: vec![
                    HierarchyItem::new("cam_main", "Main Camera").with_depth(0),
                    HierarchyItem::new("light_sun", "Directional Sun Light").with_depth(0),
                    hero_item,
                    HierarchyItem::new("mesh_body", "Body_SkinnedMesh").with_depth(1),
                    HierarchyItem::new("mesh_armor", "Armor_Plate_LOD0").with_depth(1),
                    HierarchyItem::new("armature_root", "Armature_Skeleton").with_depth(1),
                    HierarchyItem::new("env_root", "Environment_Biome").with_depth(0),
                    HierarchyItem::new("env_terrain", "Terrain_Chunk_01").with_depth(1),
                    HierarchyItem::new("env_water", "Water_Reflection_Plane").with_depth(1),
                ],
                ..Default::default()
            };
            Box::new(HierarchyEditor { tree })
        });

        // 3. Customized 3D Transform & PBR Material Inspector
        registry.register_factory("property_inspector", "Transform & Material", "Inspectors", || {
            let inspector = MetadataPropertyInspector::new(
                "Selected: Character_Model_LOD0",
                vec![
                    // Transform Section
                    PropertyItem::vec2("trans_pos", "Position (X, Y)", 0.0, 1.25, -100.0, 100.0),
                    PropertyItem::vec2("trans_rot", "Rotation (Pitch, Yaw)", 0.0, 45.0, -180.0, 180.0),
                    PropertyItem::vec2("trans_scale", "Scale (X, Y)", 1.0, 1.0, 0.01, 10.0),
                    // Material PBR Section
                    PropertyItem::color("mat_albedo", "Albedo Tint", Color::rgb(212, 163, 115)),
                    PropertyItem::float("mat_metallic", "Metallic", 0.85, 0.0, 1.0, 0.01),
                    PropertyItem::float("mat_roughness", "Roughness", 0.24, 0.0, 1.0, 0.01),
                    PropertyItem::float("mat_ao", "Ambient Occlusion", 1.0, 0.0, 1.0, 0.05),
                    PropertyItem::boolean("cast_shadows", "Cast Dynamic Shadows", true),
                    PropertyItem::boolean("receive_shadows", "Receive Shadows", true),
                ],
            );
            Box::new(InspectorEditor { inspector })
        });

        // 4. Asset Browser pre-populated with 3D project assets
        registry.register_factory("asset_browser", "Project Assets", "Data", || {
            let browser = AssetBrowser {
                items: vec![
                    AssetItem::new("mesh_hero", "Character_Hero.fbx", "Meshes"),
                    AssetItem::new("mesh_sword", "Broadsword_PBR.obj", "Meshes"),
                    AssetItem::new("mat_steel", "Damascus_Steel.ksmat", "Materials"),
                    AssetItem::new("mat_gold", "Gold_Trim.ksmat", "Materials"),
                    AssetItem::new("tex_diffuse", "Hero_BaseColor.png", "Textures"),
                    AssetItem::new("tex_normal", "Hero_Normal.png", "Textures"),
                ],
                ..Default::default()
            };
            Box::new(AssetBrowserEditor { browser })
        });

        let mut workspace = WorkspaceHost::with_registry(registry);
        workspace.switch_workspace(1); // Spatial3D preset

        // Application Menu Bar
        let menu_bar = MenuBar::new(vec![
            Menu::new(
                "File",
                vec![
                    MenuItem::action("file_new_scene", "New Scene", Some("Ctrl+N")),
                    MenuItem::action("file_open_scene", "Open Scene...", Some("Ctrl+O")),
                    MenuItem::separator(),
                    MenuItem::action("file_save_scene", "Save Scene", Some("Ctrl+S")),
                    MenuItem::action("file_build", "Build & Package Application...", Some("Ctrl+Shift+B")),
                ],
            ),
            Menu::new(
                "Edit",
                vec![
                    MenuItem::action("edit_undo", "Undo Transform", Some("Ctrl+Z")),
                    MenuItem::action("edit_redo", "Redo", Some("Ctrl+Y")),
                    MenuItem::separator(),
                    MenuItem::action("edit_duplicate", "Duplicate Object", Some("Ctrl+D")),
                    MenuItem::action("edit_delete", "Delete Selected", Some("Del")),
                ],
            ),
            Menu::new(
                "GameObject",
                vec![
                    MenuItem::action("go_empty", "Create Empty", Some("Ctrl+Shift+N")),
                    MenuItem::action("go_cube", "3D Object -> Cube", None),
                    MenuItem::action("go_sphere", "3D Object -> Sphere", None),
                    MenuItem::action("go_light", "Light -> Directional Sun", None),
                    MenuItem::action("go_camera", "Camera -> Perspective", None),
                ],
            ),
            Menu::new(
                "Window",
                vec![
                    MenuItem::action("win_reset", "Reset 3D Spatial Layout", None),
                    MenuItem::action("win_max_vp", "Maximize Viewport", Some("Shift+Space")),
                ],
            ),
        ]);

        let mut notifications = NotificationHost::new();
        notifications.info("Vulkan PBR", "3D Viewport connected to Vulkan hardware rasterizer (60 FPS).");

        Self {
            workspace,
            menu_bar,
            notifications,
            dialogs: DialogHost::new(),
            inspector_snapshot: None,
        }
    }

    pub fn trigger_build_dialog(&mut self) {
        self.dialogs.open(
            "build_modal",
            "Build & Packaging Settings",
            "Target Architecture: Windows x86_64 / Linux Vulkan. Compile standalone optimized package?",
        );
    }
}

impl Default for Spatial3dEditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl KathamoApp for Spatial3dEditorApp {
    fn title(&self) -> &str {
        "Kathamo 3D Spatial DCC Workstation"
    }

    fn default_size(&self) -> (u32, u32) {
        (1600, 1000)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let palette = ctx.theme;
        let bounds = ctx.bounds();

        // 1. Menu Bar
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar.render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // 2. 3D Docked Workspace
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
            let mut snap = InterfaceInspector::new_snapshot("spatial_3d_root", bounds);
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
                if action_id == "file_build" {
                    self.trigger_build_dialog();
                } else if action_id == "file_save_scene" {
                    self.notifications.success("Scene Saved", "All 3D actor transforms committed to disk.");
                }
                return true;
            }
        }

        if self.dialogs.is_open {
            if let Some((_id, action)) = self.dialogs.handle_mouse_down(pos, button, bounds) {
                if action == DialogAction::Confirm {
                    self.notifications.success("Build Started", "Packaging release binary...");
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

    fn handle_mouse_move(&mut self, pos: Vec2, delta: Vec2, ctx: &mut AppContext) -> bool {
        if self.dialogs.is_open { return false; }
        let bounds = ctx.bounds();
        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.handle_mouse_move(pos, delta, workspace_rect, &ctx.pointer)
    }

    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        let bounds = ctx.bounds();
        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.handle_mouse_up(pos, button, workspace_rect)
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, ctx: &mut AppContext) -> bool {
        if self.dialogs.is_open { return false; }
        let bounds = ctx.bounds();
        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.handle_scroll(delta, pos, workspace_rect)
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--native") {
        run_native(Spatial3dEditorApp::new()).expect("Kathamo native application failed");
        return;
    }
    println!("=== Kathamo Demonstration 2: 3D Spatial & DCC Editor ===");
    let app = Spatial3dEditorApp::new();
    let mut runner = HeadlessAppRunner::new(app);

    let mut last_cmd_count = 0;
    for frame_idx in 0..10 {
        last_cmd_count = runner.step_frame();
        assert!(last_cmd_count > 0, "3D Workspace must generate render commands");
        if frame_idx == 4 {
            runner.app.trigger_build_dialog();
        }
    }

    assert!(runner.app.dialogs.is_open, "Build settings dialog should be open");
    assert!(runner.app.inspector_snapshot.is_some(), "Inspector snapshot captured");

    println!("✓ 3D Spatial DCC Editor simulated successfully.");
    println!("✓ Total commands rendered: {}", last_cmd_count);
    println!("✓ Workspaces active: {}", runner.app.workspace.workspaces.len());
}
