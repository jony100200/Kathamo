//! Interface Demonstration 5: General Desktop Application
//!
//! Built exclusively using public Kathamo SDK APIs (`use kathamo::prelude::*;`).
//! Demonstrates:
//! - Desktop dashboard with declarative forms, metric cards, and controls via `LayoutBuilder`
//! - Interactive toggles, numeric scrubbers, push buttons, and horizontal card rows
//! - Diagnostics log console with real-time logging entries
//! - Modal configuration dialog (`DialogHost`) and system toast notifications (`NotificationHost`)
//! - Multi-workspace switching between Dashboard, System Settings, and Diagnostics

use kathamo::prelude::*;

pub struct DesktopApplication {
    pub workspace: WorkspaceHost,
    pub menu_bar: MenuBar,
    pub notifications: NotificationHost,
    pub dialogs: DialogHost,
    pub telemetry_enabled: bool,
    pub hardware_accel: bool,
    pub thread_count: f32,
    pub cache_limit_mb: f32,
    pub inspector_snapshot: Option<UiInspectorSnapshot>,
}

impl DesktopApplication {
    pub fn new() -> Self {
        let mut registry = EditorRegistry::new();

        // 1. Diagnostics Log Console Editor
        registry.register_factory("console_log", "System Console Logs", "Diagnostics", || {
            let mut console = ConsoleView::default();
            console.push("System kernel initialized. IPC socket open.", ConsoleLogLevel::Info);
            console.push("Connecting to local worker daemon on 127.0.0.1:4012...", ConsoleLogLevel::Info);
            console.push("Network sync established in 12ms.", ConsoleLogLevel::Info);
            console.push("Low memory warning: Cache directory exceeds 1024 MB threshold.", ConsoleLogLevel::Warning);
            Box::new(ConsoleEditor { console })
        });

        // 2. Project Asset / Database Browser
        registry.register_factory("asset_browser", "Connected Data Services", "Data", || {
            let browser = AssetBrowser {
                items: vec![
                    AssetItem::new("svc_db", "Primary SQLite Database", "Storage"),
                    AssetItem::new("svc_cache", "Memory Cache Ring Buffer", "Storage"),
                    AssetItem::new("svc_worker", "Background Process Worker Fleet", "Compute"),
                    AssetItem::new("svc_gpu", "Vulkan Compute Dispatcher", "Compute"),
                    AssetItem::new("svc_auth", "Local Token Authority", "Security"),
                ],
                ..Default::default()
            };
            Box::new(AssetBrowserEditor { browser })
        });

        // 3. Settings Property Inspector
        registry.register_factory("property_inspector", "Application Settings", "Inspectors", || {
            let inspector = MetadataPropertyInspector::new(
                "Global Configuration",
                vec![
                    PropertyItem::boolean("auto_update", "Check for Updates Automatically", true),
                    PropertyItem::boolean("offline_mode", "Enforce Strict Offline Execution", true),
                    PropertyItem::integer("max_history", "Undo History Stack Limit", 100, 10, 500),
                    PropertyItem::enum_choice(
                        "ui_theme",
                        "UI Color Theme",
                        0,
                        vec!["Kathamo Dark".into(), "Kathamo Charcoal".into(), "Kathamo Light".into()],
                    ),
                    PropertyItem::float("ui_scale", "Interface Scaling Factor", 1.0, 0.75, 2.5, 0.25),
                ],
            );
            Box::new(InspectorEditor { inspector })
        });

        let mut workspace = WorkspaceHost::with_registry(registry);
        workspace.switch_workspace(4); // GeneralDesktop preset

        // Application Menu Bar
        let menu_bar = MenuBar::new(vec![
            Menu::new(
                "Application",
                vec![
                    MenuItem::action("app_preferences", "Preferences...", Some("Ctrl+,")),
                    MenuItem::separator(),
                    MenuItem::action("app_restart", "Restart Service Fleet", Some("Ctrl+Shift+R")),
                    MenuItem::action("app_exit", "Quit Application", Some("Ctrl+Q")),
                ],
            ),
            Menu::new(
                "Tools",
                vec![
                    MenuItem::action("tools_diag", "Run Health Self-Test", Some("F5")),
                    MenuItem::action("tools_purge_cache", "Purge Memory Cache", None),
                    MenuItem::action("tools_migrate", "Database Migration Wizard...", None),
                ],
            ),
            Menu::new(
                "Help",
                vec![
                    MenuItem::action("help_docs", "Kathamo Architecture Guide", Some("F1")),
                    MenuItem::action("help_about", "About Kathamo Universal SDK", None),
                ],
            ),
        ]);

        let mut notifications = NotificationHost::new();
        notifications.info("Daemon Connected", "Connected to local hardware services on 127.0.0.1 (0.8ms latency).");

        Self {
            workspace,
            menu_bar,
            notifications,
            dialogs: DialogHost::new(),
            telemetry_enabled: false,
            hardware_accel: true,
            thread_count: 8.0,
            cache_limit_mb: 1024.0,
            inspector_snapshot: None,
        }
    }

    pub fn trigger_migration_dialog(&mut self) {
        self.dialogs.open(
            "migration_modal",
            "Database Schema Migration",
            "Target Schema: v2.4. Apply WAL migration to persistent SQLite data store?",
        );
    }
}

impl Default for DesktopApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl KathamoApp for DesktopApplication {
    fn title(&self) -> &str {
        "Kathamo Desktop Workstation & Service Dashboard"
    }

    fn default_size(&self) -> (u32, u32) {
        (1500, 950)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let palette = ctx.theme;
        let bounds = ctx.bounds();

        // 1. Menu Bar
        let menu_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), 28.0));
        self.menu_bar.render(&mut ctx.cmd_buf, menu_rect, &ctx.pointer, &palette);

        // 2. Docked Workspace Area
        let workspace_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.min.y + 28.0),
            Vec2::new(bounds.width(), bounds.height() - 28.0),
        );
        self.workspace.render(&mut ctx.cmd_buf, workspace_rect, &ctx.pointer, &palette);

        // 3. Declarative Forms & HUD in Leaf Area (via LayoutBuilder)
        let hud_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + 16.0, bounds.min.y + 40.0),
            Vec2::new(320.0, 360.0),
        );
        let mut layout = LayoutBuilder::new(hud_rect, &mut ctx.cmd_buf, &ctx.pointer, &palette);
        layout.card(|ui| {
            ui.heading("Hardware Fleet Telemetry");
            ui.separator();
            ui.label("Active Workers: 8 / 8 Online");
            ui.label("Memory Footprint: 214 MB (Clean)");
            ui.space(4.0);
            ui.toggle("Vulkan HW Acceleration", &mut self.hardware_accel);
            ui.toggle("Anonymous Telemetry", &mut self.telemetry_enabled);
            ui.space(4.0);
            ui.scrubber("Thread Pool", &mut self.thread_count, 1.0, 32.0, 1.0);
            ui.scrubber("Cache Max (MB)", &mut self.cache_limit_mb, 128.0, 4096.0, 64.0);
            ui.space(6.0);
            ui.horizontal(|row| {
                if row.button("Run Diagnostics") {
                    // Triggered
                }
                if row.button("Purge Cache") {
                    // Triggered
                }
            });
        });

        // 4. Modal Dialogs & Notifications
        self.dialogs.render(&mut ctx.cmd_buf, bounds, &ctx.pointer, &palette);
        self.notifications.render(&mut ctx.cmd_buf, bounds, &palette);

        // 5. Inspector Snapshot
        if self.inspector_snapshot.is_none() {
            let mut snap = InterfaceInspector::new_snapshot("desktop_app_root", bounds);
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
                if action_id == "tools_migrate" {
                    self.trigger_migration_dialog();
                } else if action_id == "tools_diag" {
                    self.notifications.success("Self-Test Passed", "All IPC endpoints and threads healthy.");
                } else if action_id == "tools_purge_cache" {
                    self.notifications.info("Cache Purged", "Freed 412 MB from disk cache.");
                }
                return true;
            }
        }

        if self.dialogs.is_open {
            if let Some((_id, action)) = self.dialogs.handle_mouse_down(pos, button, bounds) {
                if action == DialogAction::Confirm {
                    self.notifications.success("Migration Complete", "Database updated to schema v2.4.");
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
    println!("=== Kathamo Demonstration 5: General Desktop Application ===");
    let app = DesktopApplication::new();
    let mut runner = HeadlessAppRunner::new(app);

    let mut last_cmd_count = 0;
    for frame_idx in 0..10 {
        last_cmd_count = runner.step_frame();
        assert!(last_cmd_count > 0, "Desktop application must render commands");
        if frame_idx == 4 {
            runner.app.trigger_migration_dialog();
        }
    }

    assert!(runner.app.dialogs.is_open, "Migration dialog should be open");
    assert!(runner.app.inspector_snapshot.is_some(), "Inspector snapshot captured");

    println!("✓ General Desktop Application simulated successfully.");
    println!("✓ Total commands rendered: {}", last_cmd_count);
    println!("✓ Workspaces active: {}", runner.app.workspace.workspaces.len());
}
