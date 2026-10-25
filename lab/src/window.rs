//! Native OS Window Lifecycle and Winit 0.30 Event Loop Adapter.

use std::sync::Arc;
use ui_core::*;
use ui_platform::winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};
use ui_platform::*;
use ui_render::{BackendKind, GpuBackend, RenderBackend, SoftwarePresenter};

use crate::app::UiLabApp;
use crate::tabs::LabTab;

/// Interactive native OS window running Kolpona UI Lab with winit 0.30.
pub struct LabWindowApp {
    pub app: UiLabApp,
    pub window: Option<Arc<Window>>,
    pub viewport: Rect,
    pub revealed: bool,
}

impl LabWindowApp {
    pub fn new(app: UiLabApp) -> Self {
        Self {
            app,
            window: None,
            viewport: Rect::from_min_size(Vec2::ZERO, Vec2::new(1440.0, 900.0)),
            revealed: false,
        }
    }
}

impl ApplicationHandler for LabWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let width = 1440.0;
            let height = 900.0;
            // Zero-White-Screen Fence: Window starts HIDDEN with dark canvas brush
            let attributes = WindowAttributes::default()
                .with_title("Kalponic Studio // Kolpona UI Lab")
                .with_inner_size(LogicalSize::new(width, height))
                .with_min_inner_size(LogicalSize::new(800.0, 500.0))
                .with_visible(false);

            match event_loop.create_window(attributes) {
                Ok(win) => {
                    let win = Arc::new(win);
                    let scale = win.scale_factor() as f32;
                    let size = win.inner_size();
                    self.app.scale = scale;
                    self.app.physical_size = (size.width, size.height);
                    self.viewport = Rect::from_min_size(
                        Vec2::ZERO,
                        Vec2::new(size.width as f32 / scale, size.height as f32 / scale),
                    );

                    self.app.is_interactive = true;
                    // Initialize SoftwarePresenter for CPU Safe-Mode presentation
                    match SoftwarePresenter::new(win.clone()) {
                        Ok(presenter) => self.app.soft_presenter = Some(presenter),
                        Err(e) => {
                            eprintln!("⚠️ Failed to initialize SoftwarePresenter for window: {e}");
                            self.app.soft_presenter = None;
                        }
                    }

                    // Initialize hardware GPU backend bound to the native OS window
                    match GpuBackend::new_for_window(win.clone(), size.width, size.height) {
                        Ok(mut gpu) => {
                            gpu.set_scale_factor(scale);
                            self.app.gpu = Some(gpu);
                            self.app.active_backend = BackendKind::HardwareGpu;
                        }
                        Err(e) => {
                            eprintln!(
                                "⚠️ Failed to initialize window GPU surface: {e}. Using software CPU safe mode."
                            );
                            self.app.active_backend = BackendKind::SoftwareCpuSafeMode;
                        }
                    }

                    self.window = Some(win.clone());
                    // Explicitly request initial redraw so hidden window receives its first paint
                    win.request_redraw();
                }
                Err(e) => {
                    eprintln!("Failed to create OS window: {e}");
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref().cloned() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                let scale = window.scale_factor() as f32;
                self.app.scale = scale;
                self.app.physical_size = (physical_size.width, physical_size.height);
                self.viewport = Rect::from_min_size(
                    Vec2::ZERO,
                    Vec2::new(
                        physical_size.width as f32 / scale,
                        physical_size.height as f32 / scale,
                    ),
                );
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scale = scale_factor as f32;
                self.app.scale = scale;
                let size = window.inner_size();
                self.app.physical_size = (size.width, size.height);
                self.viewport = Rect::from_min_size(
                    Vec2::ZERO,
                    Vec2::new(size.width as f32 / scale, size.height as f32 / scale),
                );
                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = window.scale_factor() as f32;
                let logical_pos = Vec2::new(position.x as f32 / scale, position.y as f32 / scale);
                self.app.handle_mouse_move(logical_pos, self.viewport);
                window.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    WinitMouseButton::Left => PointerButton::Primary,
                    WinitMouseButton::Right => PointerButton::Secondary,
                    WinitMouseButton::Middle => PointerButton::Middle,
                    _ => PointerButton::Primary,
                };
                match state {
                    ElementState::Pressed => {
                        self.app
                            .handle_mouse_down(self.app.pointer.position, btn, self.viewport);
                    }
                    ElementState::Released => {
                        self.app
                            .handle_mouse_up(self.app.pointer.position, btn, self.viewport);
                    }
                }
                window.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_vec = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(pos) => {
                        Vec2::new(pos.x as f32 / 20.0, pos.y as f32 / 20.0)
                    }
                };
                self.app.handle_scroll(scroll_vec, self.viewport);
                window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state == ElementState::Pressed {
                    use ui_platform::winit::keyboard::{Key as WinitKey, NamedKey};
                    match key_event.logical_key {
                        WinitKey::Named(NamedKey::Delete)
                        | WinitKey::Named(NamedKey::Backspace) => {
                            if self.app.current_tab == LabTab::NodeEditor {
                                self.app.node_editor.delete_selected();
                                window.request_redraw();
                            }
                        }
                        WinitKey::Named(NamedKey::Escape) => {
                            if self.app.current_tab == LabTab::NodeEditor {
                                self.app.node_editor.pending_wire = None;
                                self.app.node_editor.selected_node = None;
                                window.request_redraw();
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Render the frame into the swapchain or software presenter
                match self.app.render_frame(self.viewport) {
                    Ok(_stats) => {
                        // Zero-white-screen reveal: show window strictly after first frame is safely presented
                        if !self.revealed {
                            window.set_visible(true);
                            self.revealed = true;
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "⚠️ Frame presentation failed: {e}. Window kept hidden until valid frame is presented."
                        );
                    }
                }

                // If continuous animations (like kinetic fling) are active, request next frame
                if self.app.virtual_list.velocity.abs() > 0.5 {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(win) = &self.window {
            // Guarantee initial redraw occurs even before first user interaction
            if !self.revealed || self.app.virtual_list.velocity.abs() > 0.5 {
                win.request_redraw();
            }
        }
    }
}
