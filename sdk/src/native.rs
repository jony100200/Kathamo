//! Native window runner for applications using the public Kathamo SDK.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use ui_core::{Rect, Vec2};
use ui_platform::{
    winit::{
        application::ApplicationHandler,
        dpi::LogicalSize,
        event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
        keyboard::{Key as WinitKey, NamedKey},
        window::{Window, WindowAttributes, WindowId},
    },
    Key, Modifiers, PointerButton,
};
use ui_render::{CpuBackend, GpuBackend, RenderBackend, SoftwarePresenter};

use crate::app::{AppContext, FileDropEvent, FileDropQueue, KathamoApp, UiWaker, WindowRequest};

/// Runs a `KathamoApp` in a real OS window, with GPU rendering and CPU fallback.
pub fn run_native<A: KathamoApp>(app: A) -> Result<(), String> {
    if let Some(path) = std::env::var_os("KATHAMO_SNAPSHOT") {
        return crate::snapshot::snapshot_png(app, std::path::Path::new(&path));
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    run_on_event_loop(app, event_loop)
}

fn run_on_event_loop<A: KathamoApp>(app: A, event_loop: EventLoop<()>) -> Result<(), String> {
    event_loop.set_control_flow(ControlFlow::Wait);
    let data_dir = desktop_data_dir(app.title());
    let waker = waker_for(event_loop.create_proxy());
    let mut runner = NativeAppRunner::with_platform(app, Some(waker), data_dir);
    event_loop.run_app(&mut runner).map_err(|e| e.to_string())
}

/// A `UiWaker` that posts an empty user event; `user_event` then redraws.
fn waker_for(proxy: EventLoopProxy<()>) -> UiWaker {
    let proxy = std::sync::Mutex::new(proxy);
    UiWaker::new(move || {
        if let Ok(proxy) = proxy.lock() {
            let _ = proxy.send_event(());
        }
    })
}

/// Per-app data folder on desktop: `<user data dir>/Kathamo/<app title>`. The
/// folder is not created here; apps call `create_dir_all` before writing.
fn desktop_data_dir(title: &str) -> Option<std::path::PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    let base = if cfg!(windows) {
        env("APPDATA")?
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library").join("Application Support")
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local").join("share")))?
    };
    Some(base.join("Kathamo").join(app_folder_name(title)))
}

/// File-system-safe folder name from an app title.
fn app_folder_name(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let name = name.trim_matches('_').to_string();
    if name.is_empty() { "app".into() } else { name }
}

/// The Android activity handle passed to `android_main`.
#[cfg(target_os = "android")]
pub use ui_platform::winit::platform::android::activity::AndroidApp;

/// Runs a `KathamoApp` inside an Android NativeActivity. Call it from the
/// `#[no_mangle] fn android_main(app: AndroidApp)` of a `cdylib` crate.
/// stderr (and panics) appear in `adb logcat -s RustStdoutStderr`.
///
/// When the activity finishes (Back, `finish()`), the process exits: winit allows
/// one event loop per process, so a relaunch needs a fresh process anyway, and
/// Android blocks `onDestroy` until this thread is gone (see `android.rs`).
#[cfg(target_os = "android")]
pub fn run_android<A: KathamoApp>(app: A, android_app: AndroidApp) -> Result<(), String> {
    use ui_platform::winit::platform::android::EventLoopBuilderExtAndroid;
    let event_loop = EventLoop::builder()
        .with_android_app(android_app.clone())
        .build()
        .map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let data_dir = android_app.internal_data_path();
    let waker = waker_for(event_loop.create_proxy());
    let mut runner = NativeAppRunner::with_platform(app, Some(waker), data_dir);
    runner.android_app = Some(android_app);
    let result = event_loop.run_app(&mut runner).map_err(|e| e.to_string());
    if runner.finishing {
        crate::android::exit_process_now(0);
    }
    result
}

/// What one touch event means for the single-pointer UI model: the first finger
/// down acts as the primary mouse button; other fingers are ignored.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TouchAction {
    Press,
    Move,
    Release,
    Ignore,
}

fn touch_action(active: &mut Option<u64>, id: u64, phase: TouchPhase) -> TouchAction {
    match phase {
        TouchPhase::Started if active.is_none() => {
            *active = Some(id);
            TouchAction::Press
        }
        TouchPhase::Moved if *active == Some(id) => TouchAction::Move,
        TouchPhase::Ended | TouchPhase::Cancelled if *active == Some(id) => {
            *active = None;
            TouchAction::Release
        }
        _ => TouchAction::Ignore,
    }
}

/// A finger drag that no widget claimed scrolls instead, in the same units as a
/// precise trackpad (pixels / 20). Content follows the finger: dragging down
/// gives a positive delta, like turning the wheel up.
/// Space a shown on-screen keyboard takes from the bottom, at least (logical px).
/// Larger than any gesture bar, smaller than any keyboard.
const KEYBOARD_MIN_INSET: f32 = 120.0;

/// Tracks the on-screen keyboard through the safe area: once a requested
/// keyboard has taken space (`seen`), getting that space back means the user
/// closed it outside the app. Returns true at that moment.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn keyboard_was_dismissed(shown: bool, seen: &mut bool, safe_bottom: f32, full_bottom: f32) -> bool {
    if !shown {
        *seen = false;
        return false;
    }
    let covered = full_bottom - safe_bottom >= KEYBOARD_MIN_INSET;
    if covered {
        *seen = true;
        return false;
    }
    std::mem::take(seen)
}

fn touch_scroll_delta(finger_delta: Vec2) -> Vec2 {
    Vec2::new(finger_delta.x / 20.0, finger_delta.y / 20.0)
}

/// Logical safe area from Android's content rect (physical px, `left, top,
/// right, bottom`), which excludes the status bar, gesture bar and cutout. An
/// empty rect (not reported yet) or one outside the window means "whole window".
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn safe_area_from_content_rect(content: (i32, i32, i32, i32), physical: (u32, u32), scale: f32) -> Rect {
    let (w, h) = (physical.0 as f32, physical.1 as f32);
    let full = Rect::from_min_size(Vec2::ZERO, Vec2::new(w / scale, h / scale));
    let (l, t, r, b) = (content.0 as f32, content.1 as f32, content.2 as f32, content.3 as f32);
    if r <= l || b <= t {
        return full;
    }
    let (l, t, r, b) = (l.clamp(0.0, w), t.clamp(0.0, h), r.clamp(0.0, w), b.clamp(0.0, h));
    if r <= l || b <= t {
        return full;
    }
    Rect::from_min_max(Vec2::new(l / scale, t / scale), Vec2::new(r / scale, b / scale))
}

/// Owns the native presentation and input lifecycle. No UI Lab dependency.
pub struct NativeAppRunner<A: KathamoApp> {
    pub app: A,
    pub ctx: AppContext,
    window: Option<Arc<Window>>,
    gpu: Option<GpuBackend>,
    cpu: CpuBackend,
    presenter: Option<SoftwarePresenter>,
    modifiers: Modifiers,
    revealed: bool,
    children: HashMap<WindowId, SecondaryWindow>,
    gpu_images_uploaded: HashMap<u64, u64>,
    cpu_images_uploaded: HashMap<u64, u64>,
    last_frame: Option<Instant>,
    wake_at: Option<Instant>,
    occluded: bool,
    touch_id: Option<u64>,
    /// File-drag events from the OS, delivered to the app once per event batch.
    drops: FileDropQueue,
    #[cfg(target_os = "android")]
    android_app: Option<AndroidApp>,
    /// Set when the platform is closing the app for good; the loop is exiting.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    finishing: bool,
    /// Whether the phone's on-screen keyboard is currently requested.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    keyboard_shown: bool,
    /// The requested keyboard has been seen taking screen space.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    keyboard_inset_seen: bool,
}

impl<A: KathamoApp> NativeAppRunner<A> {
    pub fn new(app: A) -> Self {
        Self::with_platform(app, None, None)
    }

    /// Like `new`, with the platform services the app sees from `init` onward.
    pub fn with_platform(
        mut app: A,
        waker: Option<UiWaker>,
        data_dir: Option<std::path::PathBuf>,
    ) -> Self {
        let (width, height) = app.default_size();
        let mut ctx = AppContext::new(width, height, 1.0);
        ctx.waker = waker;
        ctx.data_dir = data_dir;
        app.init(&mut ctx);
        ui_core::motion::set_instant(false);
        Self {
            app,
            ctx,
            window: None,
            gpu: None,
            cpu: CpuBackend::new(),
            presenter: None,
            modifiers: Modifiers::NONE,
            revealed: false,
            children: HashMap::new(),
            gpu_images_uploaded: HashMap::new(),
            cpu_images_uploaded: HashMap::new(),
            last_frame: None,
            wake_at: None,
            occluded: false,
            touch_id: None,
            drops: FileDropQueue::default(),
            #[cfg(target_os = "android")]
            android_app: None,
            finishing: false,
            keyboard_shown: false,
            keyboard_inset_seen: false,
        }
    }

    /// Takes a file-drag event from the OS (the window event handler calls this for winit's
    /// `HoveredFile`, `HoveredFileCancelled` and `DroppedFile`). Delivery happens in
    /// [`NativeAppRunner::deliver_file_drops`], once the event batch is read.
    pub fn file_drop_event(&mut self, event: FileDropEvent) {
        self.drops.push(event);
    }

    /// Hands queued file-drag events to the app. Returns whether the app took dropped files.
    pub fn deliver_file_drops(&mut self) -> bool {
        if !self.drops.is_pending() {
            return false;
        }
        let taken = self.drops.deliver(&mut self.app, &mut self.ctx);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        taken
    }

    /// True while the app or any widget transition needs another frame.
    /// A minimised (zero-size) or occluded window never keeps the loop spinning.
    fn needs_next_frame(&self) -> bool {
        self.visible() && (self.app.wants_continuous_redraw() || ui_core::motion::is_animating())
    }

    fn visible(&self) -> bool {
        let (width, height) = self.ctx.physical_size;
        !self.occluded && width > 0 && height > 0
    }

    fn update_size(&mut self, window: &Window) {
        let size = window.inner_size();
        let scale = window.scale_factor() as f32;
        self.ctx.resize(size.width, size.height, scale);
        self.sync_safe_area();
        if let Some(gpu) = &mut self.gpu {
            gpu.set_scale_factor(scale);
        }
        self.cpu.set_scale_factor(scale);
    }

    /// Android: winit drops `ContentRectChanged`, so the content rect is read
    /// again before every frame. Returns true when the safe area changed.
    #[cfg(target_os = "android")]
    fn sync_safe_area(&mut self) -> bool {
        let Some(app) = &self.android_app else {
            return false;
        };
        let r = app.content_rect();
        let safe = safe_area_from_content_rect(
            (r.left, r.top, r.right, r.bottom),
            self.ctx.physical_size,
            self.ctx.scale,
        );
        if safe == self.ctx.safe_area {
            return false;
        }
        self.ctx.safe_area = safe;
        let full_bottom = self.ctx.viewport.max.y;
        if keyboard_was_dismissed(self.keyboard_shown, &mut self.keyboard_inset_seen, safe.max.y, full_bottom) {
            // Android's own Back (or the keyboard's hide key) closed it: drop the
            // request so the app sees `text_input_wanted() == false`.
            self.keyboard_shown = false;
            self.ctx.text_input_wanted = false;
        }
        true
    }

    #[cfg(not(target_os = "android"))]
    fn sync_safe_area(&mut self) -> bool {
        false
    }

    /// Android: show or hide the on-screen keyboard when the app's request
    /// (`AppContext::set_text_input`) changes. Desktop keyboards need nothing.
    fn sync_keyboard(&mut self) {
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            let wanted = self.ctx.text_input_wanted;
            if wanted != self.keyboard_shown {
                if wanted {
                    app.show_soft_input(false);
                } else {
                    app.hide_soft_input(false);
                }
                self.keyboard_shown = wanted;
                self.keyboard_inset_seen = false;
            }
        }
    }

    fn upload_frames_gpu(&mut self) -> Result<(), String> {
        let Some(gpu) = self.gpu.as_mut() else {
            return Ok(());
        };
        self.gpu_images_uploaded.retain(|id, _| {
            if self.ctx.images.contains_key(id) {
                true
            } else {
                gpu.remove_image(*id);
                false
            }
        });
        for (id, image) in &self.ctx.images {
            if self.gpu_images_uploaded.get(id) != Some(&image.version) {
                gpu.upload_rgba8_image(*id, image.width, image.height, &image.rgba)?;
                self.gpu_images_uploaded.insert(*id, image.version);
            }
        }
        Ok(())
    }

    fn upload_frames_cpu(&mut self) -> Result<(), String> {
        self.cpu_images_uploaded.retain(|id, _| {
            if self.ctx.images.contains_key(id) {
                true
            } else {
                self.cpu.remove_image(*id);
                false
            }
        });
        for (id, image) in &self.ctx.images {
            if self.cpu_images_uploaded.get(id) != Some(&image.version) {
                self.cpu
                    .upload_rgba8_image(*id, image.width, image.height, &image.rgba)?;
                self.cpu_images_uploaded.insert(*id, image.version);
            }
        }
        Ok(())
    }

    fn process_window_requests(
        &mut self,
        event_loop: &ActiveEventLoop,
        requests: Vec<WindowRequest>,
    ) {
        for request in requests {
            match request {
                WindowRequest::Open {
                    id,
                    title,
                    width,
                    height,
                } => {
                    if self.children.values().any(|w| w.id == id) {
                        continue;
                    }
                    match SecondaryWindow::new(event_loop, id, title, width, height) {
                        Ok(child) => {
                            child.window.request_redraw();
                            self.children.insert(child.window.id(), child);
                        }
                        Err(error) => eprintln!("Kathamo secondary window: {error}"),
                    }
                }
                WindowRequest::Close { id } => {
                    let key = self
                        .children
                        .iter()
                        .find(|(_, child)| child.id == id)
                        .map(|(key, _)| *key);
                    if let Some(key) = key {
                        self.children.remove(&key);
                        self.app.window_closed(&id);
                        if let Some(main) = &self.window {
                            main.request_redraw();
                        }
                    }
                }
            }
        }
    }

    fn secondary_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(&event, WindowEvent::CloseRequested) {
            if let Some(child) = self.children.remove(&id) {
                self.app.window_closed(&child.id);
                if let Some(main) = &self.window {
                    main.request_redraw();
                }
            }
            return;
        }
        let Some(child) = self.children.get_mut(&id) else {
            return;
        };
        let window = child.window.clone();
        match event {
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                child.resize();
                window.request_redraw();
            }
            WindowEvent::Focused(false) => {
                for button in [
                    PointerButton::Primary,
                    PointerButton::Secondary,
                    PointerButton::Middle,
                ] {
                    child.ctx.pointer.on_button_up(button);
                }
                child.ctx.focus.clear();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Vec2::new(
                    position.x as f32 / child.ctx.scale,
                    position.y as f32 / child.ctx.scale,
                );
                let delta = pos - child.ctx.pointer.position;
                child.ctx.pointer.on_move(pos);
                self.app
                    .handle_window_mouse_move(&child.id, pos, delta, &mut child.ctx);
                window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = mouse_button(button) {
                    let pos = child.ctx.pointer.position;
                    match state {
                        ElementState::Pressed => {
                            child.ctx.pointer.on_button_down(button);
                            self.app.handle_window_mouse_down(
                                &child.id,
                                pos,
                                button,
                                &mut child.ctx,
                            );
                        }
                        ElementState::Released => {
                            self.app
                                .handle_window_mouse_up(&child.id, pos, button, &mut child.ctx);
                            child.ctx.pointer.on_button_up(button);
                        }
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        Vec2::new(p.x as f32 / 20.0, p.y as f32 / 20.0)
                    }
                };
                child.ctx.pointer.scroll_delta = delta;
                let pos = child.ctx.pointer.position;
                self.app
                    .handle_window_scroll(&child.id, delta, pos, &mut child.ctx);
                window.request_redraw();
            }
            WindowEvent::ModifiersChanged(mods) => {
                let mods = mods.state();
                child.modifiers = Modifiers {
                    ctrl: mods.control_key(),
                    shift: mods.shift_key(),
                    alt: mods.alt_key(),
                    command: mods.super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Released => {
                let key = key_from_winit(&event.logical_key);
                self.app
                    .handle_window_key_up(&child.id, key, child.modifiers, &mut child.ctx);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let key = key_from_winit(&event.logical_key);
                let handled = self.app.handle_window_key_down(
                    &child.id,
                    key.clone(),
                    child.modifiers,
                    &mut child.ctx,
                );
                if !handled && key == Key::Tab {
                    child.ctx.focus.cycle_focus(child.modifiers.shift);
                }
                if let Some(text) = event.text {
                    if accepts_key_text(text.as_str(), child.modifiers) {
                        self.app
                            .handle_window_text_input(&child.id, text.as_str(), &mut child.ctx);
                    }
                }
                window.request_redraw();
            }
            WindowEvent::Ime(ime) => {
                use ui_platform::ImeEvent;
                let event = match ime {
                    Ime::Preedit(text, cursor) => Some(ImeEvent::Preedit(text, cursor)),
                    Ime::Commit(text) => Some(ImeEvent::Commit(text)),
                    Ime::Disabled => Some(ImeEvent::Disabled),
                    Ime::Enabled => None,
                };
                if let Some(event) = event {
                    self.app.handle_window_ime(&child.id, event, &mut child.ctx);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                match child.present(&mut self.app) {
                    Ok(()) => {
                        if !child.revealed
                            && child.ctx.physical_size.0 > 0
                            && child.ctx.physical_size.1 > 0
                        {
                            window.set_visible(true);
                            child.revealed = true;
                        }
                    }
                    Err(error) => eprintln!("Kathamo secondary window frame: {error}"),
                }
                if self.app.wants_continuous_redraw() || ui_core::motion::is_animating() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
        let requests = std::mem::take(&mut child.ctx.window_requests);
        self.process_window_requests(event_loop, requests);
    }

    fn present(&mut self) -> Result<(), String> {
        let (width, height) = self.ctx.physical_size;
        if width == 0 || height == 0 {
            return Ok(());
        }

        // After an idle gap (no redraws), step one nominal frame instead of jumping.
        let now = Instant::now();
        let gap = self.last_frame.map_or(0.0, |t| (now - t).as_secs_f32());
        let dt = if gap > 0.1 { 1.0 / 60.0 } else { gap };
        self.last_frame = Some(now);
        ui_core::motion::begin_frame(dt);

        self.ctx.cmd_buf.clear();
        self.ctx.frame_count = self.ctx.frame_count.wrapping_add(1);
        self.app.update(&mut self.ctx);
        self.sync_keyboard();
        if let Some(t) = ui_core::motion::take_wake_deadline() {
            self.wake_at = Some(self.wake_at.map_or(t, |w| w.min(t)));
        }

        if let Some(gpu) = &mut self.gpu {
            self.app.prepare_gpu(gpu);
        }
        self.upload_frames_gpu()?;
        if let Some(gpu) = &mut self.gpu {
            gpu.begin_frame(width, height, self.ctx.theme.canvas);
            gpu.submit(&self.ctx.cmd_buf);
            if gpu.end_frame().is_ok() {
                self.ctx.pointer.step_frame();
                return Ok(());
            }
            // A lost GPU device gets one recovery attempt before software fallback.
            if gpu.recover().is_ok() {
                self.gpu_images_uploaded.clear();
                self.app.prepare_gpu(gpu);
                gpu.set_scale_factor(self.ctx.scale);
                // Application must re-register any external views after recovery.
                gpu.begin_frame(width, height, self.ctx.theme.canvas);
                gpu.submit(&self.ctx.cmd_buf);
                if gpu.end_frame().is_ok() {
                    self.ctx.pointer.step_frame();
                    return Ok(());
                }
            }
            self.gpu = None;
        }

        self.upload_frames_cpu()?;
        self.cpu.begin_frame(width, height, self.ctx.theme.canvas);
        self.cpu.submit(&self.ctx.cmd_buf);
        self.cpu.end_frame()?;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or("CPU window presenter unavailable")?;
        presenter.present(self.cpu.pixels(), width, height)?;
        self.ctx.pointer.step_frame();
        Ok(())
    }
}

fn mouse_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

fn key_from_winit(key: &WinitKey) -> Key {
    match key {
        WinitKey::Character(chars) if chars.chars().count() == 1 => {
            Key::Character(chars.chars().next().unwrap_or(' '))
        }
        WinitKey::Named(NamedKey::Escape) => Key::Escape,
        WinitKey::Named(NamedKey::Enter) => Key::Enter,
        WinitKey::Named(NamedKey::Tab) => Key::Tab,
        WinitKey::Named(NamedKey::Backspace) => Key::Backspace,
        WinitKey::Named(NamedKey::Delete) => Key::Delete,
        WinitKey::Named(NamedKey::Space) => Key::Space,
        WinitKey::Named(NamedKey::ArrowUp) => Key::ArrowUp,
        WinitKey::Named(NamedKey::ArrowDown) => Key::ArrowDown,
        WinitKey::Named(NamedKey::ArrowLeft) => Key::ArrowLeft,
        WinitKey::Named(NamedKey::ArrowRight) => Key::ArrowRight,
        WinitKey::Named(NamedKey::Home) => Key::Custom("Home".into()),
        WinitKey::Named(NamedKey::End) => Key::Custom("End".into()),
        WinitKey::Named(NamedKey::PageUp) => Key::Custom("PageUp".into()),
        WinitKey::Named(NamedKey::PageDown) => Key::Custom("PageDown".into()),
        // Android's Back button (winit reports it as the browser key).
        WinitKey::Named(NamedKey::BrowserBack) => Key::Custom("Back".into()),
        WinitKey::Named(named) => {
            function_key(*named).map_or_else(|| Key::Custom(format!("{key:?}")), Key::F)
        }
        _ => Key::Custom(format!("{key:?}")),
    }
}

/// F1..F12 as their number.
fn function_key(key: NamedKey) -> Option<u8> {
    const KEYS: [NamedKey; 12] = [
        NamedKey::F1,
        NamedKey::F2,
        NamedKey::F3,
        NamedKey::F4,
        NamedKey::F5,
        NamedKey::F6,
        NamedKey::F7,
        NamedKey::F8,
        NamedKey::F9,
        NamedKey::F10,
        NamedKey::F11,
        NamedKey::F12,
    ];
    KEYS.iter().position(|k| *k == key).map(|i| i as u8 + 1)
}

/// Attributes for the app's main window: title, size, and the maximized start.
fn main_window_attributes<A: KathamoApp>(app: &A) -> WindowAttributes {
    let (width, height) = app.default_size();
    WindowAttributes::default()
        .with_title(app.title())
        .with_inner_size(LogicalSize::new(width as f64, height as f64))
        .with_min_inner_size(LogicalSize::new(480.0, 320.0))
        .with_maximized(app.start_maximized())
        .with_visible(false)
}

impl<A: KathamoApp> ApplicationHandler for NativeAppRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        eprintln!("[Kathamo Native] resumed() event triggered, creating OS window...");
        match event_loop.create_window(main_window_attributes(&self.app)) {
            Ok(window) => {
                eprintln!("[Kathamo Native] OS window created: {:?}", window.id());
                let window = Arc::new(window);
                self.update_size(&window);
                // On Android this would pop up the on-screen keyboard; there the
                // app asks for it with `AppContext::set_text_input` instead.
                #[cfg(not(target_os = "android"))]
                window.set_ime_allowed(true);
                match SoftwarePresenter::new(window.clone()) {
                    Ok(presenter) => self.presenter = Some(presenter),
                    Err(err) => eprintln!("[Kathamo Native] software presenter unavailable: {err}"),
                }
                match GpuBackend::new_for_window(
                    window.clone(),
                    self.ctx.physical_size.0,
                    self.ctx.physical_size.1,
                ) {
                    Ok(mut gpu) => {
                        gpu.set_scale_factor(self.ctx.scale);
                        self.gpu = Some(gpu);
                        eprintln!("[Kathamo Native] GPU backend initialized.");
                    }
                    Err(err) => eprintln!("[Kathamo Native] GPU unavailable, using CPU: {err}"),
                }
                self.window = Some(window.clone());
                // Hidden Windows windows may not receive WM_PAINT/redraw events.
                // Bootstrap presentation directly, then reveal only real pixels.
                match self.present() {
                    Ok(()) => {
                        window.set_visible(true);
                        self.revealed = true;
                    }
                    Err(error) => eprintln!("Kathamo first frame: {error}"),
                }
                window.request_redraw();
                eprintln!("[Kathamo Native] Initial redraw requested.");
            }
            Err(err) => {
                eprintln!("[Kathamo Native] could not create window: {err}");
                event_loop.exit();
            }
        }
    }

    /// Android (and iOS) destroy the native surface when the app goes to the
    /// background. Drop everything tied to it; `resumed` builds it again, and the
    /// app state in `self.app` / `self.ctx` survives.
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            if crate::android::activity_is_finishing(app) {
                self.finishing = true;
                event_loop.exit();
            }
        }
        #[cfg(not(target_os = "android"))]
        let _ = event_loop;
        self.gpu = None;
        self.presenter = None;
        self.window = None;
        self.revealed = false;
        self.gpu_images_uploaded.clear();
        self.touch_id = None;
        self.keyboard_shown = false;
        for button in [
            PointerButton::Primary,
            PointerButton::Secondary,
            PointerButton::Middle,
        ] {
            self.ctx.pointer.on_button_up(button);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };
        if window.id() != id {
            self.secondary_window_event(event_loop, id, event);
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if !occluded {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.update_size(&window);
                window.request_redraw();
            }
            WindowEvent::Focused(false) => {
                self.ctx.pointer.on_button_up(PointerButton::Primary);
                self.ctx.pointer.on_button_up(PointerButton::Secondary);
                self.ctx.pointer.on_button_up(PointerButton::Middle);
                self.ctx.focus.clear();
                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.ctx.scale;
                let pos = Vec2::new(position.x as f32 / scale, position.y as f32 / scale);
                let delta = pos - self.ctx.pointer.position;
                self.ctx.pointer.on_move(pos);
                self.app.handle_mouse_move(pos, delta, &mut self.ctx);
                window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = mouse_button(button) {
                    let pos = self.ctx.pointer.position;
                    match state {
                        ElementState::Pressed => {
                            self.ctx.pointer.on_button_down(button);
                            self.app.handle_mouse_down(pos, button, &mut self.ctx);
                        }
                        ElementState::Released => {
                            self.app.handle_mouse_up(pos, button, &mut self.ctx);
                            self.ctx.pointer.on_button_up(button);
                        }
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::Touch(touch) => {
                let scale = self.ctx.scale;
                let pos = Vec2::new(
                    touch.location.x as f32 / scale,
                    touch.location.y as f32 / scale,
                );
                let delta = pos - self.ctx.pointer.position;
                match touch_action(&mut self.touch_id, touch.id, touch.phase) {
                    TouchAction::Press => {
                        // A finger has no hover: move there first so widgets see the
                        // press at the right place.
                        self.ctx.pointer.on_move(pos);
                        self.app.handle_mouse_move(pos, delta, &mut self.ctx);
                        self.ctx.pointer.on_button_down(PointerButton::Primary);
                        self.app
                            .handle_mouse_down(pos, PointerButton::Primary, &mut self.ctx);
                    }
                    TouchAction::Move => {
                        self.ctx.pointer.on_move(pos);
                        if !self.app.handle_mouse_move(pos, delta, &mut self.ctx) {
                            let scroll = touch_scroll_delta(delta);
                            self.ctx.pointer.scroll_delta = scroll;
                            self.app.handle_scroll(scroll, pos, &mut self.ctx);
                        }
                    }
                    TouchAction::Release => {
                        self.ctx.pointer.on_move(pos);
                        self.app
                            .handle_mouse_up(pos, PointerButton::Primary, &mut self.ctx);
                        self.ctx.pointer.on_button_up(PointerButton::Primary);
                    }
                    TouchAction::Ignore => {}
                }
                window.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        Vec2::new(p.x as f32 / 20.0, p.y as f32 / 20.0)
                    }
                };
                self.ctx.pointer.scroll_delta = delta;
                self.app
                    .handle_scroll(delta, self.ctx.pointer.position, &mut self.ctx);
                window.request_redraw();
            }
            WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled
            | WindowEvent::DroppedFile(_) => {
                if let Some(drop) = file_drop_event_of(event) {
                    self.file_drop_event(drop);
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                self.modifiers = Modifiers {
                    ctrl: state.control_key(),
                    shift: state.shift_key(),
                    alt: state.alt_key(),
                    command: state.super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Released => {
                let key = key_from_winit(&event.logical_key);
                self.app.handle_key_up(key, self.modifiers, &mut self.ctx);
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let key = key_from_winit(&event.logical_key);
                let consumed = self
                    .app
                    .handle_key_down(key.clone(), self.modifiers, &mut self.ctx);
                if !consumed && key == Key::Tab {
                    self.ctx.focus.cycle_focus(self.modifiers.shift);
                }
                if !consumed && key == Key::Custom("Back".into()) {
                    // Android: an unhandled Back closes the activity, as users
                    // expect. `suspended` then sees `isFinishing` and ends the loop.
                    #[cfg(target_os = "android")]
                    if let Some(app) = &self.android_app {
                        crate::android::finish_activity(app);
                        return;
                    }
                }
                if let Some(text) = event.text {
                    if accepts_key_text(text.as_str(), self.modifiers) {
                        self.app.handle_text_input(text.as_str(), &mut self.ctx);
                    }
                }
                window.request_redraw();
            }
            WindowEvent::Ime(ime) => {
                use ui_platform::ImeEvent;
                let event = match ime {
                    Ime::Preedit(text, cursor) => Some(ImeEvent::Preedit(text, cursor)),
                    Ime::Commit(text) => Some(ImeEvent::Commit(text)),
                    Ime::Disabled => Some(ImeEvent::Disabled),
                    Ime::Enabled => None,
                };
                if let Some(event) = event {
                    self.app.handle_ime(event, &mut self.ctx);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.sync_safe_area();
                match self.present() {
                    Ok(()) => {
                        if !self.revealed
                            && self.ctx.physical_size.0 > 0
                            && self.ctx.physical_size.1 > 0
                        {
                            window.set_visible(true);
                            self.revealed = true;
                        }
                    }
                    Err(err) => eprintln!("Kathamo frame presentation failed: {err}"),
                }
                if self.needs_next_frame() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
        self.deliver_file_drops();
        let requests = std::mem::take(&mut self.ctx.window_requests);
        self.process_window_requests(event_loop, requests);
    }

    /// A `UiWaker` fired (usually from a worker thread): draw a fresh frame.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Android: the keyboard opening or closing changes the content rect but
        // winit sends no event for it, so check once per batch of events.
        if self.sync_safe_area() {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        let wake_due = match self.wake_at {
            Some(t) if Instant::now() >= t => {
                self.wake_at = None;
                self.visible()
            }
            _ => false,
        };
        if !self.revealed || self.needs_next_frame() || wake_due {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(match self.wake_at {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
        for child in self.children.values() {
            if !child.revealed || self.app.wants_continuous_redraw() {
                child.window.request_redraw();
            }
        }
    }
}

/// The OS file-drag events as `FileDropEvent`s; `None` for any other window event.
fn file_drop_event_of(event: WindowEvent) -> Option<FileDropEvent> {
    match event {
        WindowEvent::HoveredFile(path) => Some(FileDropEvent::Hovered(path)),
        WindowEvent::HoveredFileCancelled => Some(FileDropEvent::HoverCancelled),
        WindowEvent::DroppedFile(path) => Some(FileDropEvent::Dropped(path)),
        _ => None,
    }
}

// Enter, Tab, Backspace and Delete are handled once through the key callback.
// Winit may also supply their control characters in KeyEvent.text on Windows.
fn accepts_key_text(text: &str, modifiers: Modifiers) -> bool {
    !modifiers.ctrl && !modifiers.command && !text.is_empty() && !text.chars().any(char::is_control)
}

struct SecondaryWindow {
    id: String,
    window: Arc<Window>,
    ctx: AppContext,
    gpu: Option<GpuBackend>,
    cpu: CpuBackend,
    presenter: Option<SoftwarePresenter>,
    gpu_images_uploaded: HashMap<u64, u64>,
    cpu_images_uploaded: HashMap<u64, u64>,
    modifiers: Modifiers,
    revealed: bool,
}

impl SecondaryWindow {
    fn new(
        event_loop: &ActiveEventLoop,
        id: String,
        title: String,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title(title)
                        .with_inner_size(LogicalSize::new(width as f64, height as f64))
                        .with_visible(true),
                )
                .map_err(|e| e.to_string())?,
        );
        window.set_ime_allowed(true);
        let scale = window.scale_factor() as f32;
        let size = window.inner_size();
        let mut ctx = AppContext::new(size.width, size.height, scale);
        ctx.window_id = id.clone();
        let presenter = SoftwarePresenter::new(window.clone()).ok();
        let gpu = GpuBackend::new_for_window(window.clone(), size.width, size.height)
            .ok()
            .map(|mut gpu| {
                gpu.set_scale_factor(scale);
                gpu
            });
        if gpu.is_none() && presenter.is_none() {
            return Err("no renderer available for secondary window".into());
        }
        let mut cpu = CpuBackend::new();
        cpu.set_scale_factor(scale);
        Ok(Self {
            id,
            window,
            ctx,
            gpu,
            cpu,
            presenter,
            gpu_images_uploaded: HashMap::new(),
            cpu_images_uploaded: HashMap::new(),
            modifiers: Modifiers::NONE,
            revealed: false,
        })
    }

    fn resize(&mut self) {
        let scale = self.window.scale_factor() as f32;
        let size = self.window.inner_size();
        self.ctx.resize(size.width, size.height, scale);
        if let Some(gpu) = &mut self.gpu {
            gpu.set_scale_factor(scale);
        }
        self.cpu.set_scale_factor(scale);
    }

    fn present<A: KathamoApp>(&mut self, app: &mut A) -> Result<(), String> {
        let (width, height) = self.ctx.physical_size;
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.ctx.cmd_buf.clear();
        self.ctx.frame_count = self.ctx.frame_count.wrapping_add(1);
        app.update_window(&self.id, &mut self.ctx);
        if let Some(gpu) = self.gpu.as_mut() {
            app.prepare_window_gpu(&self.id, gpu);
            for (id, image) in &self.ctx.images {
                if self.gpu_images_uploaded.get(id) != Some(&image.version) {
                    gpu.upload_rgba8_image(*id, image.width, image.height, &image.rgba)?;
                    self.gpu_images_uploaded.insert(*id, image.version);
                }
            }
            gpu.begin_frame(width, height, self.ctx.theme.canvas);
            gpu.submit(&self.ctx.cmd_buf);
            if gpu.end_frame().is_ok() {
                self.ctx.pointer.step_frame();
                return Ok(());
            }
            // A separate native window can fail independently of the main window.
            if gpu.recover().is_ok() {
                self.gpu_images_uploaded.clear();
                app.prepare_window_gpu(&self.id, gpu);
            } else {
                self.gpu = None;
            }
        }
        for (id, image) in &self.ctx.images {
            if self.cpu_images_uploaded.get(id) != Some(&image.version) {
                self.cpu
                    .upload_rgba8_image(*id, image.width, image.height, &image.rgba)?;
                self.cpu_images_uploaded.insert(*id, image.version);
            }
        }
        self.cpu.begin_frame(width, height, self.ctx.theme.canvas);
        self.cpu.submit(&self.ctx.cmd_buf);
        self.cpu.end_frame()?;
        self.presenter
            .as_mut()
            .ok_or("secondary window CPU presentation unavailable")?
            .present(self.cpu.pixels(), width, height)?;
        self.ctx.pointer.step_frame();
        Ok(())
    }
}

#[cfg(test)]
mod touch_tests {
    use super::*;

    #[test]
    fn first_finger_drives_the_pointer_and_others_are_ignored() {
        let mut active = None;
        assert_eq!(touch_action(&mut active, 7, TouchPhase::Started), TouchAction::Press);
        assert_eq!(touch_action(&mut active, 8, TouchPhase::Started), TouchAction::Ignore);
        assert_eq!(touch_action(&mut active, 8, TouchPhase::Moved), TouchAction::Ignore);
        assert_eq!(touch_action(&mut active, 7, TouchPhase::Moved), TouchAction::Move);
        assert_eq!(touch_action(&mut active, 8, TouchPhase::Ended), TouchAction::Ignore);
        assert_eq!(touch_action(&mut active, 7, TouchPhase::Cancelled), TouchAction::Release);
        assert_eq!(active, None);
        // After release a new finger takes over.
        assert_eq!(touch_action(&mut active, 8, TouchPhase::Started), TouchAction::Press);
    }

    #[test]
    fn unclaimed_drag_scrolls_with_the_finger() {
        assert_eq!(touch_scroll_delta(Vec2::new(0.0, 40.0)), Vec2::new(0.0, 2.0));
    }

    #[test]
    fn content_rect_becomes_a_logical_safe_area() {
        // Status bar 84 px and gesture bar 42 px on a 720x1600 @1.75 screen.
        let safe = safe_area_from_content_rect((0, 84, 720, 1558), (720, 1600), 1.75);
        assert_eq!(safe.min, Vec2::new(0.0, 48.0));
        assert_eq!(safe.max, Vec2::new(720.0 / 1.75, 1558.0 / 1.75));
        // Not reported yet, or nonsense: the whole window.
        let full = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 800.0));
        assert_eq!(safe_area_from_content_rect((0, 0, 0, 0), (800, 1600), 2.0), full);
        assert_eq!(safe_area_from_content_rect((900, 0, 1000, 10), (800, 1600), 2.0), full);
    }

    #[test]
    fn keyboard_closed_by_the_system_is_noticed_once() {
        let mut seen = false;
        // Requested, not on screen yet: nothing to report.
        assert!(!keyboard_was_dismissed(true, &mut seen, 868.0, 892.0));
        // On screen: it takes ~300 px.
        assert!(!keyboard_was_dismissed(true, &mut seen, 560.0, 892.0));
        // Space returns while still requested: the user closed it.
        assert!(keyboard_was_dismissed(true, &mut seen, 868.0, 892.0));
        assert!(!keyboard_was_dismissed(true, &mut seen, 868.0, 892.0));
        // Not requested: never reported.
        assert!(!keyboard_was_dismissed(false, &mut seen, 560.0, 892.0));
        assert!(!keyboard_was_dismissed(false, &mut seen, 868.0, 892.0));
    }

    #[test]
    fn app_titles_become_safe_folder_names() {
        assert_eq!(app_folder_name("Setu Remote"), "Setu_Remote");
        assert_eq!(app_folder_name("../x"), "x");
        assert_eq!(app_folder_name("সেতু"), "সেতু");
        assert_eq!(app_folder_name("///"), "app");
    }

    #[test]
    fn function_keys_arrive_as_numbered_keys() {
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::F1)), Key::F(1));
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::F12)), Key::F(12));
    }

    #[test]
    fn android_back_arrives_as_a_named_custom_key() {
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::BrowserBack)), Key::Custom("Back".into()));
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    struct Plain(bool);
    impl KathamoApp for Plain {
        fn start_maximized(&self) -> bool {
            self.0
        }
        fn update(&mut self, _ctx: &mut AppContext) {}
    }

    #[test]
    fn main_window_starts_maximized_only_when_the_app_asks() {
        assert!(!main_window_attributes(&Plain(false)).maximized);
        let attrs = main_window_attributes(&Plain(true));
        assert!(attrs.maximized);
        assert!(!attrs.visible, "still revealed only after the first frame");
    }
}

#[cfg(test)]
mod key_text_tests {
    use super::*;
    #[test]
    fn control_keys_do_not_become_text_or_duplicate_editor_actions() {
        for text in ["\t", "\r", "\n", "\u{8}", "\u{7f}", ""] {
            assert!(!accepts_key_text(text, Modifiers::default()));
        }
        assert!(accepts_key_text("ক", Modifiers::default()));
        assert!(accepts_key_text("A", Modifiers::default()));
        assert!(!accepts_key_text(
            "a",
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            }
        ));
    }
}

#[cfg(test)]
mod file_drop_tests {
    use super::*;

    #[derive(Default)]
    struct Sink {
        dropped: Vec<Vec<std::path::PathBuf>>,
        hovering: bool,
    }
    impl KathamoApp for Sink {
        fn update(&mut self, _ctx: &mut AppContext) {}
        fn handle_files_hovered(&mut self, _p: Vec<std::path::PathBuf>, _c: &mut AppContext) {
            self.hovering = true;
        }
        fn handle_files_hover_cancelled(&mut self, _c: &mut AppContext) {
            self.hovering = false;
        }
        fn handle_files_dropped(&mut self, p: Vec<std::path::PathBuf>, _c: &mut AppContext) -> bool {
            self.hovering = false;
            self.dropped.push(p);
            true
        }
    }

    /// The runner turns winit's file events into one app call per event batch.
    #[test]
    fn winit_file_events_reach_the_app_hooks() {
        let mut runner = NativeAppRunner::new(Sink::default());
        let hover = file_drop_event_of(WindowEvent::HoveredFile("a.png".into())).unwrap();
        runner.file_drop_event(hover);
        assert!(!runner.app.hovering, "nothing is delivered before the batch ends");
        runner.deliver_file_drops();
        assert!(runner.app.hovering);
        for name in ["a.png", "b.png"] {
            runner.file_drop_event(file_drop_event_of(WindowEvent::DroppedFile(name.into())).unwrap());
        }
        assert!(runner.deliver_file_drops());
        assert!(!runner.app.hovering);
        assert_eq!(
            runner.app.dropped,
            vec![vec![std::path::PathBuf::from("a.png"), std::path::PathBuf::from("b.png")]]
        );
        assert!(!runner.deliver_file_drops(), "an empty queue delivers nothing");
        assert!(file_drop_event_of(WindowEvent::Focused(true)).is_none());
    }
}
