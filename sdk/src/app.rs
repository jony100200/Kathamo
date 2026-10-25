//! Standard Application Lifecycle, Context, and Runner for Kathamo SDK.

use ui_core::*;
use ui_platform::{FocusManager, ImeEvent, Key, Modifiers, PointerButton, PointerState};
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

/// Unified runtime context provided to applications on each frame.
pub struct AppContext {
    pub scale: f32,
    pub physical_size: (u32, u32),
    pub viewport: Rect,
    pub pointer: PointerState,
    pub focus: FocusManager,
    pub theme: ThemePalette,
    pub cmd_buf: CommandBuffer,
    pub frame_count: u64,
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
            scale,
            physical_size: (width, height),
            viewport: Rect::from_min_size(Vec2::ZERO, Vec2::new(logical_w, logical_h)),
            pointer: PointerState::default(),
            focus: FocusManager::new(),
            theme: ThemePalette::dark(),
            cmd_buf: CommandBuffer::new(),
            frame_count: 0,
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
