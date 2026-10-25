//! Interface Demonstration 4: Video Editor
//!
//! Built exclusively using public Kathamo SDK APIs (`use kathamo::prelude::*;`).
//! Demonstrates:
//! - Video preview monitor host (16:9 monitor frame)
//! - Multi-track scrubbable timeline with keyframes for video overlays, footage, and audio tracks
//! - Clip & sequence properties inspector (Playback Speed, Audio Gain, In/Out Points, Blending)
//! - Media browser for project footage, sound effects, and music stems
//! - MenuBar, DialogHost (Export Video modal), and NotificationHost

use kathamo::prelude::*;

pub struct VideoEditorApp {
    pub workspace: WorkspaceHost,
    pub menu_bar: MenuBar,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
    pub inspector_snapshot: Option<UiInspectorSnapshot>,
}

impl VideoEditorApp {
    pub fn new() -> Self {
        let mut registry = EditorRegistry::new();

        // 1. Video Monitor Preview Canvas
        registry.register_factory("canvas", "Video Monitor Preview", "Viewports", || {
            Box::new(CanvasEditor::new_3d_mock())
        });

        // 2. Multi-track Timeline Editor
        registry.register_factory("timeline", "Sequencer / Tracks", "Animation", || {
            let timeline = Timeline {
                total_frames: 300,
                current_frame: 48,
                fps: 60,
                tracks: vec![
                    TimelineTrack {
                        name: "V2: Lower Thirds & Overlays".into(),
                        keyframes: vec![0, 30, 90, 150, 240],
                    },
                    TimelineTrack {
                        name: "V1: Master Camera A (4K Prores)".into(),
                        keyframes: vec![0, 60, 120, 180, 240, 300],
                    },
                    TimelineTrack {
                        name: "A1: Dialogue & Narration (24-bit)".into(),
                        keyframes: vec![0, 15, 45, 90, 180],
                    },
                    TimelineTrack {
                        name: "A2: Ambient Foley & Music Stem".into(),
                        keyframes: vec![0, 60, 120, 240],
                    },
                ],
                ..Default::default()
            };
            Box::new(TimelineEditor { timeline })
        });

        // 3. Clip & Track Property Inspector
        registry.register_factory("property_inspector", "Clip Properties", "Inspectors", || {
            let inspector = MetadataPropertyInspector::new(
                "Selected Clip: Camera_A_Take03.mov",
                vec![
                    PropertyItem::integer("in_frame", "In Point (Frames)", 0, 0, 10000),
                    PropertyItem::integer("out_frame", "Out Point (Frames)", 240, 0, 10000),
                    PropertyItem::float("playback_speed", "Speed Multiplier", 1.0, 0.1, 10.0, 0.1),
                    PropertyItem::float("audio_gain", "Audio Volume (dB)", 0.0, -60.0, 12.0, 0.5),
                    PropertyItem::float("video_opacity", "Opacity", 1.0, 0.0, 1.0, 0.05),
                    PropertyItem::boolean("color_lut_enabled", "Apply Rec.709 3D LUT", true),
                    PropertyItem::enum_choice(
                        "composite_mode",
                        "Composite Mode",
                        0,
                        vec!["Normal".into(), "Screen".into(), "Multiply".into(), "Overlay".into()],
                    ),
                ],
            );
            Box::new(InspectorEditor { inspector })
        });

        // 4. Media Bin Asset Browser
        registry.register_factory("asset_browser", "Project Media Bin", "Data", || {
            let browser = AssetBrowser {
                items: vec![
                    AssetItem::new("clip_a", "Camera_A_Take01.mov", "Video"),
                    AssetItem::new("clip_b", "Camera_A_Take02.mov", "Video"),
                    AssetItem::new("clip_c", "Camera_A_Take03.mov", "Video"),
                    AssetItem::new("audio_vo", "Dialogue_Narration.wav", "Audio"),
                    AssetItem::new("audio_bgm", "Cinematic_Theme_Master.wav", "Audio"),
                    AssetItem::new("overlay_logo", "Studio_Watermark.png", "Graphics"),
                ],
                ..Default::default()
            };
            Box::new(AssetBrowserEditor { browser })
        });

        let mut workspace = WorkspaceHost::with_registry(registry);
        workspace.switch_workspace(3); // VideoEditing preset

        // Application Menu Bar
        let menu_bar = MenuBar::new(vec![
            Menu::new(
                "File",
                vec![
                    MenuItem::action("file_new_proj", "New Sequence", Some("Ctrl+N")),
                    MenuItem::action("file_import", "Import Footage...", Some("Ctrl+I")),
                    MenuItem::separator(),
                    MenuItem::action("file_save_proj", "Save Sequence", Some("Ctrl+S")),
                    MenuItem::action("file_export_video", "Render & Export Master...", Some("Ctrl+M")),
                ],
            ),
            Menu::new(
                "Edit",
                vec![
                    MenuItem::action("edit_undo", "Undo Split", Some("Ctrl+Z")),
                    MenuItem::action("edit_redo", "Redo", Some("Ctrl+Y")),
                    MenuItem::separator(),
                    MenuItem::action("edit_split", "Split Clip at Playhead", Some("Ctrl+K")),
                    MenuItem::action("edit_ripple_del", "Ripple Delete", Some("Shift+Del")),
                ],
            ),
            Menu::new(
                "Playback",
                vec![
                    MenuItem::action("play_toggle", "Play / Pause", Some("Space")),
                    MenuItem::action("play_next_frame", "Step Forward 1 Frame", Some("Right")),
                    MenuItem::action("play_prev_frame", "Step Backward 1 Frame", Some("Left")),
                    MenuItem::action("play_jump_start", "Jump to Sequence Start", Some("Home")),
                ],
            ),
        ]);

        let mut notifications = NotificationHost::new();
        notifications.info("Timeline Active", "Sequence: 3840x2160 @ 60.00 FPS. Audio: 48kHz Stereo.");

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
            "render_export_modal",
            "Render Master Video",
            "Output Format: ProRes 422 HQ / H.265. Start asynchronous background render job?",
        );
    }
}

impl Default for VideoEditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl KathamoApp for VideoEditorApp {
    fn title(&self) -> &str {
        "Kathamo NLE Video Workstation"
    }

    fn default_size(&self) -> (u32, u32) {
        (1600, 950)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let palette = ctx.theme;
        let bounds = ctx.bounds();

        // 1. Menu Bar
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar.render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // 2. Docked Video Workspace
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
            let mut snap = InterfaceInspector::new_snapshot("video_editor_root", bounds);
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
                if action_id == "file_export_video" {
                    self.trigger_export_dialog();
                } else if action_id == "file_save_proj" {
                    self.notifications.success("Sequence Saved", "Timeline markers and EDL synced.");
                }
                return true;
            }
        }

        if self.dialogs.is_open {
            if let Some((_id, action)) = self.dialogs.handle_mouse_down(pos, button, bounds) {
                if action == DialogAction::Confirm {
                    self.notifications.success("Render Queued", "Exporting video sequence to disk...");
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
    println!("=== Kathamo Demonstration 4: Video Editor ===");
    let app = VideoEditorApp::new();
    let mut runner = HeadlessAppRunner::new(app);

    let mut last_cmd_count = 0;
    for frame_idx in 0..10 {
        last_cmd_count = runner.step_frame();
        assert!(last_cmd_count > 0, "Video editor must render commands");
        if frame_idx == 4 {
            runner.app.trigger_export_dialog();
        }
    }

    assert!(runner.app.dialogs.is_open, "Export video dialog should be open");
    assert!(runner.app.inspector_snapshot.is_some(), "Inspector snapshot captured");

    println!("✓ Video Editor simulated successfully.");
    println!("✓ Total commands rendered: {}", last_cmd_count);
    println!("✓ Workspaces active: {}", runner.app.workspace.workspaces.len());
}
