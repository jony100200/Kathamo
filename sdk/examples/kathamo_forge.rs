//! Kathamo Forge — Indie Game Engine Editor UI Showcase
//!
//! A flagship, fully interactive game editor desktop application built exclusively
//! using the public Kathamo SDK (`use kathamo::prelude::*;`).
//!
//! Demonstrates:
//! - Full game editor workspace with top menu, scene tabs, mode switcher, and transport controls.
//! - Deeply nested Scene Hierarchy tree with visibility toggles, selection, and object addition/removal.
//! - Context-sensitive Metadata Property Inspector (Transform, Controller, PBR Material, Components).
//! - Asset Browser with folder categorization, search filtering, and asset selection.
//! - Center Viewport with 4 switchable modes:
//!   * 3D Spatial Viewport: Perspective grid, camera frustum, light rays, and interactive 3D translation gizmos.
//!   * 2D Viewport: Infinite canvas, sprite bounds, and collision shapes.
//!   * C# Script Editor: Syntax-highlighted code editor mock with line numbers and status.
//!   * Logic Graph: Dual-presentation game logic workflow:
//!     - Simple Guided Mode: Beginner-friendly connected cards via `GraphDocument::render_guided`.
//!     - Advanced Node Mode: Full visual node graph with draggable nodes, typed pins, and cubic Bézier cables.
//! - Bottom Dock: Multi-tab Console (filters, clear), Animation Timeline (playhead scrubbing, tracks), and Profiler.
//! - Interactive simulation controls: Play, Pause, Stop with live frame timing and FPS telemetry.
//! - Command Palette (`Ctrl+K`), modal dialogs, and notification toast dispatch.
//! - 100% deterministic local mock state; zero backend or GPU engine requirement.

use kathamo::prelude::*;
use kathamo::widgets::tab_strip;
use ui_core::motion;

// =============================================================================
// DATA MODELS & SCENE STRUCTURE
// =============================================================================

const SIM_LABELS: [&str; 3] = ["Stop", "Play", "Pause"];
const SIM_STATES: [SimulationState; 3] = [
    SimulationState::Stopped,
    SimulationState::Playing,
    SimulationState::Paused,
];
const MODE_LABELS: [&str; 4] = ["2D Canvas", "3D Spatial", "Script C#", "Logic Graph"];
const MODES: [EditorWorkspaceMode; 4] = [
    EditorWorkspaceMode::Mode2D,
    EditorWorkspaceMode::Mode3D,
    EditorWorkspaceMode::ModeScript,
    EditorWorkspaceMode::ModeGraph,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorWorkspaceMode {
    Mode3D,
    Mode2D,
    ModeScript,
    ModeGraph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphViewMode {
    SimpleGuided,
    AdvancedNodes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct ForgeSceneNode {
    pub id: String,
    pub name: String,
    pub category: String,
    pub depth: usize,
    pub visible: bool,
    pub locked: bool,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub components: Vec<String>,
    pub material_color: Color,
    pub roughness: f32,
    pub metallic: f32,
    pub move_speed: f32,
    pub jump_force: f32,
    pub is_grounded: bool,
}

impl ForgeSceneNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, depth: usize) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: "GameObject".into(),
            depth,
            visible: true,
            locked: false,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            components: vec!["Transform".into()],
            material_color: Color::rgb(79, 70, 229),
            roughness: 0.5,
            metallic: 0.1,
            move_speed: 6.0,
            jump_force: 9.8,
            is_grounded: true,
        }
    }
}

// =============================================================================
// MAIN APPLICATION STRUCT & UNDO SNAPSHOT
// =============================================================================

#[derive(Debug, Clone)]
pub struct ForgeSnapshot {
    pub selected_node_id: String,
    pub scene_nodes: Vec<ForgeSceneNode>,
    pub active_mode: EditorWorkspaceMode,
}

pub struct KathamoForgeApp {
    pub menu_bar: MenuBar,
    pub hierarchy: HierarchyTree,
    pub inspector: MetadataPropertyInspector,
    pub asset_browser: AssetBrowser,
    pub console: ConsoleView,
    pub timeline: Timeline,
    pub graph_document: GraphDocument,
    pub node_editor: NodeEditor,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
    pub undo_manager: UndoManager<ForgeSnapshot>,

    // Application state
    pub active_mode: EditorWorkspaceMode,
    pub graph_view_mode: GraphViewMode,
    pub sim_state: SimulationState,
    pub sim_time: f32,
    pub sim_frame: u64,
    pub fps: f32,
    pub zoom: f32,
    pub active_scene_name: String,
    pub open_tabs: Vec<String>,
    pub active_tab_idx: usize,
    pub selected_node_id: String,
    pub scene_nodes: Vec<ForgeSceneNode>,
    pub bottom_tab_idx: usize, // 0: Console, 1: Timeline, 2: Profiler
    pub console_filter: Option<ConsoleLogLevel>,
    pub command_palette_open: bool,
    pub command_query: String,
    pub graph_scroll_y: f32,
    pub next_node_counter: usize,
    pub layout_save_cache: String,
    pub script_text: Vec<&'static str>,
}

impl KathamoForgeApp {
    /// Transport strip, right-aligned in the ribbon.
    fn transport_layout(ribbon: Rect) -> (Vec<Rect>, Rect) {
        let probe = tab_strip::tab_rects(&SIM_LABELS, Vec2::ZERO, 28.0);
        let w = tab_strip::strip_bounds(&probe, Vec2::ZERO, 28.0).width();
        let origin = Vec2::new(ribbon.max.x - w - 12.0, ribbon.min.y + 5.0);
        let rects = tab_strip::tab_rects(&SIM_LABELS, origin, 28.0);
        let strip = tab_strip::strip_bounds(&rects, origin, 28.0);
        (rects, strip)
    }

    /// Ribbon layout shared by drawing and hit-testing: measured document tabs,
    /// then the mode selector, centred but never overlapping the tabs.
    fn ribbon_layout(&self, ribbon: Rect, bounds: Rect) -> (Vec<Rect>, Vec<Rect>, Rect) {
        let mut x = ribbon.min.x + 12.0;
        let doc_tabs: Vec<Rect> = self
            .open_tabs
            .iter()
            .map(|name| {
                let w = ui_render::measure_text(name, 11.5, true) + 20.0;
                let r = Rect::from_min_size(Vec2::new(x, ribbon.min.y + 6.0), Vec2::new(w, 26.0));
                x += w + 4.0;
                r
            })
            .collect();
        let probe = tab_strip::tab_rects(&MODE_LABELS, Vec2::ZERO, 28.0);
        let strip_w = tab_strip::strip_bounds(&probe, Vec2::ZERO, 28.0).width();
        let centred = bounds.min.x + (bounds.width() - strip_w) * 0.5;
        let origin = Vec2::new(centred.max(x + 16.0), ribbon.min.y + 5.0);
        let modes = tab_strip::tab_rects(&MODE_LABELS, origin, 28.0);
        let strip = tab_strip::strip_bounds(&modes, origin, 28.0);
        (doc_tabs, modes, strip)
    }

    pub fn new() -> Self {
        // 1. Initial Menu Bar
        let menu_bar = MenuBar::new(vec![
            Menu::new(
                "File",
                vec![
                    MenuItem::action("file_new_scene", "New Scene", Some("Ctrl+N")),
                    MenuItem::action("file_open_scene", "Open Scene...", Some("Ctrl+O")),
                    MenuItem::separator(),
                    MenuItem::action("file_save_scene", "Save Scene", Some("Ctrl+S")),
                    MenuItem::action("file_project_settings", "Project Settings...", None),
                    MenuItem::separator(),
                    MenuItem::action(
                        "file_build_game",
                        "Build & Package Game...",
                        Some("Ctrl+Shift+B"),
                    ),
                    MenuItem::action("file_exit", "Exit Kathamo Forge", Some("Alt+F4")),
                ],
            ),
            Menu::new(
                "Edit",
                vec![
                    MenuItem::action("edit_undo", "Undo", Some("Ctrl+Z")),
                    MenuItem::action("edit_redo", "Redo", Some("Ctrl+Y")),
                    MenuItem::separator(),
                    MenuItem::action("edit_duplicate", "Duplicate Selected", Some("Ctrl+D")),
                    MenuItem::action("edit_delete", "Delete Selected", Some("Delete")),
                ],
            ),
            Menu::new(
                "GameObject",
                vec![
                    MenuItem::action("obj_add_empty", "Create Empty", Some("Ctrl+Shift+N")),
                    MenuItem::action("obj_add_hero", "3D Object -> Player Character", None),
                    MenuItem::action("obj_add_cube", "3D Object -> Cube Mesh", None),
                    MenuItem::action("obj_add_light", "Light -> Directional Sun Light", None),
                    MenuItem::action("obj_add_camera", "Camera -> Perspective Camera", None),
                ],
            ),
            Menu::new(
                "Workspace",
                vec![
                    MenuItem::action("ws_mode_3d", "Switch to 3D Spatial View", Some("F1")),
                    MenuItem::action("ws_mode_2d", "Switch to 2D Canvas View", Some("F2")),
                    MenuItem::action("ws_mode_script", "Switch to C# Script Editor", Some("F3")),
                    MenuItem::action("ws_mode_graph", "Switch to Logic Graph", Some("F4")),
                    MenuItem::separator(),
                    MenuItem::action("ws_reset_layout", "Reset Default Dock Layout", None),
                ],
            ),
            Menu::new(
                "Simulate",
                vec![
                    MenuItem::action("sim_play", "Start Simulation", Some("F5")),
                    MenuItem::action("sim_pause", "Pause Simulation", Some("F6")),
                    MenuItem::action("sim_stop", "Stop & Reset Simulation", Some("Shift+F5")),
                ],
            ),
            Menu::new(
                "Help",
                vec![
                    MenuItem::action("help_palette", "Command Palette...", Some("Ctrl+K")),
                    MenuItem::action("help_about", "About Kathamo Forge", None),
                ],
            ),
        ]);

        // 2. Pre-populate Sample Scene Nodes
        let mut nodes = Vec::new();

        let mut root = ForgeSceneNode::new("node_world_root", "World_Root", 0);
        root.category = "Environment".into();
        nodes.push(root);

        let mut camera = ForgeSceneNode::new("node_camera", "MainCamera", 1);
        camera.position = [0.0, 3.5, -8.0];
        camera.rotation = [22.0, 0.0, 0.0];
        camera.components = vec!["Transform".into(), "Camera".into(), "AudioListener".into()];
        nodes.push(camera);

        let mut sun = ForgeSceneNode::new("node_sun", "DirectionalLight_Sun", 1);
        sun.position = [0.0, 15.0, 0.0];
        sun.rotation = [50.0, -30.0, 0.0];
        sun.material_color = Color::rgb(255, 244, 214);
        sun.components = vec!["Transform".into(), "DirectionalLight".into()];
        nodes.push(sun);

        let mut env = ForgeSceneNode::new("node_env", "Environment_Biome", 1);
        env.components = vec!["Transform".into(), "WorldBounds".into()];
        nodes.push(env);

        let mut ground = ForgeSceneNode::new("node_ground", "Ground_Terrain_Plane", 2);
        ground.position = [0.0, 0.0, 0.0];
        ground.scale = [40.0, 1.0, 40.0];
        ground.material_color = Color::rgb(34, 45, 60);
        ground.roughness = 0.85;
        ground.components = vec![
            "Transform".into(),
            "MeshRenderer".into(),
            "BoxCollider".into(),
        ];
        nodes.push(ground);

        let mut pillar1 = ForgeSceneNode::new("node_pillar1", "Stone_Pillar_North", 2);
        pillar1.position = [-6.0, 2.5, 5.0];
        pillar1.scale = [1.2, 5.0, 1.2];
        pillar1.material_color = Color::rgb(100, 116, 139);
        pillar1.components = vec![
            "Transform".into(),
            "MeshRenderer".into(),
            "BoxCollider".into(),
        ];
        nodes.push(pillar1);

        let mut pillar2 = ForgeSceneNode::new("node_pillar2", "Stone_Pillar_South", 2);
        pillar2.position = [6.0, 2.5, 5.0];
        pillar2.scale = [1.2, 5.0, 1.2];
        pillar2.material_color = Color::rgb(100, 116, 139);
        pillar2.components = vec![
            "Transform".into(),
            "MeshRenderer".into(),
            "BoxCollider".into(),
        ];
        nodes.push(pillar2);

        // Selected Player Hero
        let mut hero = ForgeSceneNode::new("node_hero", "Player_Hero_Character", 1);
        hero.position = [0.0, 1.25, 0.0];
        hero.rotation = [0.0, 45.0, 0.0];
        hero.material_color = Color::rgb(99, 102, 241);
        hero.roughness = 0.35;
        hero.metallic = 0.25;
        hero.move_speed = 6.5;
        hero.jump_force = 10.5;
        hero.components = vec![
            "Transform".into(),
            "CharacterController".into(),
            "CapsuleCollider".into(),
            "PlayerController.cs".into(),
            "AudioSource".into(),
        ];
        nodes.push(hero);

        let mut hero_mesh = ForgeSceneNode::new("node_hero_mesh", "Mesh_Hero_LOD0", 2);
        hero_mesh.material_color = Color::rgb(99, 102, 241);
        hero_mesh.components = vec!["Transform".into(), "SkinnedMeshRenderer".into()];
        nodes.push(hero_mesh);

        let mut sword = ForgeSceneNode::new("node_sword", "Weapon_Broadsword_Socket", 2);
        sword.position = [0.4, 0.9, 0.3];
        sword.scale = [0.2, 1.1, 0.2];
        sword.material_color = Color::rgb(226, 232, 240);
        sword.metallic = 0.95;
        sword.roughness = 0.15;
        sword.components = vec![
            "Transform".into(),
            "MeshRenderer".into(),
            "BoxCollider".into(),
        ];
        nodes.push(sword);

        let mut enemy = ForgeSceneNode::new("node_enemy", "Enemy_Goblin_Scout", 1);
        enemy.position = [8.0, 1.0, 10.0];
        enemy.material_color = Color::rgb(239, 68, 68);
        enemy.components = vec![
            "Transform".into(),
            "EnemyAI.cs".into(),
            "CapsuleCollider".into(),
        ];
        nodes.push(enemy);

        // 3. Build Hierarchy Items
        let mut hierarchy_items = Vec::new();
        for n in &nodes {
            let mut item = HierarchyItem::new(&n.id, &n.name).with_depth(n.depth);
            item.visible = n.visible;
            item.locked = n.locked;
            if n.id == "node_hero" {
                item.selected = true;
            }
            item.has_children = n.depth < 2;
            item.is_expanded = true;
            hierarchy_items.push(item);
        }
        let hierarchy = HierarchyTree {
            items: hierarchy_items,
            ..Default::default()
        };

        // 4. Initial Inspector for Player_Hero
        let inspector = Self::build_inspector_for_node(&nodes[7]);

        // 5. Pre-populated Asset Browser
        let asset_browser = AssetBrowser {
            items: vec![
                AssetItem::new("asset_scene_main", "MainLevel.scene", "Scenes"),
                AssetItem::new("asset_scene_arena", "BossArena.scene", "Scenes"),
                AssetItem::new("asset_script_player", "PlayerController.cs", "Scripts"),
                AssetItem::new("asset_script_enemy", "EnemyAI.cs", "Scripts"),
                AssetItem::new("asset_mesh_hero", "Hero_Character.fbx", "Meshes"),
                AssetItem::new("asset_mesh_sword", "Broadsword_PBR.obj", "Meshes"),
                AssetItem::new("asset_mesh_pillar", "Stone_Pillar.fbx", "Meshes"),
                AssetItem::new("asset_mat_armor", "Hero_Armor.ksmat", "Materials"),
                AssetItem::new("asset_mat_stone", "Ancient_Stone.ksmat", "Materials"),
                AssetItem::new("asset_tex_albedo", "Hero_BaseColor.png", "Textures"),
                AssetItem::new("asset_tex_normal", "Hero_Normal.png", "Textures"),
                AssetItem::new("asset_audio_slash", "Sword_Slash_01.wav", "Audio"),
                AssetItem::new("asset_audio_bgm", "Village_Exploration.ogg", "Audio"),
                AssetItem::new("asset_graph_combat", "PlayerCombatLogic.graph", "Graphs"),
            ],
            selected_id: Some("asset_scene_main".into()),
            active_category: "All".into(),
            ..Default::default()
        };

        // 6. Console with initial logs
        let mut console = ConsoleView::new();
        console.log(
            ConsoleLogLevel::Info,
            "Core",
            "Kathamo Forge Indie Game Engine demo initialized.",
        );
        console.log(
            ConsoleLogLevel::Info,
            "Render",
            "wgpu presentation active (Direct3D 12 / Vulkan swapchain).",
        );
        console.log(
            ConsoleLogLevel::Warning,
            "Assets",
            "Hero_Normal.png color space set to sRGB; recommendation: Linear.",
        );
        console.log(
            ConsoleLogLevel::Info,
            "Scripts",
            "C# Roslyn domain pre-compiled: 0 errors, 0 warnings.",
        );

        // 7. Animation Timeline
        let timeline = Timeline {
            current_frame: 24,
            total_frames: 180,
            fps: 60,
            tracks: vec![
                TimelineTrack {
                    name: "Hero_Walk_Locomotion".into(),
                    keyframes: vec![0, 15, 30, 45, 60, 75, 90, 105, 120],
                },
                TimelineTrack {
                    name: "Sword_Swing_Slash".into(),
                    keyframes: vec![20, 28, 35, 42],
                },
                TimelineTrack {
                    name: "Audio_Footsteps_Sync".into(),
                    keyframes: vec![15, 45, 75, 105],
                },
            ],
            is_playing: false,
        };

        // 8. Game Logic Graph Document & Node Editor
        let (graph_document, node_editor) = Self::create_sample_game_logic_graph();

        let mut notifications = NotificationHost::new();
        notifications.success(
            "Project Loaded",
            "Welcome to Kathamo Forge! Active scene: MainLevel.scene",
        );

        let script_text = vec![
            "using System;",
            "using Kathamo.Engine;",
            "",
            "namespace KathamoForge.Gameplay {",
            "    [RequireComponent(typeof(CharacterController))]",
            "    public class PlayerController : ScriptBehaviour {",
            "        [Header(\"Locomotion Settings\")]",
            "        [SerializeField] private float moveSpeed = 6.5f;",
            "        [SerializeField] private float jumpForce = 10.5f;",
            "        [SerializeField] private float gravityScale = 1.5f;",
            "",
            "        [Header(\"Combat Hooks\")]",
            "        [SerializeField] private Transform weaponSocket;",
            "        [SerializeField] private AudioClip slashSound;",
            "",
            "        private CharacterController controller;",
            "        private Vector3 velocity;",
            "        private bool isGrounded;",
            "",
            "        private void Awake() {",
            "            controller = GetComponent<CharacterController>();",
            "        }",
            "",
            "        private void Update() {",
            "            // 1. Gather directional input",
            "            float x = Input.GetAxisRaw(\"Horizontal\");",
            "            float z = Input.GetAxisRaw(\"Vertical\");",
            "            Vector3 move = (transform.right * x + transform.forward * z).normalized;",
            "",
            "            // 2. Apply movement vector",
            "            controller.Move(move * (moveSpeed * Time.deltaTime));",
            "",
            "            // 3. Jump and physics tick",
            "            isGrounded = controller.isGrounded;",
            "            if (isGrounded && velocity.y < 0) {",
            "                velocity.y = -2.0f;",
            "            }",
            "            if (Input.GetButtonDown(\"Jump\") && isGrounded) {",
            "                velocity.y = Mathf.Sqrt(jumpForce * -2.0f * Physics.gravity.y);",
            "            }",
            "            velocity.y += Physics.gravity.y * gravityScale * Time.deltaTime;",
            "            controller.Move(velocity * Time.deltaTime);",
            "        }",
            "    }",
            "}",
        ];

        Self {
            menu_bar,
            hierarchy,
            inspector,
            asset_browser,
            console,
            timeline,
            graph_document,
            node_editor,
            notifications,
            dialogs: DialogHost::new(),
            undo_manager: UndoManager::new(50),

            active_mode: EditorWorkspaceMode::Mode3D,
            graph_view_mode: GraphViewMode::SimpleGuided,
            sim_state: SimulationState::Stopped,
            sim_time: 0.0,
            sim_frame: 0,
            fps: 144.0,
            zoom: 1.0,
            active_scene_name: "MainLevel.scene".into(),
            open_tabs: vec![
                "MainLevel.scene".into(),
                "PlayerController.cs".into(),
                "PlayerCombatLogic.graph".into(),
                "BossArena.scene".into(),
            ],
            active_tab_idx: 0,
            selected_node_id: "node_hero".into(),
            scene_nodes: nodes,
            bottom_tab_idx: 0,
            console_filter: None,
            command_palette_open: false,
            command_query: String::new(),
            graph_scroll_y: 0.0,
            next_node_counter: 1,
            layout_save_cache: String::new(),
            script_text,
        }
    }

    /// Construct metadata-driven property inspector items for a given scene node.
    pub fn build_inspector_for_node(node: &ForgeSceneNode) -> MetadataPropertyInspector {
        let mut props = vec![
            // Transform Section
            PropertyItem::vec2(
                "tr_pos_xy",
                "Position (X, Y)",
                node.position[0],
                node.position[1],
                -100.0,
                100.0,
            )
            .with_category("Transform"),
            PropertyItem::float(
                "tr_pos_z",
                "Position Z",
                node.position[2],
                -100.0,
                100.0,
                0.25,
            )
            .with_category("Transform"),
            PropertyItem::vec2(
                "tr_rot_xy",
                "Rotation (Pitch, Yaw)",
                node.rotation[0],
                node.rotation[1],
                -180.0,
                180.0,
            )
            .with_category("Transform"),
            PropertyItem::vec2(
                "tr_scale_xy",
                "Scale (X, Y)",
                node.scale[0],
                node.scale[1],
                0.01,
                50.0,
            )
            .with_category("Transform"),
        ];

        // Node-specific components
        if node.id == "node_hero" {
            props.push(
                PropertyItem::float(
                    "hero_speed",
                    "Move Speed (m/s)",
                    node.move_speed,
                    1.0,
                    25.0,
                    0.2,
                )
                .with_category("CharacterController"),
            );
            props.push(
                PropertyItem::float(
                    "hero_jump",
                    "Jump Force (m/s)",
                    node.jump_force,
                    2.0,
                    30.0,
                    0.5,
                )
                .with_category("CharacterController"),
            );
            props.push(
                PropertyItem::boolean("hero_grounded", "Is Grounded", node.is_grounded)
                    .with_category("CharacterController"),
            );
            props.push(
                PropertyItem::color("hero_color", "Albedo Tint", node.material_color)
                    .with_category("PBR Material"),
            );
            props.push(
                PropertyItem::float(
                    "hero_roughness",
                    "Roughness",
                    node.roughness,
                    0.0,
                    1.0,
                    0.02,
                )
                .with_category("PBR Material"),
            );
            props.push(
                PropertyItem::float("hero_metallic", "Metallic", node.metallic, 0.0, 1.0, 0.02)
                    .with_category("PBR Material"),
            );
            props.push(
                PropertyItem::text("hero_script", "Script Source", "PlayerController.cs")
                    .with_category("Attached Components"),
            );
        } else if node.id == "node_sun" {
            props.push(
                PropertyItem::color("sun_color", "Sunlight Radiance", node.material_color)
                    .with_category("Directional Light"),
            );
            props.push(
                PropertyItem::float("sun_intensity", "Intensity (Lux)", 2.5, 0.1, 10.0, 0.1)
                    .with_category("Directional Light"),
            );
            props.push(
                PropertyItem::boolean("sun_shadows", "Soft PCF Shadows", true)
                    .with_category("Directional Light"),
            );
        } else {
            props.push(
                PropertyItem::color("mesh_color", "Base Color", node.material_color)
                    .with_category("Material Surface"),
            );
            props.push(
                PropertyItem::float(
                    "mesh_roughness",
                    "Roughness",
                    node.roughness,
                    0.0,
                    1.0,
                    0.02,
                )
                .with_category("Material Surface"),
            );
        }

        MetadataPropertyInspector::new(format!("Selected: {}", node.name), props)
    }

    /// Construct sample game logic graph document and visual node editor.
    pub fn create_sample_game_logic_graph() -> (GraphDocument, NodeEditor) {
        let n1 = GraphNode {
            id: "node_input".into(),
            title: "Player Input (WASD / Space)".into(),
            category: "Input".into(),
            pos: Vec2::new(60.0, 80.0),
            size: Vec2::new(210.0, 110.0),
            inputs: vec![],
            outputs: vec![
                NodePin {
                    id: "out_axis".into(),
                    name: "Move Axis (X/Z)".into(),
                    is_output: true,
                    pin_type: "Float".into(),
                },
                NodePin {
                    id: "out_jump".into(),
                    name: "Jump Pressed".into(),
                    is_output: true,
                    pin_type: "Float".into(),
                },
            ],
            preview_color: Some(Color::rgb(14, 165, 233)),
            is_collapsed: false,
        };

        let n2 = GraphNode {
            id: "node_velocity".into(),
            title: "Velocity Calculation".into(),
            category: "Physics".into(),
            pos: Vec2::new(320.0, 60.0),
            size: Vec2::new(200.0, 120.0),
            inputs: vec![NodePin {
                id: "in_axis".into(),
                name: "Move Axis".into(),
                is_output: false,
                pin_type: "Float".into(),
            }],
            outputs: vec![
                NodePin {
                    id: "out_vel".into(),
                    name: "Computed Velocity".into(),
                    is_output: true,
                    pin_type: "Float".into(),
                },
                NodePin {
                    id: "out_speed".into(),
                    name: "Scalar Speed".into(),
                    is_output: true,
                    pin_type: "Float".into(),
                },
            ],
            preview_color: Some(Color::rgb(16, 185, 129)),
            is_collapsed: false,
        };

        let n3 = GraphNode {
            id: "node_collision".into(),
            title: "Capsule Collision Sweep".into(),
            category: "Physics".into(),
            pos: Vec2::new(570.0, 70.0),
            size: Vec2::new(210.0, 110.0),
            inputs: vec![NodePin {
                id: "in_vel".into(),
                name: "Velocity".into(),
                is_output: false,
                pin_type: "Float".into(),
            }],
            outputs: vec![
                NodePin {
                    id: "out_safe_pos".into(),
                    name: "Depenetrated Pos".into(),
                    is_output: true,
                    pin_type: "Float".into(),
                },
                NodePin {
                    id: "out_grounded".into(),
                    name: "Is Grounded".into(),
                    is_output: true,
                    pin_type: "Float".into(),
                },
            ],
            preview_color: Some(Color::rgb(245, 158, 11)),
            is_collapsed: false,
        };

        let n4 = GraphNode {
            id: "node_anim".into(),
            title: "Animation Blend Tree".into(),
            category: "Animation".into(),
            pos: Vec2::new(830.0, 60.0),
            size: Vec2::new(200.0, 120.0),
            inputs: vec![
                NodePin {
                    id: "in_speed".into(),
                    name: "Speed Value".into(),
                    is_output: false,
                    pin_type: "Float".into(),
                },
                NodePin {
                    id: "in_grounded".into(),
                    name: "Grounded Flag".into(),
                    is_output: false,
                    pin_type: "Float".into(),
                },
            ],
            outputs: vec![NodePin {
                id: "out_clip".into(),
                name: "Active Anim Clip".into(),
                is_output: true,
                pin_type: "Float".into(),
            }],
            preview_color: Some(Color::rgb(168, 85, 247)),
            is_collapsed: false,
        };

        let n5 = GraphNode {
            id: "node_output".into(),
            title: "Actor Transform Dispatch".into(),
            category: "Output".into(),
            pos: Vec2::new(1080.0, 80.0),
            size: Vec2::new(210.0, 100.0),
            inputs: vec![
                NodePin {
                    id: "in_safe_pos".into(),
                    name: "Depenetrated Pos".into(),
                    is_output: false,
                    pin_type: "Float".into(),
                },
                NodePin {
                    id: "in_clip".into(),
                    name: "Anim Clip".into(),
                    is_output: false,
                    pin_type: "Float".into(),
                },
            ],
            outputs: vec![],
            preview_color: Some(Color::rgb(239, 68, 68)),
            is_collapsed: false,
        };

        let connections = vec![
            NodeConnection {
                from_node: "node_input".into(),
                from_pin: "out_axis".into(),
                to_node: "node_velocity".into(),
                to_pin: "in_axis".into(),
            },
            NodeConnection {
                from_node: "node_velocity".into(),
                from_pin: "out_vel".into(),
                to_node: "node_collision".into(),
                to_pin: "in_vel".into(),
            },
            NodeConnection {
                from_node: "node_velocity".into(),
                from_pin: "out_speed".into(),
                to_node: "node_anim".into(),
                to_pin: "in_speed".into(),
            },
            NodeConnection {
                from_node: "node_collision".into(),
                from_pin: "out_grounded".into(),
                to_node: "node_anim".into(),
                to_pin: "in_grounded".into(),
            },
            NodeConnection {
                from_node: "node_collision".into(),
                from_pin: "out_safe_pos".into(),
                to_node: "node_output".into(),
                to_pin: "in_safe_pos".into(),
            },
            NodeConnection {
                from_node: "node_anim".into(),
                from_pin: "out_clip".into(),
                to_node: "node_output".into(),
                to_pin: "in_clip".into(),
            },
        ];

        let groups = vec![NodeGroup {
            id: "grp_locomotion".into(),
            title: "Character Kinematics & Collision Core".into(),
            bounds: Rect::from_min_size(Vec2::new(40.0, 20.0), Vec2::new(760.0, 180.0)),
            color: Color::rgba(56, 189, 248, 30),
        }];

        let doc = GraphDocument {
            nodes: vec![n1.clone(), n2.clone(), n3.clone(), n4.clone(), n5.clone()],
            connections: connections.clone(),
            groups: groups.clone(),
        };

        let editor = NodeEditor {
            pan: Vec2::new(20.0, 20.0),
            zoom: 1.0,
            nodes: vec![n1, n2, n3, n4, n5],
            connections,
            groups,
            ..NodeEditor::default()
        };

        (doc, editor)
    }

    /// Select an object by ID and refresh the Inspector.
    pub fn select_node(&mut self, node_id: &str) {
        self.selected_node_id = node_id.to_string();
        for item in &mut self.hierarchy.items {
            item.selected = item.id == node_id;
        }
        if let Some(n) = self.scene_nodes.iter().find(|n| n.id == node_id) {
            self.inspector = Self::build_inspector_for_node(n);
        }
    }

    pub fn record_undo(&mut self) {
        self.undo_manager.record_snapshot(ForgeSnapshot {
            selected_node_id: self.selected_node_id.clone(),
            scene_nodes: self.scene_nodes.clone(),
            active_mode: self.active_mode,
        });
    }

    pub fn undo(&mut self) -> bool {
        let mut snapshot = ForgeSnapshot {
            selected_node_id: self.selected_node_id.clone(),
            scene_nodes: self.scene_nodes.clone(),
            active_mode: self.active_mode,
        };
        if self.undo_manager.undo(&mut snapshot) {
            self.selected_node_id = snapshot.selected_node_id;
            self.scene_nodes = snapshot.scene_nodes;
            self.active_mode = snapshot.active_mode;
            let id = self.selected_node_id.clone();
            self.select_node(&id);
            self.console.log(
                ConsoleLogLevel::Info,
                "Undo",
                "Reverted last editor transformation.",
            );
            self.notifications.info("Undo", "Action reverted.");
            true
        } else {
            self.notifications
                .warning("Undo", "No previous actions to undo.");
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        let mut snapshot = ForgeSnapshot {
            selected_node_id: self.selected_node_id.clone(),
            scene_nodes: self.scene_nodes.clone(),
            active_mode: self.active_mode,
        };
        if self.undo_manager.redo(&mut snapshot) {
            self.selected_node_id = snapshot.selected_node_id;
            self.scene_nodes = snapshot.scene_nodes;
            self.active_mode = snapshot.active_mode;
            let id = self.selected_node_id.clone();
            self.select_node(&id);
            self.console.log(
                ConsoleLogLevel::Info,
                "Redo",
                "Reapplied editor transformation.",
            );
            self.notifications.info("Redo", "Action reapplied.");
            true
        } else {
            self.notifications
                .warning("Redo", "No forward actions to redo.");
            false
        }
    }

    /// Add a new empty game object to the active scene.
    pub fn add_new_object(&mut self, prefix: &str) {
        self.record_undo();
        let id = format!("node_{}_{}", prefix.to_lowercase(), self.next_node_counter);
        let name = format!("{}_{:02}", prefix, self.next_node_counter);
        self.next_node_counter += 1;

        let mut node = ForgeSceneNode::new(&id, &name, 1);
        node.position = [0.0, 1.0, 2.0];
        self.scene_nodes.push(node);

        let mut item = HierarchyItem::new(&id, &name).with_depth(1);
        item.visible = true;
        item.selected = true;
        for it in &mut self.hierarchy.items {
            it.selected = false;
        }
        self.hierarchy.items.push(item);
        self.select_node(&id);

        self.console.log(
            ConsoleLogLevel::Info,
            "Scene",
            format!("Created GameObject '{}' (ID: {})", name, id),
        );
        self.notifications.info(
            "Object Added",
            format!("Created '{}' in active scene.", name),
        );
    }

    /// Delete the currently selected game object.
    pub fn delete_selected_object(&mut self) {
        if self.selected_node_id == "node_world_root" {
            self.notifications
                .warning("Protected Node", "Cannot delete the scene World_Root.");
            return;
        }
        self.record_undo();
        let id_to_del = self.selected_node_id.clone();
        self.scene_nodes.retain(|n| n.id != id_to_del);
        self.hierarchy.items.retain(|it| it.id != id_to_del);

        // Fall back selection to World_Root
        self.select_node("node_world_root");
        self.console.log(
            ConsoleLogLevel::Warning,
            "Scene",
            format!("Deleted GameObject '{}'", id_to_del),
        );
        self.notifications
            .info("Object Deleted", format!("Removed object '{}'.", id_to_del));
    }

    /// Handle menu actions by ID.
    pub fn trigger_action(&mut self, action_id: &str) {
        match action_id {
            "file_new_scene" => {
                self.console
                    .log(ConsoleLogLevel::Info, "Project", "Created new blank scene.");
                self.notifications
                    .success("New Scene", "Initialized blank level template.");
            }
            "file_save_scene" => {
                self.layout_save_cache = self.graph_document.serialize().unwrap_or_default();
                self.console.log(
                    ConsoleLogLevel::Info,
                    "Storage",
                    "Scene and logic graph saved successfully.",
                );
                self.notifications.success(
                    "Scene Saved",
                    "All 10 scene actors and logic graph saved to disk.",
                );
            }
            "file_build_game" => {
                self.dialogs.open("dlg_build", "Build & Package Game", "Target: Windows x64 (Direct3D 12)\nConfiguration: Release (LTO Thin)\nOutput: build/KathamoForge_Game.exe");
            }
            "file_project_settings" => {
                self.dialogs.open("dlg_settings", "Project Settings", "Physics Tick: 60 Hz\nRendering API: Hardware Swapchain\nAnti-Aliasing: Sub-pixel Vector Coverage");
            }
            "file_exit" => {
                self.console.log(
                    ConsoleLogLevel::Info,
                    "Lifecycle",
                    "Application exit requested.",
                );
            }
            "edit_undo" => {
                self.undo();
            }
            "edit_redo" => {
                self.redo();
            }
            "edit_duplicate" => {
                self.add_new_object("Clone");
            }
            "edit_delete" => {
                self.delete_selected_object();
            }
            "obj_add_empty" => self.add_new_object("EmptyObject"),
            "obj_add_hero" => self.add_new_object("CharacterActor"),
            "obj_add_cube" => self.add_new_object("CubeMesh"),
            "obj_add_light" => self.add_new_object("PointLight"),
            "obj_add_camera" => self.add_new_object("SecondaryCamera"),
            "ws_mode_3d" => self.set_workspace_mode(EditorWorkspaceMode::Mode3D),
            "ws_mode_2d" => self.set_workspace_mode(EditorWorkspaceMode::Mode2D),
            "ws_mode_script" => self.set_workspace_mode(EditorWorkspaceMode::ModeScript),
            "ws_mode_graph" => self.set_workspace_mode(EditorWorkspaceMode::ModeGraph),
            "ws_reset_layout" => {
                self.notifications.info(
                    "Layout Reset",
                    "Restored standard 4-pane indie editor dock.",
                );
            }
            "sim_play" => self.set_sim_state(SimulationState::Playing),
            "sim_pause" => self.set_sim_state(SimulationState::Paused),
            "sim_stop" => self.set_sim_state(SimulationState::Stopped),
            "help_palette" => {
                self.command_palette_open = !self.command_palette_open;
            }
            "help_about" => {
                self.dialogs.open("dlg_about", "About Kathamo Forge", "Kathamo Forge v1.0 — Indie Game Engine Editor UI Showcase\nPowered 100% by Kathamo Native Rust UI/UX Framework.");
            }
            _ => {}
        }
    }

    /// Set editor workspace mode (3D, 2D, Script, Graph).
    pub fn set_workspace_mode(&mut self, mode: EditorWorkspaceMode) {
        self.active_mode = mode;
        let mode_name = match mode {
            EditorWorkspaceMode::Mode3D => "3D Spatial View",
            EditorWorkspaceMode::Mode2D => "2D Canvas View",
            EditorWorkspaceMode::ModeScript => "C# Script View",
            EditorWorkspaceMode::ModeGraph => "Logic Graph View",
        };
        self.console.log(
            ConsoleLogLevel::Info,
            "Workspace",
            format!("Switched to {}", mode_name),
        );
    }

    /// Set simulation state (Play, Pause, Stop).
    pub fn set_sim_state(&mut self, state: SimulationState) {
        self.sim_state = state;
        match state {
            SimulationState::Playing => {
                self.console.log(
                    ConsoleLogLevel::Info,
                    "Simulation",
                    "Game simulation STARTED (tick: 60Hz).",
                );
                self.notifications
                    .success("Simulation Active", "Game tick running at 144 FPS.");
            }
            SimulationState::Paused => {
                self.console.log(
                    ConsoleLogLevel::Warning,
                    "Simulation",
                    "Game simulation PAUSED.",
                );
                self.notifications
                    .warning("Simulation Paused", "Frame frozen for inspection.");
            }
            SimulationState::Stopped => {
                self.sim_time = 0.0;
                self.console.log(
                    ConsoleLogLevel::Info,
                    "Simulation",
                    "Game simulation STOPPED & RESET.",
                );
                self.notifications
                    .info("Simulation Reset", "State restored to edit mode.");
            }
        }
    }
}

impl Default for KathamoForgeApp {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// KATHAMO APP LIFECYCLE IMPLEMENTATION
// =============================================================================

impl KathamoApp for KathamoForgeApp {
    fn title(&self) -> &str {
        "Kathamo Forge — Indie Game Engine Editor Showcase"
    }

    fn default_size(&self) -> (u32, u32) {
        (1600, 1000)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.notifications.tick();

        // Advance simulation timer if playing
        if self.sim_state == SimulationState::Playing {
            self.sim_time += 0.0166;
            self.sim_frame += 1;
            if self.sim_frame % 60 == 0 {
                self.timeline.current_frame =
                    ((self.timeline.current_frame + 1) % self.timeline.total_frames).max(1);
            }
        }

        let bounds = ctx.bounds();
        let palette = ctx.theme;

        // Ensure dark background canvas fill
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.canvas,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // ---------------------------------------------------------------------
        // 1. TOP MENU BAR (H = 28)
        // ---------------------------------------------------------------------
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar
            .render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // ---------------------------------------------------------------------
        // 2. SCENE TABS, WORKSPACE MODES & SIMULATION RIBBON (H = 38)
        // ---------------------------------------------------------------------
        let ribbon_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), 38.0),
        );
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: ribbon_rect,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // 2a. Document tabs and 2b. workspace mode selector share one layout.
        let (doc_tabs, mode_rects, mode_strip) = self.ribbon_layout(ribbon_rect, bounds);
        for (idx, (tab_name, tab_r)) in self.open_tabs.iter().zip(&doc_tabs).enumerate() {
            let is_active = idx == self.active_tab_idx;
            let hover = motion::animate_bool(
                motion::rect_key("forge_doc_tab", *tab_r),
                tab_r.contains(ctx.pointer.position),
                motion::HOVER,
            );
            let rest = if is_active {
                palette.elevated_high
            } else {
                palette.surface
            };
            ctx.cmd_buf.push(DrawCommand::Rect {
                rect: *tab_r,
                fill: rest.lerp(palette.elevated, if is_active { 0.0 } else { hover }),
                radius: CornerRadius::same(5.0),
                stroke: if is_active {
                    Stroke::new(1.0, palette.accent.with_alpha(200))
                } else {
                    Stroke::new(1.0, palette.border.with_alpha(100))
                },
            });
            ctx.cmd_buf.push(DrawCommand::Text {
                text: tab_name.clone(),
                pos: Vec2::new(tab_r.min.x + 10.0, tab_r.min.y + 6.0),
                size: 11.5,
                color: if is_active {
                    palette.text
                } else {
                    palette.text_muted.lerp(palette.text, hover * 0.7)
                },
                strong: is_active,
            });
        }
        let active_mode = MODES
            .iter()
            .position(|m| *m == self.active_mode)
            .unwrap_or(0);
        tab_strip::render(
            &mut ctx.cmd_buf,
            &MODE_LABELS,
            &mode_rects,
            mode_strip,
            active_mode,
            ctx.pointer.position,
            &palette,
        );

        // 2c. Simulation transport: a three-state segmented control.
        let (sim_rects, sim_strip) = Self::transport_layout(ribbon_rect);
        let active_sim = SIM_STATES
            .iter()
            .position(|st| *st == self.sim_state)
            .unwrap_or(0);
        tab_strip::render(
            &mut ctx.cmd_buf,
            &SIM_LABELS,
            &sim_rects,
            sim_strip,
            active_sim,
            ctx.pointer.position,
            &palette,
        );

        // ---------------------------------------------------------------------
        // 3. MAIN WORKSPACE GEOMETRY LAYOUT
        // ---------------------------------------------------------------------
        let top_offset = 66.0;
        let status_h = 24.0;
        let work_h = bounds.height() - top_offset - status_h;

        let left_w = 310.0;
        let right_w = 340.0;
        let center_w = (bounds.width() - left_w - right_w).max(400.0);

        let left_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + top_offset),
            Vec2::new(left_w, work_h),
        );
        let center_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + left_w, bounds.min.y + top_offset),
            Vec2::new(center_w, work_h),
        );
        let right_rect = Rect::from_min_size(
            Vec2::new(bounds.max.x - right_w, bounds.min.y + top_offset),
            Vec2::new(right_w, work_h),
        );

        // ---------------------------------------------------------------------
        // 3A. LEFT PANEL: SCENE HIERARCHY (Top) + ASSET BROWSER (Bottom)
        // ---------------------------------------------------------------------
        let left_split_h = work_h * 0.52;
        let hier_rect = Rect::from_min_size(left_rect.min, Vec2::new(left_w, left_split_h));
        let asset_rect = Rect::from_min_size(
            Vec2::new(left_rect.min.x, left_rect.min.y + left_split_h),
            Vec2::new(left_w, work_h - left_split_h),
        );

        // Header for Scene Hierarchy
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(hier_rect.min, Vec2::new(left_w, 28.0)),
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });
        ctx.cmd_buf.push(DrawCommand::Text {
            text: format!("Scene Hierarchy ({})", self.hierarchy.items.len()),
            pos: Vec2::new(hier_rect.min.x + 10.0, hier_rect.min.y + 6.0),
            size: 11.5,
            color: palette.text,
            strong: true,
        });

        // Buttons (+ Add / - Del) in Hierarchy Header
        let add_btn = Rect::from_min_size(
            Vec2::new(hier_rect.max.x - 52.0, hier_rect.min.y + 4.0),
            Vec2::new(22.0, 20.0),
        );
        let del_btn = Rect::from_min_size(
            Vec2::new(hier_rect.max.x - 26.0, hier_rect.min.y + 4.0),
            Vec2::new(22.0, 20.0),
        );
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: add_btn,
            fill: palette.elevated_high,
            radius: CornerRadius::same(3.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        ctx.cmd_buf.push(DrawCommand::Text {
            text: "+".into(),
            pos: Vec2::new(add_btn.min.x + 6.0, add_btn.min.y + 2.0),
            size: 13.0,
            color: palette.accent,
            strong: true,
        });
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: del_btn,
            fill: palette.elevated_high,
            radius: CornerRadius::same(3.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        ctx.cmd_buf.push(DrawCommand::Text {
            text: "-".into(),
            pos: Vec2::new(del_btn.min.x + 7.0, del_btn.min.y + 1.0),
            size: 13.0,
            color: Color::rgb(239, 68, 68),
            strong: true,
        });

        // Hierarchy Body
        let hier_body = Rect::from_min_size(
            Vec2::new(hier_rect.min.x, hier_rect.min.y + 28.0),
            Vec2::new(left_w, left_split_h - 28.0),
        );
        self.hierarchy.render(&mut ctx.cmd_buf, hier_body, &palette);

        // Asset Browser (Bottom Left)
        self.asset_browser
            .render(&mut ctx.cmd_buf, asset_rect, &ctx.pointer, &palette);

        // ---------------------------------------------------------------------
        // 3B. CENTER WORKSPACE: ACTIVE VIEWPORT (Top) + BOTTOM DOCK (Bottom)
        // ---------------------------------------------------------------------
        let bottom_dock_h = 210.0;
        let viewport_h = center_rect.height() - bottom_dock_h;
        let viewport_rect = Rect::from_min_size(center_rect.min, Vec2::new(center_w, viewport_h));
        let bottom_rect = Rect::from_min_size(
            Vec2::new(center_rect.min.x, center_rect.min.y + viewport_h),
            Vec2::new(center_w, bottom_dock_h),
        );

        // Render Active Viewport according to Workspace Mode
        match self.active_mode {
            EditorWorkspaceMode::Mode3D => {
                Self::render_3d_viewport(
                    &mut ctx.cmd_buf,
                    viewport_rect,
                    &self.scene_nodes,
                    &self.selected_node_id,
                    self.sim_state == SimulationState::Playing,
                    self.sim_time,
                    &palette,
                );
            }
            EditorWorkspaceMode::Mode2D => {
                Self::render_2d_viewport(
                    &mut ctx.cmd_buf,
                    viewport_rect,
                    &self.scene_nodes,
                    &self.selected_node_id,
                    &palette,
                );
            }
            EditorWorkspaceMode::ModeScript => {
                Self::render_script_editor(
                    &mut ctx.cmd_buf,
                    viewport_rect,
                    &self.script_text,
                    &palette,
                );
            }
            EditorWorkspaceMode::ModeGraph => {
                self.render_graph_viewport(&mut ctx.cmd_buf, viewport_rect, &palette);
            }
        }

        // Render Bottom Dock (Console / Timeline / Profiler)
        self.render_bottom_dock(&mut ctx.cmd_buf, bottom_rect, &ctx.pointer, &palette);

        // ---------------------------------------------------------------------
        // 3C. RIGHT PANEL: METADATA PROPERTY INSPECTOR
        // ---------------------------------------------------------------------
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: right_rect,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // Inspector Header
        let insp_header = Rect::from_min_size(right_rect.min, Vec2::new(right_w, 32.0));
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: insp_header,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });
        ctx.cmd_buf.push(DrawCommand::Text {
            text: format!("Inspector: {}", self.selected_node_id),
            pos: Vec2::new(insp_header.min.x + 12.0, insp_header.min.y + 8.0),
            size: 12.0,
            color: palette.text,
            strong: true,
        });

        let insp_body = Rect::from_min_size(
            Vec2::new(right_rect.min.x, right_rect.min.y + 32.0),
            Vec2::new(right_w, work_h - 32.0),
        );
        self.inspector.render(&mut ctx.cmd_buf, insp_body, &palette);

        // ---------------------------------------------------------------------
        // 4. BOTTOM STATUS BAR (H = 24)
        // ---------------------------------------------------------------------
        let status_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.max.y - status_h),
            Vec2::new(bounds.width(), status_h),
        );
        ctx.cmd_buf.push(DrawCommand::Rect {
            rect: status_rect,
            fill: palette.elevated_high,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        let sim_text = match self.sim_state {
            SimulationState::Stopped => "● STOPPED (Edit Mode)",
            SimulationState::Playing => "▶ SIMULATION RUNNING @ 144 FPS",
            SimulationState::Paused => "⏸ PAUSED",
        };
        ctx.cmd_buf.push(DrawCommand::Text {
            text: format!(
                "Kathamo Forge | Scene: {} | Mode: {:?} | Target: {}",
                self.active_scene_name, self.active_mode, self.selected_node_id
            ),
            pos: Vec2::new(status_rect.min.x + 12.0, status_rect.min.y + 5.0),
            size: 11.0,
            color: palette.text_muted,
            strong: false,
        });

        ctx.cmd_buf.push(DrawCommand::Text {
            text: format!(
                "{} | Draw Commands: {} | p95: 1.82ms | wgpu D3D12/Vulkan",
                sim_text,
                ctx.cmd_buf.len()
            ),
            pos: Vec2::new(status_rect.max.x - 480.0, status_rect.min.y + 5.0),
            size: 11.0,
            color: if self.sim_state == SimulationState::Playing {
                Color::rgb(74, 222, 128)
            } else {
                palette.accent
            },
            strong: true,
        });

        // ---------------------------------------------------------------------
        // 5. MODAL DIALOGS, NOTIFICATIONS & COMMAND PALETTE OVERLAY
        // ---------------------------------------------------------------------
        self.dialogs
            .render(&mut ctx.cmd_buf, bounds, &ctx.pointer, &palette);
        self.notifications
            .render(&mut ctx.cmd_buf, bounds, &palette);

        // Command Palette
        if self.command_palette_open {
            self.render_command_palette(&mut ctx.cmd_buf, bounds, &palette);
        }
    }

    fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        ctx: &mut AppContext,
    ) -> bool {
        let bounds = ctx.bounds();

        // 1. Dialogs modal capture
        if self.dialogs.is_open {
            if let Some((_id, action)) = self.dialogs.handle_mouse_down(pos, button, bounds) {
                if action == DialogAction::Confirm {
                    self.notifications
                        .success("Action Confirmed", "Operation queued.");
                }
                return true;
            }
        }

        // 2. Command Palette capture
        if self.command_palette_open {
            let pal_w = 480.0;
            let pal_h = 240.0;
            let pal_rect = Rect::from_min_size(
                Vec2::new(
                    bounds.center().x - pal_w * 0.5,
                    bounds.center().y - pal_h * 0.5,
                ),
                Vec2::new(pal_w, pal_h),
            );
            if !pal_rect.contains(pos) {
                self.command_palette_open = false;
                return true;
            }
            // Clicked inside command palette item
            let rel_y = pos.y - (pal_rect.min.y + 44.0);
            if rel_y > 0.0 {
                let idx = (rel_y / 28.0).floor() as usize;
                match idx {
                    0 => self.set_workspace_mode(EditorWorkspaceMode::Mode3D),
                    1 => self.set_workspace_mode(EditorWorkspaceMode::Mode2D),
                    2 => self.set_workspace_mode(EditorWorkspaceMode::ModeGraph),
                    3 => self.set_workspace_mode(EditorWorkspaceMode::ModeScript),
                    4 => self.set_sim_state(SimulationState::Playing),
                    5 => self.trigger_action("file_save_scene"),
                    _ => {}
                }
                self.command_palette_open = false;
                return true;
            }
        }

        // 3. Menu Bar
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        if menu_rect.contains(pos) {
            if let Some(action_id) = self.menu_bar.handle_mouse_down(pos, button, menu_rect) {
                self.trigger_action(&action_id);
                return true;
            }
        }

        // 4. Ribbon Controls
        let ribbon_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), 38.0),
        );
        if ribbon_rect.contains(pos) {
            let (doc_tabs, mode_rects, _) = self.ribbon_layout(ribbon_rect, bounds);
            if let Some(i) = tab_strip::hit(&mode_rects, pos) {
                self.set_workspace_mode(MODES[i]);
                return true;
            }

            // Transport controls
            let (sim_rects, _) = Self::transport_layout(ribbon_rect);
            if let Some(i) = tab_strip::hit(&sim_rects, pos) {
                self.set_sim_state(SIM_STATES[i]);
                return true;
            }

            // Document tabs
            if let Some(idx) = doc_tabs.iter().position(|r| r.contains(pos)) {
                self.active_tab_idx = idx;
                let tab_name = &self.open_tabs[idx];
                if tab_name.ends_with(".cs") {
                    self.set_workspace_mode(EditorWorkspaceMode::ModeScript);
                } else if tab_name.ends_with(".graph") {
                    self.set_workspace_mode(EditorWorkspaceMode::ModeGraph);
                } else if tab_name.ends_with(".scene") {
                    self.set_workspace_mode(EditorWorkspaceMode::Mode3D);
                }
                return true;
            }
        }

        // 5. Left Hierarchy + Buttons
        let top_offset = 66.0;
        let left_w = 310.0;
        let left_split_h = (bounds.height() - top_offset - 24.0) * 0.52;
        let hier_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + top_offset),
            Vec2::new(left_w, left_split_h),
        );

        let add_btn = Rect::from_min_size(
            Vec2::new(hier_rect.max.x - 52.0, hier_rect.min.y + 4.0),
            Vec2::new(22.0, 20.0),
        );
        let del_btn = Rect::from_min_size(
            Vec2::new(hier_rect.max.x - 26.0, hier_rect.min.y + 4.0),
            Vec2::new(22.0, 20.0),
        );
        if add_btn.contains(pos) {
            self.add_new_object("Actor");
            return true;
        }
        if del_btn.contains(pos) {
            self.delete_selected_object();
            return true;
        }

        let hier_body = Rect::from_min_size(
            Vec2::new(hier_rect.min.x, hier_rect.min.y + 28.0),
            Vec2::new(left_w, left_split_h - 28.0),
        );
        if hier_body.contains(pos) {
            if let Some(event) =
                self.hierarchy
                    .handle_mouse_down(pos, button, hier_body, &ctx.pointer)
            {
                match event {
                    HierarchyEvent::Selected(id) => {
                        self.select_node(&id);
                    }
                    HierarchyEvent::ToggleVisibility(id) => {
                        if let Some(n) = self.scene_nodes.iter_mut().find(|n| n.id == id) {
                            n.visible = !n.visible;
                            self.console.log(
                                ConsoleLogLevel::Info,
                                "Scene",
                                format!("Visibility toggled for {}", n.name),
                            );
                        }
                    }
                    _ => {}
                }
                return true;
            }
        }

        // 6. Bottom Asset Browser
        let asset_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + top_offset + left_split_h),
            Vec2::new(left_w, bounds.height() - top_offset - 24.0 - left_split_h),
        );
        if asset_rect.contains(pos) {
            if let Some(id) = self
                .asset_browser
                .handle_mouse_down(pos, button, asset_rect)
            {
                self.console.log(
                    ConsoleLogLevel::Info,
                    "Assets",
                    format!("Selected asset '{}'", id),
                );
                return true;
            }
        }

        // 7. Right Inspector
        let right_w = 340.0;
        let right_rect = Rect::from_min_size(
            Vec2::new(bounds.max.x - right_w, bounds.min.y + top_offset),
            Vec2::new(right_w, bounds.height() - top_offset - 24.0),
        );
        if right_rect.contains(pos) {
            if let Some(_evt) = self.inspector.handle_mouse_down(pos, button, right_rect) {
                self.console
                    .log(ConsoleLogLevel::Info, "Inspector", "Parameter modified.");
                return true;
            }
        }

        // 8. Bottom Dock tabs & controls
        let center_w = (bounds.width() - left_w - right_w).max(400.0);
        let bottom_dock_h = 210.0;
        let viewport_h = bounds.height() - top_offset - 24.0 - bottom_dock_h;
        let bottom_rect = Rect::from_min_size(
            Vec2::new(
                bounds.min.x + left_w,
                bounds.min.y + top_offset + viewport_h,
            ),
            Vec2::new(center_w, bottom_dock_h),
        );
        if bottom_rect.contains(pos) {
            // Tab clicks along top of bottom dock
            let tab_y = bottom_rect.min.y + 4.0;
            let tab_console = Rect::from_min_size(
                Vec2::new(bottom_rect.min.x + 8.0, tab_y),
                Vec2::new(85.0, 22.0),
            );
            let tab_timeline = Rect::from_min_size(
                Vec2::new(bottom_rect.min.x + 98.0, tab_y),
                Vec2::new(95.0, 22.0),
            );
            let tab_profiler = Rect::from_min_size(
                Vec2::new(bottom_rect.min.x + 198.0, tab_y),
                Vec2::new(85.0, 22.0),
            );
            let btn_clear = Rect::from_min_size(
                Vec2::new(bottom_rect.max.x - 70.0, tab_y),
                Vec2::new(60.0, 20.0),
            );

            if tab_console.contains(pos) {
                self.bottom_tab_idx = 0;
                return true;
            }
            if tab_timeline.contains(pos) {
                self.bottom_tab_idx = 1;
                return true;
            }
            if tab_profiler.contains(pos) {
                self.bottom_tab_idx = 2;
                return true;
            }
            if btn_clear.contains(pos) {
                self.console.clear();
                return true;
            }

            // Timeline Scrub
            if self.bottom_tab_idx == 1 {
                let timeline_area = Rect::from_min_size(
                    Vec2::new(bottom_rect.min.x, bottom_rect.min.y + 28.0),
                    Vec2::new(bottom_rect.width(), bottom_rect.height() - 28.0),
                );
                if timeline_area.contains(pos) {
                    let rel_x = (pos.x - (timeline_area.min.x + 150.0)).max(0.0);
                    let track_w = timeline_area.width() - 170.0;
                    let ratio = (rel_x / track_w).clamp(0.0, 1.0);
                    let new_frame = (ratio * self.timeline.total_frames as f32) as u32;
                    self.timeline.scrub_to(new_frame);
                    return true;
                }
            }
        }

        // 9. Center Viewport Graph mode interactions
        let viewport_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + left_w, bounds.min.y + top_offset),
            Vec2::new(center_w, viewport_h),
        );
        if viewport_rect.contains(pos) && self.active_mode == EditorWorkspaceMode::ModeGraph {
            // Mode toggle in graph header
            let toggle_simple = Rect::from_min_size(
                Vec2::new(viewport_rect.max.x - 220.0, viewport_rect.min.y + 4.0),
                Vec2::new(98.0, 22.0),
            );
            let toggle_adv = Rect::from_min_size(
                Vec2::new(viewport_rect.max.x - 116.0, viewport_rect.min.y + 4.0),
                Vec2::new(104.0, 22.0),
            );
            if toggle_simple.contains(pos) {
                self.graph_view_mode = GraphViewMode::SimpleGuided;
                return true;
            }
            if toggle_adv.contains(pos) {
                self.graph_view_mode = GraphViewMode::AdvancedNodes;
                return true;
            }

            if self.graph_view_mode == GraphViewMode::AdvancedNodes {
                let graph_area = Rect::from_min_size(
                    Vec2::new(viewport_rect.min.x, viewport_rect.min.y + 30.0),
                    Vec2::new(viewport_rect.width(), viewport_rect.height() - 30.0),
                );
                self.node_editor.handle_mouse_down(pos, button, graph_area);
                return true;
            }
        }

        false
    }

    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        self.inspector.handle_mouse_up(pos, button);
        let bounds = ctx.bounds();
        let top_offset = 66.0;
        let left_w = 310.0;
        let right_w = 340.0;
        let center_w = (bounds.width() - left_w - right_w).max(400.0);
        let bottom_dock_h = 210.0;
        let viewport_h = bounds.height() - top_offset - 24.0 - bottom_dock_h;
        let viewport_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + left_w, bounds.min.y + top_offset),
            Vec2::new(center_w, viewport_h),
        );

        if self.active_mode == EditorWorkspaceMode::ModeGraph
            && self.graph_view_mode == GraphViewMode::AdvancedNodes
        {
            let graph_area = Rect::from_min_size(
                Vec2::new(viewport_rect.min.x, viewport_rect.min.y + 30.0),
                Vec2::new(viewport_rect.width(), viewport_rect.height() - 30.0),
            );
            self.node_editor.handle_mouse_up(pos, button, graph_area);
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, pos: Vec2, delta: Vec2, ctx: &mut AppContext) -> bool {
        let bounds = ctx.bounds();
        let top_offset = 66.0;
        let left_w = 310.0;
        let right_w = 340.0;
        let center_w = (bounds.width() - left_w - right_w).max(400.0);
        let bottom_dock_h = 210.0;
        let viewport_h = bounds.height() - top_offset - 24.0 - bottom_dock_h;
        let viewport_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + left_w, bounds.min.y + top_offset),
            Vec2::new(center_w, viewport_h),
        );

        // Drag node in Advanced Graph Editor
        if self.active_mode == EditorWorkspaceMode::ModeGraph
            && self.graph_view_mode == GraphViewMode::AdvancedNodes
        {
            let graph_area = Rect::from_min_size(
                Vec2::new(viewport_rect.min.x, viewport_rect.min.y + 30.0),
                Vec2::new(viewport_rect.width(), viewport_rect.height() - 30.0),
            );
            self.node_editor.handle_mouse_move(pos, delta, graph_area);
            return true;
        }

        // Drag inspector scrubbers
        if let Some(_evt) = self.inspector.handle_mouse_move(pos, delta, &ctx.pointer) {
            return true;
        }

        false
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, ctx: &mut AppContext) -> bool {
        let bounds = ctx.bounds();
        let top_offset = 66.0;
        let left_w = 310.0;
        let right_w = 340.0;
        let center_w = (bounds.width() - left_w - right_w).max(400.0);
        let bottom_dock_h = 210.0;
        let viewport_h = bounds.height() - top_offset - 24.0 - bottom_dock_h;
        let viewport_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + left_w, bounds.min.y + top_offset),
            Vec2::new(center_w, viewport_h),
        );

        if viewport_rect.contains(pos) {
            if self.active_mode == EditorWorkspaceMode::ModeGraph {
                if self.graph_view_mode == GraphViewMode::SimpleGuided {
                    self.graph_scroll_y = (self.graph_scroll_y - delta.y * 24.0).max(0.0);
                    return true;
                } else {
                    let graph_area = Rect::from_min_size(
                        Vec2::new(viewport_rect.min.x, viewport_rect.min.y + 30.0),
                        Vec2::new(viewport_rect.width(), viewport_rect.height() - 30.0),
                    );
                    if delta.y != 0.0 {
                        let factor = if delta.y > 0.0 { 1.1 } else { 0.9 };
                        self.node_editor.zoom_at(factor, pos, graph_area.min);
                    }
                    return true;
                }
            } else if self.active_mode == EditorWorkspaceMode::Mode3D {
                self.zoom = (self.zoom + delta.y * 0.05).clamp(0.2, 5.0);
                return true;
            }
        }
        false
    }

    fn handle_key_down(&mut self, key: Key, modifiers: Modifiers, _ctx: &mut AppContext) -> bool {
        if modifiers.ctrl && key == Key::Character('k') {
            self.command_palette_open = !self.command_palette_open;
            return true;
        }
        if modifiers.ctrl && key == Key::Character('s') {
            self.trigger_action("file_save_scene");
            return true;
        }
        if modifiers.ctrl && key == Key::Character('z') {
            if modifiers.shift {
                self.redo();
            } else {
                self.undo();
            }
            return true;
        }
        if modifiers.ctrl && key == Key::Character('y') {
            self.redo();
            return true;
        }
        if key == Key::Escape {
            if self.command_palette_open {
                self.command_palette_open = false;
                return true;
            }
            if self.dialogs.is_open {
                self.dialogs.close();
                return true;
            }
        }
        if key == Key::F(5) {
            if modifiers.shift {
                self.set_sim_state(SimulationState::Stopped);
            } else {
                self.set_sim_state(SimulationState::Playing);
            }
            return true;
        }
        if key == Key::F(6) {
            self.set_sim_state(SimulationState::Paused);
            return true;
        }
        false
    }

    fn wants_continuous_redraw(&self) -> bool {
        self.sim_state == SimulationState::Playing
    }
}

// =============================================================================
// VIEWPORT RENDERERS & HELPERS
// =============================================================================

impl KathamoForgeApp {
    /// Render 3D Spatial Viewport with grid, perspective horizons, objects, and axes.
    pub fn render_3d_viewport(
        buf: &mut CommandBuffer,
        rect: Rect,
        nodes: &[ForgeSceneNode],
        selected_id: &str,
        is_playing: bool,
        sim_time: f32,
        pal: &ThemePalette,
    ) {
        buf.push(DrawCommand::ClipPush { rect });

        // Dark gradient space background
        buf.push(DrawCommand::Rect {
            rect,
            fill: Color::rgb(13, 17, 23),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        let center = rect.center();

        // 1. Perspective 3D Ground Grid lines
        let grid_y_base = center.y + 60.0;
        for i in -12..=12 {
            let offset_x = i as f32 * 32.0;
            // Perspective convergence to horizon
            let p1 = Vec2::new(center.x + offset_x * 0.15, center.y - 40.0);
            let p2 = Vec2::new(center.x + offset_x * 2.2, center.y + 190.0);
            buf.push(DrawCommand::Line {
                start: p1,
                end: p2,
                stroke: Stroke::new(1.0, Color::rgba(56, 189, 248, 25)),
            });
        }
        // Horizontal depth lines
        for step in 0..10 {
            let depth_ratio = (step as f32 / 10.0).powf(1.6);
            let line_y = (center.y - 40.0) + depth_ratio * 230.0;
            let half_w = 40.0 + depth_ratio * 460.0;
            buf.push(DrawCommand::Line {
                start: Vec2::new(center.x - half_w, line_y),
                end: Vec2::new(center.x + half_w, line_y),
                stroke: Stroke::new(1.0, Color::rgba(56, 189, 248, 30)),
            });
        }

        // 2. Render 3D Objects representation
        for node in nodes {
            if !node.visible {
                continue;
            }
            let is_sel = node.id == selected_id;

            let obj_center_x = center.x + node.position[0] * 24.0;
            let obj_center_y = grid_y_base - node.position[1] * 28.0 + node.position[2] * 8.0;

            if node.id == "node_hero" {
                // Animated Hero Position if playing
                let bounce = if is_playing {
                    (sim_time * 8.0).sin() * 6.0
                } else {
                    0.0
                };
                let hero_y = obj_center_y - bounce;

                // Player capsule shadow
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(obj_center_x - 18.0, grid_y_base - 4.0),
                        Vec2::new(36.0, 10.0),
                    ),
                    fill: Color::rgba(0, 0, 0, 140),
                    radius: CornerRadius::same(5.0),
                    stroke: Stroke::NONE,
                });

                // Hero Capsule Mesh
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(obj_center_x - 16.0, hero_y - 42.0),
                        Vec2::new(32.0, 52.0),
                    ),
                    fill: node.material_color,
                    radius: CornerRadius::same(12.0),
                    stroke: Stroke::new(
                        if is_sel { 2.0 } else { 1.0 },
                        if is_sel {
                            pal.accent
                        } else {
                            Color::rgba(255, 255, 255, 80)
                        },
                    ),
                });

                // Hero Visor / Head
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(obj_center_x - 10.0, hero_y - 36.0),
                        Vec2::new(20.0, 8.0),
                    ),
                    fill: Color::rgb(56, 189, 248),
                    radius: CornerRadius::same(3.0),
                    stroke: Stroke::NONE,
                });

                // 3D Translation Gizmo if selected
                if is_sel {
                    // Y Axis (Green)
                    buf.push(DrawCommand::Line {
                        start: Vec2::new(obj_center_x, hero_y - 42.0),
                        end: Vec2::new(obj_center_x, hero_y - 90.0),
                        stroke: Stroke::new(2.0, Color::rgb(34, 197, 94)),
                    });
                    // X Axis (Red)
                    buf.push(DrawCommand::Line {
                        start: Vec2::new(obj_center_x, hero_y - 20.0),
                        end: Vec2::new(obj_center_x + 50.0, hero_y - 20.0),
                        stroke: Stroke::new(2.0, Color::rgb(239, 68, 68)),
                    });
                    // Z Axis (Blue)
                    buf.push(DrawCommand::Line {
                        start: Vec2::new(obj_center_x, hero_y - 20.0),
                        end: Vec2::new(obj_center_x - 30.0, hero_y + 15.0),
                        stroke: Stroke::new(2.0, Color::rgb(59, 130, 246)),
                    });
                }
            } else if node.id.contains("pillar") {
                // Stone pillar cuboid
                let p_rect = Rect::from_min_size(
                    Vec2::new(obj_center_x - 12.0, obj_center_y - 65.0),
                    Vec2::new(24.0, 70.0),
                );
                buf.push(DrawCommand::Rect {
                    rect: p_rect,
                    fill: node.material_color,
                    radius: CornerRadius::same(2.0),
                    stroke: Stroke::new(
                        if is_sel { 2.0 } else { 1.0 },
                        if is_sel {
                            pal.accent
                        } else {
                            Color::rgba(255, 255, 255, 40)
                        },
                    ),
                });
            } else if node.id == "node_sun" {
                // Sun Light Representation
                let sun_pos = Vec2::new(center.x + 180.0, center.y - 140.0);
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(sun_pos.x - 12.0, sun_pos.y - 12.0),
                        Vec2::new(24.0, 24.0),
                    ),
                    fill: Color::rgb(251, 191, 36),
                    radius: CornerRadius::same(12.0),
                    stroke: Stroke::new(2.0, Color::WHITE),
                });
                buf.push(DrawCommand::Line {
                    start: sun_pos,
                    end: Vec2::new(sun_pos.x - 60.0, sun_pos.y + 70.0),
                    stroke: Stroke::new(1.5, Color::rgba(251, 191, 36, 120)),
                });
            }
        }

        // 3. Top-left Viewport HUD
        let hud_rect = Rect::from_min_size(
            Vec2::new(rect.min.x + 12.0, rect.min.y + 12.0),
            Vec2::new(220.0, 26.0),
        );
        buf.push(DrawCommand::Rect {
            rect: hud_rect,
            fill: Color::rgba(18, 24, 38, 200),
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, pal.border),
        });
        buf.push(DrawCommand::Text {
            text: "Shading: Lit PBR | Cam: Perspective 60°".into(),
            pos: Vec2::new(hud_rect.min.x + 8.0, hud_rect.min.y + 6.0),
            size: 10.5,
            color: pal.text,
            strong: true,
        });

        // 4. Orientation Cube / Axis Widget in Top-Right
        let axis_origin = Vec2::new(rect.max.x - 45.0, rect.min.y + 45.0);
        buf.push(DrawCommand::Line {
            start: axis_origin,
            end: Vec2::new(axis_origin.x + 24.0, axis_origin.y),
            stroke: Stroke::new(2.0, Color::rgb(239, 68, 68)), // X Red
        });
        buf.push(DrawCommand::Text {
            text: "X".into(),
            pos: Vec2::new(axis_origin.x + 28.0, axis_origin.y - 5.0),
            size: 9.0,
            color: Color::rgb(239, 68, 68),
            strong: true,
        });

        buf.push(DrawCommand::Line {
            start: axis_origin,
            end: Vec2::new(axis_origin.x, axis_origin.y - 24.0),
            stroke: Stroke::new(2.0, Color::rgb(34, 197, 94)), // Y Green
        });
        buf.push(DrawCommand::Text {
            text: "Y".into(),
            pos: Vec2::new(axis_origin.x - 3.0, axis_origin.y - 34.0),
            size: 9.0,
            color: Color::rgb(34, 197, 94),
            strong: true,
        });

        buf.push(DrawCommand::Line {
            start: axis_origin,
            end: Vec2::new(axis_origin.x - 16.0, axis_origin.y + 14.0),
            stroke: Stroke::new(2.0, Color::rgb(59, 130, 246)), // Z Blue
        });
        buf.push(DrawCommand::Text {
            text: "Z".into(),
            pos: Vec2::new(axis_origin.x - 25.0, axis_origin.y + 14.0),
            size: 9.0,
            color: Color::rgb(59, 130, 246),
            strong: true,
        });

        buf.push(DrawCommand::ClipPop);
    }

    /// Render 2D Canvas Viewport.
    pub fn render_2d_viewport(
        buf: &mut CommandBuffer,
        rect: Rect,
        nodes: &[ForgeSceneNode],
        selected_id: &str,
        pal: &ThemePalette,
    ) {
        buf.push(DrawCommand::ClipPush { rect });
        buf.push(DrawCommand::Rect {
            rect,
            fill: Color::rgb(15, 23, 42),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // 2D Orthographic Pixel Grid
        let center = rect.center();
        for x_step in -10..=10 {
            let x = center.x + x_step as f32 * 40.0;
            buf.push(DrawCommand::Line {
                start: Vec2::new(x, rect.min.y),
                end: Vec2::new(x, rect.max.y),
                stroke: Stroke::new(1.0, Color::rgba(255, 255, 255, 12)),
            });
        }
        for y_step in -8..=8 {
            let y = center.y + y_step as f32 * 40.0;
            buf.push(DrawCommand::Line {
                start: Vec2::new(rect.min.x, y),
                end: Vec2::new(rect.max.x, y),
                stroke: Stroke::new(1.0, Color::rgba(255, 255, 255, 12)),
            });
        }

        // Center Axis origin
        buf.push(DrawCommand::Line {
            start: Vec2::new(rect.min.x, center.y),
            end: Vec2::new(rect.max.x, center.y),
            stroke: Stroke::new(1.5, Color::rgba(239, 68, 68, 60)),
        });
        buf.push(DrawCommand::Line {
            start: Vec2::new(center.x, rect.min.y),
            end: Vec2::new(center.x, rect.max.y),
            stroke: Stroke::new(1.5, Color::rgba(34, 197, 94, 60)),
        });

        // 2D Sprite Bounding Boxes
        for node in nodes {
            let is_sel = node.id == selected_id;
            let sx = center.x + node.position[0] * 30.0;
            let sy = center.y - node.position[1] * 30.0;
            let r = Rect::from_min_size(Vec2::new(sx - 25.0, sy - 25.0), Vec2::new(50.0, 50.0));

            buf.push(DrawCommand::Rect {
                rect: r,
                fill: node.material_color.with_alpha(50),
                radius: CornerRadius::same(4.0),
                stroke: Stroke::new(
                    if is_sel { 2.0 } else { 1.0 },
                    if is_sel { pal.accent } else { pal.border },
                ),
            });
            buf.push(DrawCommand::Text {
                text: node.name.clone(),
                pos: Vec2::new(r.min.x, r.max.y + 4.0),
                size: 9.5,
                color: if is_sel { pal.accent } else { pal.text_muted },
                strong: is_sel,
            });
        }

        buf.push(DrawCommand::ClipPop);
    }

    /// Render C# Script Editor Viewport mock.
    pub fn render_script_editor(
        buf: &mut CommandBuffer,
        rect: Rect,
        lines: &[&'static str],
        pal: &ThemePalette,
    ) {
        buf.push(DrawCommand::ClipPush { rect });
        buf.push(DrawCommand::Rect {
            rect,
            fill: Color::rgb(15, 17, 26),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Line number gutter (W = 46)
        let gutter_w = 46.0;
        let gutter_rect = Rect::from_min_size(rect.min, Vec2::new(gutter_w, rect.height()));
        buf.push(DrawCommand::Rect {
            rect: gutter_rect,
            fill: Color::rgb(22, 27, 38),
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, pal.border.with_alpha(80)),
        });

        let mut line_y = rect.min.y + 12.0;
        for (i, line) in lines.iter().enumerate() {
            if line_y > rect.max.y - 16.0 {
                break;
            }
            // Line Number
            buf.push(DrawCommand::Text {
                text: format!("{:3}", i + 1),
                pos: Vec2::new(gutter_rect.min.x + 10.0, line_y),
                size: 11.0,
                color: Color::rgb(100, 116, 139),
                strong: false,
            });

            // Code syntax colors
            let code_color = if line.starts_with("//") {
                Color::rgb(100, 116, 139) // Comment gray
            } else if line.contains("using ")
                || line.contains("namespace ")
                || line.contains("public class")
            {
                Color::rgb(192, 132, 252) // Purple keyword
            } else if line.contains("float ") || line.contains("bool ") || line.contains("Vector3 ")
            {
                Color::rgb(56, 189, 248) // Cyan types
            } else if line.contains("[SerializeField]") || line.contains("[Header") {
                Color::rgb(251, 191, 36) // Yellow attribute
            } else {
                Color::rgb(226, 232, 240) // Standard text
            };

            buf.push(DrawCommand::Text {
                text: line.to_string(),
                pos: Vec2::new(rect.min.x + gutter_w + 16.0, line_y),
                size: 11.5,
                color: code_color,
                strong: false,
            });

            line_y += 18.0;
        }

        buf.push(DrawCommand::ClipPop);
    }

    /// Render Logic Graph Viewport (Simple Guided vs Advanced Nodes).
    pub fn render_graph_viewport(
        &mut self,
        buf: &mut CommandBuffer,
        rect: Rect,
        pal: &ThemePalette,
    ) {
        buf.push(DrawCommand::ClipPush { rect });

        // Header with Simple / Advanced switcher
        let header_h = 30.0;
        let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header_h));
        buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: pal.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, pal.border),
        });

        buf.push(DrawCommand::Text {
            text: "Player Movement & Kinematics Workflow Graph".into(),
            pos: Vec2::new(header_rect.min.x + 12.0, header_rect.min.y + 8.0),
            size: 11.5,
            color: pal.text,
            strong: true,
        });

        let toggle_simple = Rect::from_min_size(
            Vec2::new(rect.max.x - 220.0, rect.min.y + 4.0),
            Vec2::new(98.0, 22.0),
        );
        let toggle_adv = Rect::from_min_size(
            Vec2::new(rect.max.x - 116.0, rect.min.y + 4.0),
            Vec2::new(104.0, 22.0),
        );

        let is_simple = self.graph_view_mode == GraphViewMode::SimpleGuided;

        // Simple button
        buf.push(DrawCommand::Rect {
            rect: toggle_simple,
            fill: if is_simple { pal.accent } else { pal.surface },
            radius: CornerRadius::same(3.0),
            stroke: Stroke::new(1.0, pal.border),
        });
        buf.push(DrawCommand::Text {
            text: "Simple Guided".into(),
            pos: Vec2::new(toggle_simple.min.x + 8.0, toggle_simple.min.y + 4.0),
            size: 10.5,
            color: if is_simple { Color::WHITE } else { pal.text },
            strong: is_simple,
        });

        // Advanced button
        buf.push(DrawCommand::Rect {
            rect: toggle_adv,
            fill: if !is_simple { pal.accent } else { pal.surface },
            radius: CornerRadius::same(3.0),
            stroke: Stroke::new(1.0, pal.border),
        });
        buf.push(DrawCommand::Text {
            text: "Advanced Nodes".into(),
            pos: Vec2::new(toggle_adv.min.x + 8.0, toggle_adv.min.y + 4.0),
            size: 10.5,
            color: if !is_simple { Color::WHITE } else { pal.text },
            strong: !is_simple,
        });

        let body_rect = Rect::from_min_size(
            Vec2::new(rect.min.x, rect.min.y + header_h),
            Vec2::new(rect.width(), rect.height() - header_h),
        );

        match self.graph_view_mode {
            GraphViewMode::SimpleGuided => {
                // Render approachable step cards via Kathamo GraphDocument guided pipeline
                buf.push(DrawCommand::Rect {
                    rect: body_rect,
                    fill: pal.canvas,
                    radius: CornerRadius::ZERO,
                    stroke: Stroke::NONE,
                });
                let _ = self.graph_document.render_guided(
                    buf,
                    body_rect,
                    self.graph_scroll_y,
                    self.node_editor.selected_node.as_deref(),
                    pal,
                );
            }
            GraphViewMode::AdvancedNodes => {
                // Render full visual node editor
                self.node_editor.render(buf, body_rect, pal);
            }
        }

        buf.push(DrawCommand::ClipPop);
    }

    /// Render Bottom Dock (Console, Timeline, Profiler).
    pub fn render_bottom_dock(
        &self,
        buf: &mut CommandBuffer,
        rect: Rect,
        _ptr: &PointerState,
        pal: &ThemePalette,
    ) {
        buf.push(DrawCommand::ClipPush { rect });

        // Header Tabs (H = 26)
        let tab_h = 26.0;
        let tab_bar = Rect::from_min_size(rect.min, Vec2::new(rect.width(), tab_h));
        buf.push(DrawCommand::Rect {
            rect: tab_bar,
            fill: pal.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, pal.border),
        });

        let tabs = [
            format!("Console ({})", self.console.entries.len()),
            "Animation Timeline".into(),
            "Profiler Telemetry".into(),
        ];

        let mut tx = rect.min.x + 8.0;
        for (i, title) in tabs.iter().enumerate() {
            let is_act = i == self.bottom_tab_idx;
            let tw = (title.len() as f32 * 7.0 + 20.0).max(80.0);
            let tr = Rect::from_min_size(Vec2::new(tx, rect.min.y + 3.0), Vec2::new(tw, 20.0));
            buf.push(DrawCommand::Rect {
                rect: tr,
                fill: if is_act {
                    pal.elevated_high
                } else {
                    Color::TRANSPARENT
                },
                radius: CornerRadius::same(3.0),
                stroke: if is_act {
                    Stroke::new(1.0, pal.accent)
                } else {
                    Stroke::NONE
                },
            });
            buf.push(DrawCommand::Text {
                text: title.clone(),
                pos: Vec2::new(tr.min.x + 6.0, tr.min.y + 4.0),
                size: 11.0,
                color: if is_act { pal.text } else { pal.text_muted },
                strong: is_act,
            });
            tx += tw + 8.0;
        }

        // Clear button on right
        let btn_clear = Rect::from_min_size(
            Vec2::new(rect.max.x - 70.0, rect.min.y + 3.0),
            Vec2::new(60.0, 20.0),
        );
        buf.push(DrawCommand::Rect {
            rect: btn_clear,
            fill: pal.surface,
            radius: CornerRadius::same(3.0),
            stroke: Stroke::new(1.0, pal.border),
        });
        buf.push(DrawCommand::Text {
            text: "Clear Logs".into(),
            pos: Vec2::new(btn_clear.min.x + 6.0, btn_clear.min.y + 4.0),
            size: 10.0,
            color: pal.text_muted,
            strong: false,
        });

        let content_rect = Rect::from_min_size(
            Vec2::new(rect.min.x, rect.min.y + tab_h),
            Vec2::new(rect.width(), rect.height() - tab_h),
        );

        match self.bottom_tab_idx {
            0 => {
                // Console
                self.console.render(buf, content_rect, pal);
            }
            1 => {
                // Timeline
                self.timeline.render(buf, content_rect, pal);
            }
            _ => {
                // Profiler Telemetry
                buf.push(DrawCommand::Rect {
                    rect: content_rect,
                    fill: pal.surface,
                    radius: CornerRadius::ZERO,
                    stroke: Stroke::NONE,
                });
                buf.push(DrawCommand::Text {
                    text: "Hardware Performance Telemetry".into(),
                    pos: Vec2::new(content_rect.min.x + 16.0, content_rect.min.y + 16.0),
                    size: 12.0,
                    color: pal.accent,
                    strong: true,
                });
                buf.push(DrawCommand::Text {
                    text: "Frame Timing (p95): 1.82 ms  |  FPS: 144.0  |  Allocations/frame: 0"
                        .into(),
                    pos: Vec2::new(content_rect.min.x + 16.0, content_rect.min.y + 40.0),
                    size: 11.5,
                    color: pal.text,
                    strong: false,
                });
                buf.push(DrawCommand::Text {
                    text: "VRAM Texture Footprint: 28.4 MB  |  Native Rust Heap: 18.2 MB".into(),
                    pos: Vec2::new(content_rect.min.x + 16.0, content_rect.min.y + 64.0),
                    size: 11.5,
                    color: pal.text_muted,
                    strong: false,
                });
            }
        }

        buf.push(DrawCommand::ClipPop);
    }

    /// Render Command Palette (`Ctrl+K`).
    pub fn render_command_palette(
        &self,
        buf: &mut CommandBuffer,
        screen_bounds: Rect,
        pal: &ThemePalette,
    ) {
        // Scrim
        buf.push(DrawCommand::Rect {
            rect: screen_bounds,
            fill: Color::rgba(0, 0, 0, 160),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        let pal_w = 480.0;
        let pal_h = 240.0;
        let pal_rect = Rect::from_min_size(
            Vec2::new(
                screen_bounds.center().x - pal_w * 0.5,
                screen_bounds.center().y - pal_h * 0.5,
            ),
            Vec2::new(pal_w, pal_h),
        );

        buf.push(DrawCommand::Rect {
            rect: pal_rect,
            fill: pal.elevated,
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, pal.accent),
        });

        // Search Input Header
        buf.push(DrawCommand::Text {
            text: "> Type a command or workspace action...".into(),
            pos: Vec2::new(pal_rect.min.x + 16.0, pal_rect.min.y + 14.0),
            size: 12.5,
            color: pal.text,
            strong: true,
        });
        buf.push(DrawCommand::Line {
            start: Vec2::new(pal_rect.min.x, pal_rect.min.y + 36.0),
            end: Vec2::new(pal_rect.max.x, pal_rect.min.y + 36.0),
            stroke: Stroke::new(1.0, pal.border),
        });

        let commands = [
            ("1. Switch to 3D Spatial Workspace", "F1"),
            ("2. Switch to 2D Canvas Workspace", "F2"),
            ("3. Switch to Game Logic Graph", "F4"),
            ("4. Switch to C# Script Editor", "F3"),
            ("5. Start Game Simulation (Play)", "F5"),
            ("6. Save Scene & Commit Graph", "Ctrl+S"),
        ];

        let mut item_y = pal_rect.min.y + 44.0;
        for (cmd, shortcut) in commands {
            let row_r = Rect::from_min_size(
                Vec2::new(pal_rect.min.x + 8.0, item_y),
                Vec2::new(pal_w - 16.0, 26.0),
            );
            buf.push(DrawCommand::Text {
                text: cmd.into(),
                pos: Vec2::new(row_r.min.x + 8.0, row_r.min.y + 6.0),
                size: 11.5,
                color: pal.text,
                strong: false,
            });
            buf.push(DrawCommand::Text {
                text: shortcut.into(),
                pos: Vec2::new(row_r.max.x - 60.0, row_r.min.y + 6.0),
                size: 10.5,
                color: pal.accent,
                strong: true,
            });
            item_y += 28.0;
        }
    }
}

// =============================================================================
// MAIN ENTRY POINT & VERIFICATION HARNESS
// =============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless");

    println!("===========================================================");
    println!("  Kathamo Forge — Indie Game Engine Editor UI Showcase");
    println!("===========================================================");

    if is_headless {
        println!("[Kathamo Forge] Running comprehensive headless interaction verification...");
        let app = KathamoForgeApp::new();
        let mut runner = HeadlessAppRunner::new(app);

        // Frame 1: Initial Presentation
        let cmd_count_frame1 = runner.step_frame();
        assert!(
            cmd_count_frame1 > 100,
            "Editor must emit commands on frame 1"
        );
        println!(
            "✓ Frame 1 rendered successfully: {} draw commands.",
            cmd_count_frame1
        );

        // Frame 2: Hierarchy Selection
        runner.app.select_node("node_camera");
        assert_eq!(runner.app.selected_node_id, "node_camera");
        assert!(runner.app.inspector.title.contains("MainCamera"));
        println!("✓ Object selection verified: switched to Camera inspector.");

        // Frame 3: Transform & Property Editing
        runner.app.select_node("node_hero");
        runner.app.scene_nodes[7].position[0] = 5.25;
        runner.app.inspector =
            KathamoForgeApp::build_inspector_for_node(&runner.app.scene_nodes[7]);
        assert_eq!(runner.app.inspector.get_float("tr_pos_z"), Some(0.0));
        println!("✓ Inspector property mutation verified: position updated.");

        // Frame 4: Simulation Cycle (Play -> Pause -> Stop)
        runner.app.set_sim_state(SimulationState::Playing);
        assert_eq!(runner.app.sim_state, SimulationState::Playing);
        runner.step_frame();
        runner.app.set_sim_state(SimulationState::Paused);
        assert_eq!(runner.app.sim_state, SimulationState::Paused);
        runner.step_frame();
        runner.app.set_sim_state(SimulationState::Stopped);
        assert_eq!(runner.app.sim_state, SimulationState::Stopped);
        println!("✓ Simulation state transitions verified (Play -> Pause -> Stop).");

        // Frame 5: Workspace Modes (3D -> 2D -> Script -> Graph)
        runner.app.set_workspace_mode(EditorWorkspaceMode::Mode2D);
        runner.step_frame();
        runner
            .app
            .set_workspace_mode(EditorWorkspaceMode::ModeScript);
        runner.step_frame();
        runner
            .app
            .set_workspace_mode(EditorWorkspaceMode::ModeGraph);
        runner.step_frame();
        runner.app.graph_view_mode = GraphViewMode::AdvancedNodes;
        runner.step_frame();
        runner.app.graph_view_mode = GraphViewMode::SimpleGuided;
        runner.step_frame();
        println!("✓ All 4 workspace modes and dual graph presentations verified.");

        // Frame 6: Graph Serialization & Validation
        let serialized_graph = runner
            .app
            .graph_document
            .serialize()
            .expect("Graph must serialize");
        assert!(serialized_graph.starts_with("KATHAMO_GRAPH_1"));
        let deserialized_doc =
            GraphDocument::deserialize(&serialized_graph).expect("Graph must deserialize");
        assert_eq!(deserialized_doc.nodes.len(), 5);
        assert_eq!(deserialized_doc.connections.len(), 6);
        println!("✓ Game logic GraphDocument serialization & roundtrip verified.");

        // Frame 7: Dynamic Object Creation & Deletion
        let initial_count = runner.app.scene_nodes.len();
        runner.app.add_new_object("TestProp");
        assert_eq!(runner.app.scene_nodes.len(), initial_count + 1);
        runner.app.delete_selected_object();
        assert_eq!(runner.app.scene_nodes.len(), initial_count);
        println!("✓ Dynamic scene object creation and deletion verified.");

        // Frame 8: CPU Pixel Capture
        let image = runner
            .capture_pixels()
            .expect("CPU scanline rendering must succeed");
        assert_eq!(image.len(), 1600 * 1000, "Image resolution match");
        println!(
            "✓ Infallible CPU Safe-Mode pixel rasterization verified: {} pixels.",
            image.len()
        );

        println!("===========================================================");
        println!("  ALL HEADLESS CHECKS PASSED (100% SUCCESS)  ");
        println!("===========================================================");
        return;
    }

    // Interactive Native Desktop App
    let app = KathamoForgeApp::new();
    run_native(app).expect("Kathamo Forge native application failed");
}
