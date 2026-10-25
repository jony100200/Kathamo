//! Standard Application Lifecycle, Context, and Runner for Kathamo SDK.

use ui_core::*;
use std::collections::HashMap;
use std::sync::Arc;
use ui_render::GpuBackend;
use ui_platform::{FocusManager, ImeEvent, Key, Modifiers, PointerButton, PointerState};
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

/// An RGBA8 frame supplied by the consuming application (4 bytes per pixel).
pub struct ImageFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
    pub version: u64,
}

/// Secondary OS-window lifecycle request, processed by NativeAppRunner.
pub enum WindowRequest {
    Open { id: String, title: String, width: u32, height: u32 },
    Close { id: String },
}

/// Unified runtime context provided to applications on each frame.
pub struct AppContext {
    pub window_id: String,
    pub window_requests: Vec<WindowRequest>,
    pub scale: f32,
    pub physical_size: (u32, u32),
    pub viewport: Rect,
    pub pointer: PointerState,
    pub focus: FocusManager,
    pub theme: ThemePalette,
    pub cmd_buf: CommandBuffer,
    pub frame_count: u64,
    pub images: HashMap<u64, ImageFrame>,
    next_image_version: u64,
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new(1440, 900, 1.0)
    }
}

impl AppContext {
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        let logical_w = width as f32 / scale;
        let logical_h = height as f32 / scale;
        Self {
            window_id: "main".into(),
            window_requests: Vec::new(),
            scale,
            physical_size: (width, height),
            viewport: Rect::from_min_size(Vec2::ZERO, Vec2::new(logical_w, logical_h)),
            pointer: PointerState::default(),
            focus: FocusManager::new(),
            theme: ThemePalette::dark(),
            cmd_buf: CommandBuffer::new(),
            frame_count: 0,
            images: HashMap::new(),
            next_image_version: 0,
        }
    }

    /// Register or update an RGBA8 image for use with DrawCommand::Image.
    /// CPU-safe-mode and GPU rendering both consume this same image.
    pub fn set_image_rgba8(&mut self, id: u64, width: u32, height: u32, rgba: impl Into<Arc<[u8]>>) -> Result<(), String> {
        let rgba = rgba.into();
        if width == 0 || height == 0 ||
            (width as usize).checked_mul(height as usize).and_then(|v| v.checked_mul(4)) != Some(rgba.len()) {
            return Err("invalid RGBA8 image dimensions or byte count".into());
        }
        self.next_image_version = self.next_image_version.wrapping_add(1);
        self.images.insert(id, ImageFrame { width, height, rgba, version: self.next_image_version });
        Ok(())
    }

    pub fn remove_image(&mut self, id: u64) {
        self.images.remove(&id);
    }

    /// Queue a secondary OS window, rendered through KathamoApp::update_window.
    pub fn open_window(&mut self, id: &str, title: &str, width: u32, height: u32) {
        if id != "main" && !id.is_empty() {
            self.window_requests.push(WindowRequest::Open {
                id: id.into(), title: title.into(), width: width.max(240), height: height.max(180),
            });
        }
    }

    pub fn close_window(&mut self, id: &str) {
        if id != "main" {
            self.window_requests.push(WindowRequest::Close { id: id.into() });
        }
    }

    /// Resize context viewport on OS window resize.
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) {
        self.scale = scale;
        self.physical_size = (width, height);
        self.viewport = Rect::from_min_size(
            Vec2::ZERO,
            Vec2::new(width as f32 / scale, height as f32 / scale),
        );
    }

    /// Logical bounding rectangle of the application window.
    pub fn bounds(&self) -> Rect {
        self.viewport
    }

    /// Active theme palette reference.
    pub fn palette(&self) -> &ThemePalette {
        &self.theme
    }
}

/// The core application trait that consuming applications implement.
pub trait KathamoApp: 'static {
    /// Window title.
    fn title(&self) -> &str {
        "Kathamo Application"
    }

    /// Initial logical window dimensions.
    fn default_size(&self) -> (u32, u32) {
        (1440, 900)
    }

    /// Optional initialization logic.
    fn init(&mut self, _ctx: &mut AppContext) {}

    /// Main frame update and UI command generation loop.
    fn update(&mut self, ctx: &mut AppContext);

    /// Optional pointer input hook.
    fn handle_mouse_down(&mut self, _pos: Vec2, _button: PointerButton, _ctx: &mut AppContext) -> bool {
        false
    }

    /// Pointer movement (including dragging) in logical coordinates.
    fn handle_mouse_move(&mut self, _pos: Vec2, _delta: Vec2, _ctx: &mut AppContext) -> bool {
        false
    }

    fn handle_mouse_up(&mut self, _pos: Vec2, _button: PointerButton, _ctx: &mut AppContext) -> bool {
        false
    }

    fn handle_scroll(&mut self, _delta: Vec2, _pos: Vec2, _ctx: &mut AppContext) -> bool {
        false
    }

    fn handle_key_down(&mut self, _key: Key, _modifiers: Modifiers, _ctx: &mut AppContext) -> bool {
        false
    }

    fn handle_text_input(&mut self, _text: &str, _ctx: &mut AppContext) -> bool {
        false
    }

    fn handle_ime(&mut self, _event: ImeEvent, _ctx: &mut AppContext) -> bool {
        false
    }

    /// Render a secondary native window. Override to display detached editor content.
    /// The default renders the application's regular interface.
    fn update_window(&mut self, _window_id: &str, ctx: &mut AppContext) {
        self.update(ctx);
    }

    /// The default routes window-specific input to the normal app hooks.
    fn handle_window_mouse_down(&mut self, _id: &str, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        self.handle_mouse_down(pos, button, ctx)
    }
    fn handle_window_mouse_move(&mut self, _id: &str, pos: Vec2, delta: Vec2, ctx: &mut AppContext) -> bool {
        self.handle_mouse_move(pos, delta, ctx)
    }
    fn handle_window_mouse_up(&mut self, _id: &str, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        self.handle_mouse_up(pos, button, ctx)
    }
    fn handle_window_scroll(&mut self, _id: &str, delta: Vec2, pos: Vec2, ctx: &mut AppContext) -> bool {
        self.handle_scroll(delta, pos, ctx)
    }
    fn handle_window_key_down(&mut self, _id: &str, key: Key, modifiers: Modifiers, ctx: &mut AppContext) -> bool {
        self.handle_key_down(key, modifiers, ctx)
    }
    fn handle_window_text_input(&mut self, _id: &str, text: &str, ctx: &mut AppContext) -> bool {
        self.handle_text_input(text, ctx)
    }
    fn handle_window_ime(&mut self, _id: &str, event: ImeEvent, ctx: &mut AppContext) -> bool {
        self.handle_ime(event, ctx)
    }
    fn window_closed(&mut self, _id: &str) {}

    /// Prepare a secondary window's own GPU device for same-device textures.
    fn prepare_window_gpu(&mut self, _id: &str, gpu: &mut GpuBackend) {
        self.prepare_gpu(gpu);
    }

    /// Optional zero-copy GPU viewport registration and rendering hook.
    /// `GpuBackend::register_external_view` accepts same-device wgpu texture views.
    fn prepare_gpu(&mut self, _gpu: &mut GpuBackend) {}

    /// Enable this for animation or other actively updating views.
    fn wants_continuous_redraw(&self) -> bool {
        false
    }
}

/// Headless test runner executing frames without requiring an OS window.
pub struct HeadlessAppRunner<A: KathamoApp> {
    pub app: A,
    pub ctx: AppContext,
}

impl<A: KathamoApp> HeadlessAppRunner<A> {
    pub fn new(mut app: A) -> Self {
        let (width, height) = app.default_size();
        let mut ctx = AppContext::new(width, height, 1.0);
        app.init(&mut ctx);
        Self { app, ctx }
    }

    /// Execute a simulated frame, returning the count of emitted draw commands.
    pub fn step_frame(&mut self) -> usize {
        self.ctx.cmd_buf.clear();
        self.ctx.frame_count += 1;
        self.app.update(&mut self.ctx);
        self.ctx.cmd_buf.commands().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestApp {
        rendered_frames: u64,
    }

    impl KathamoApp for TestApp {
        fn title(&self) -> &str {
            "Test Suite App"
        }

        fn default_size(&self) -> (u32, u32) {
            (1280, 720)
        }

        fn update(&mut self, ctx: &mut AppContext) {
            self.rendered_frames += 1;
            ctx.cmd_buf.push(ui_render::DrawCommand::Rect {
                rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 100.0)),
                fill: Color::WHITE,
                radius: CornerRadius::ZERO,
                stroke: Stroke::NONE,
            });
        }
    }

    #[test]
    fn test_headless_app_runner_executes_lifecycle() {
        let app = TestApp { rendered_frames: 0 };
        let mut runner = HeadlessAppRunner::new(app);
        assert_eq!(runner.ctx.physical_size, (1280, 720));
        assert_eq!(runner.ctx.viewport.size(), Vec2::new(1280.0, 720.0));

        let command_count = runner.step_frame();
        assert_eq!(command_count, 1);
        assert_eq!(runner.app.rendered_frames, 1);
        assert_eq!(runner.ctx.frame_count, 1);
    }
}
