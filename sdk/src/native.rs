//! Native window runner for applications using the public Kathamo SDK.

use std::sync::Arc;

use ui_core::Vec2;
use ui_platform::{
    winit::{
        application::ApplicationHandler,
        dpi::LogicalSize,
        event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        keyboard::{Key as WinitKey, NamedKey},
        window::{Window, WindowAttributes, WindowId},
    },
    Key, Modifiers, PointerButton,
};
use ui_render::{CpuBackend, GpuBackend, RenderBackend, SoftwarePresenter};

use crate::app::{AppContext, KathamoApp};

/// Runs a `KathamoApp` in a real OS window, with GPU rendering and CPU fallback.
pub fn run_native<A: KathamoApp>(app: A) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut runner = NativeAppRunner::new(app);
    event_loop.run_app(&mut runner).map_err(|e| e.to_string())
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
}

impl<A: KathamoApp> NativeAppRunner<A> {
    pub fn new(mut app: A) -> Self {
        let (width, height) = app.default_size();
        let mut ctx = AppContext::new(width, height, 1.0);
        app.init(&mut ctx);
        Self {
            app,
            ctx,
            window: None,
            gpu: None,
            cpu: CpuBackend::new(),
            presenter: None,
            modifiers: Modifiers::NONE,
            revealed: false,
        }
    }

    fn update_size(&mut self, window: &Window) {
        let size = window.inner_size();
        let scale = window.scale_factor() as f32;
        self.ctx.resize(size.width, size.height, scale);
        if let Some(gpu) = &mut self.gpu {
            gpu.set_scale_factor(scale);
        }
        self.cpu.set_scale_factor(scale);
    }

    fn present(&mut self) -> Result<(), String> {
        let (width, height) = self.ctx.physical_size;
        if width == 0 || height == 0 {
            return Ok(());
        }

        self.ctx.cmd_buf.clear();
        self.ctx.frame_count = self.ctx.frame_count.wrapping_add(1);
        self.app.update(&mut self.ctx);

        if let Some(gpu) = &mut self.gpu {
            gpu.begin_frame(width, height, self.ctx.theme.canvas);
            gpu.submit(&self.ctx.cmd_buf);
            if gpu.end_frame().is_ok() {
                self.ctx.pointer.step_frame();
                return Ok(());
            }
            // A lost GPU device gets one recovery attempt before software fallback.
            if gpu.recover().is_ok() {
                gpu.set_scale_factor(self.ctx.scale);
                gpu.begin_frame(width, height, self.ctx.theme.canvas);
                gpu.submit(&self.ctx.cmd_buf);
                if gpu.end_frame().is_ok() {
                    self.ctx.pointer.step_frame();
                    return Ok(());
                }
            }
            self.gpu = None;
        }

        self.cpu.begin_frame(width, height, self.ctx.theme.canvas);
        self.cpu.submit(&self.ctx.cmd_buf);
        self.cpu.end_frame()?;
        let presenter = self.presenter.as_mut().ok_or("CPU window presenter unavailable")?;
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
        _ => Key::Custom(format!("{key:?}")),
    }
}

impl<A: KathamoApp> ApplicationHandler for NativeAppRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (width, height) = self.app.default_size();
        let attributes = WindowAttributes::default()
            .with_title(self.app.title())
            .with_inner_size(LogicalSize::new(width as f64, height as f64))
            .with_min_inner_size(LogicalSize::new(480.0, 320.0))
            .with_visible(false);

        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                self.update_size(&window);
                window.set_ime_allowed(true);
                match SoftwarePresenter::new(window.clone()) {
                    Ok(presenter) => self.presenter = Some(presenter),
                    Err(err) => eprintln!("Kathamo software presenter unavailable: {err}"),
                }
                match GpuBackend::new_for_window(
                    window.clone(), self.ctx.physical_size.0, self.ctx.physical_size.1,
                ) {
                    Ok(mut gpu) => {
                        gpu.set_scale_factor(self.ctx.scale);
                        self.gpu = Some(gpu);
                    }
                    Err(err) => eprintln!("Kathamo GPU unavailable, using CPU: {err}"),
                }
                self.window = Some(window.clone());
                window.request_redraw();
            }
            Err(err) => {
                eprintln!("Kathamo could not create a window: {err}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref().cloned() else { return; };
        if window.id() != id { return; }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
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
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(p) => {
                        Vec2::new(p.x as f32 / 20.0, p.y as f32 / 20.0)
                    }
                };
                self.ctx.pointer.scroll_delta = delta;
                self.app.handle_scroll(delta, self.ctx.pointer.position, &mut self.ctx);
                window.request_redraw();
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
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let key = key_from_winit(&event.logical_key);
                let consumed = self.app.handle_key_down(key.clone(), self.modifiers, &mut self.ctx);
                if !consumed && key == Key::Tab {
                    self.ctx.focus.cycle_focus(self.modifiers.shift);
                }
                if let Some(text) = event.text {
                    self.app.handle_text_input(text.as_str(), &mut self.ctx);
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
                match self.present() {
                    Ok(()) => {
                        if !self.revealed && self.ctx.physical_size.0 > 0 && self.ctx.physical_size.1 > 0 {
                            window.set_visible(true);
                            self.revealed = true;
                        }
                    }
                    Err(err) => eprintln!("Kathamo frame presentation failed: {err}"),
                }
                if self.app.wants_continuous_redraw() {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.revealed || self.app.wants_continuous_redraw() {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}
