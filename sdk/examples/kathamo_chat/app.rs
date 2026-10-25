//! Kathamo Chat — Application Coordinator & Trait Implementation
//!
//! Orchestrates workspace layout, deterministic simulation ticks, user events,
//! keyboard shortcuts, and state transitions according to KS Engineering Standard.

use super::chat_view::render_chat_view;
use super::composer::render_composer;
use super::mock_data::{create_prepopulated_conversations, generate_scripted_response};
use super::models::{
    ChatConversation, ChatMessageItem, ChatModel, CodeArtifact, StreamingState, UiState,
};
use super::preview_panel::render_preview_panel;
use super::sidebar::render_sidebar;
use super::top_bar::{render_model_dropdown, render_top_bar};

use kathamo::prelude::*;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;
use ui_widgets::rich_text::ChatRole;

pub struct KathamoChatApp {
    pub conversations: Vec<ChatConversation>,
    pub active_conv_id: String,
    pub selected_model: ChatModel,
    pub streaming: StreamingState,
    pub ui: UiState,
    pub window_size: Vec2,
}

impl KathamoChatApp {
    pub fn new() -> Self {
        let conversations = create_prepopulated_conversations();
        let active_conv_id = conversations
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_default();

        let initial_artifact = conversations
            .first()
            .and_then(|c| c.messages.iter().find_map(|m| m.artifact.clone()));

        Self {
            conversations,
            active_conv_id,
            selected_model: ChatModel::ClaudeFast,
            streaming: StreamingState::default(),
            ui: UiState {
                active_artifact: initial_artifact,
                ..UiState::default()
            },
            window_size: Vec2::new(1440.0, 900.0),
        }
    }

    pub fn active_conversation(&self) -> Option<&ChatConversation> {
        self.conversations
            .iter()
            .find(|c| c.id == self.active_conv_id)
    }

    pub fn active_conversation_mut(&mut self) -> Option<&mut ChatConversation> {
        self.conversations
            .iter_mut()
            .find(|c| c.id == self.active_conv_id)
    }

    pub fn select_conversation(&mut self, id: impl Into<String>) {
        let id = id.into();
        if let Some(conv) = self.conversations.iter().find(|c| c.id == id) {
            let model = conv.model;
            let art = conv.messages.iter().find_map(|m| m.artifact.clone());
            self.active_conv_id = id;
            self.ui.scroll_y = 0.0;
            self.selected_model = model;
            if let Some(a) = art {
                self.ui.active_artifact = Some(a);
            }
        }
    }

    pub fn create_new_conversation(&mut self) {
        let new_id = format!("conv_{}", self.conversations.len() + 1);
        let title = "New Conversation";
        let new_conv = ChatConversation::new(
            new_id.clone(),
            title,
            "Fresh empty conversation",
            self.selected_model,
            "Just now",
        );
        self.conversations.insert(0, new_conv);
        self.active_conv_id = new_id;
        self.ui.scroll_y = 0.0;
        self.set_toast("Created new conversation");
    }

    #[allow(dead_code)]
    pub fn delete_selected_conversation(&mut self) {
        if let Some(pos) = self
            .conversations
            .iter()
            .position(|c| c.id == self.active_conv_id)
        {
            self.conversations.remove(pos);
            if let Some(next) = self.conversations.first() {
                self.active_conv_id = next.id.clone();
            } else {
                self.active_conv_id.clear();
            }
            self.set_toast("Deleted conversation");
        }
    }

    pub fn send_message(&mut self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }

        // If no active conversation, create one
        if self.active_conversation().is_none() {
            self.create_new_conversation();
        }

        let user_msg_id = format!(
            "msg_u_{}",
            std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_millis()
        );
        let user_msg = ChatMessageItem::new(user_msg_id, ChatRole::User, trimmed, "Just now");

        // Generate response script
        let (scripted_reply, artifact) = generate_scripted_response(trimmed, self.selected_model);

        let bot_msg_id = format!(
            "msg_b_{}",
            std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_millis()
        );
        let mut bot_msg =
            ChatMessageItem::new(bot_msg_id.clone(), ChatRole::Assistant, "", "Just now");
        if let Some(art) = &artifact {
            bot_msg.artifact = Some(art.clone());
            self.ui.active_artifact = Some(art.clone());
        }

        if let Some(conv) = self.active_conversation_mut() {
            conv.add_message(user_msg);
            conv.add_message(bot_msg);
            // If title is default, update it from prompt
            if conv.title == "New Conversation" {
                conv.title = trimmed.chars().take(28).collect::<String>();
            }
        }

        // Initiate typewriter streaming
        self.streaming.start(
            self.active_conv_id.clone(),
            bot_msg_id,
            scripted_reply,
            artifact,
        );

        self.ui.composer_text.clear();
        self.ui.scroll_y = 1200.0; // scroll to bottom
    }

    pub fn stop_streaming(&mut self) {
        if self.streaming.is_active {
            // Commit whatever was typed so far
            let cur_text = self
                .streaming
                .full_response
                .chars()
                .take(self.streaming.current_char_index)
                .collect::<String>();
            let msg_id = self.streaming.message_id.clone();
            if let Some(conv) = self.active_conversation_mut() {
                if let Some(msg) = conv.messages.iter_mut().find(|m| m.id == msg_id) {
                    msg.content = cur_text;
                    msg.is_streaming = false;
                }
            }
            self.streaming.stop();
            self.set_toast("Response generation stopped");
        }
    }

    pub fn regenerate_response(&mut self) {
        let last_user_prompt = self.active_conversation().and_then(|c| {
            c.messages
                .iter()
                .rev()
                .find(|m| m.role == ChatRole::User)
                .map(|m| m.content.clone())
        });

        if let Some(prompt) = last_user_prompt {
            self.send_message(&prompt);
            self.set_toast("Regenerating response...");
        }
    }

    pub fn copy_to_clipboard(&mut self, text: &str) {
        self.ui.clipboard_buffer = text.to_string();
        self.set_toast("Copied to clipboard!");
    }

    pub fn set_toast(&mut self, text: impl Into<String>) {
        self.ui.toast_message = Some((text.into(), 120)); // ~2 seconds at 60fps
    }

    pub fn set_model(&mut self, model: ChatModel) {
        self.selected_model = model;
        if let Some(conv) = self.active_conversation_mut() {
            conv.model = model;
        }
        self.ui.model_dropdown_open = false;
        self.set_toast(format!("Switched to {}", model.short_name()));
    }

    pub fn toggle_sidebar(&mut self) {
        self.ui.sidebar_expanded = !self.ui.sidebar_expanded;
    }

    pub fn toggle_preview_panel(&mut self) {
        self.ui.preview_panel_open = !self.ui.preview_panel_open;
    }

    pub fn set_active_artifact(&mut self, art: CodeArtifact) {
        self.ui.active_artifact = Some(art);
        self.ui.preview_panel_open = true;
    }
}

impl KathamoApp for KathamoChatApp {
    fn title(&self) -> &str {
        "Kathamo Chat — Native Desktop AI Interaction Showcase"
    }

    fn default_size(&self) -> (u32, u32) {
        (1440, 900)
    }

    fn wants_continuous_redraw(&self) -> bool {
        self.streaming.is_active || self.ui.toast_message.is_some()
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let bounds = ctx.bounds();
        self.window_size = bounds.size();

        // 1. Advance typewriter simulation if active
        if self.streaming.is_active {
            if let Some(partial) = self.streaming.advance_step() {
                let msg_id = self.streaming.message_id.clone();
                if let Some(conv) = self.active_conversation_mut() {
                    if let Some(msg) = conv.messages.iter_mut().find(|m| m.id == msg_id) {
                        msg.content = partial;
                    }
                }
            } else {
                // Done streaming
                let full = self.streaming.full_response.clone();
                let msg_id = self.streaming.message_id.clone();
                if let Some(conv) = self.active_conversation_mut() {
                    if let Some(msg) = conv.messages.iter_mut().find(|m| m.id == msg_id) {
                        msg.content = full;
                        msg.is_streaming = false;
                    }
                }
                self.streaming.stop();
            }
        }

        // Decrement toast lifetime
        if let Some((_, remaining)) = &mut self.ui.toast_message {
            if *remaining > 0 {
                *remaining -= 1;
            } else {
                self.ui.toast_message = None;
            }
        }

        // 2. Compute Layout Regions
        let win_w = self.window_size.x;
        let win_h = self.window_size.y;

        let sidebar_w = if self.ui.sidebar_expanded {
            250.0
        } else {
            58.0
        };
        let preview_w = if self.ui.preview_panel_open {
            420.0
        } else {
            0.0
        };

        let sidebar_rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(sidebar_w, win_h));
        let center_x = sidebar_w;
        let center_w = (win_w - sidebar_w - preview_w).max(300.0);

        let top_bar_h = 48.0;
        let composer_h = 100.0;
        let top_bar_rect =
            Rect::from_min_size(Vec2::new(center_x, 0.0), Vec2::new(center_w, top_bar_h));
        let composer_rect = Rect::from_min_size(
            Vec2::new(center_x, win_h - composer_h),
            Vec2::new(center_w, composer_h),
        );
        let chat_view_rect = Rect::from_min_size(
            Vec2::new(center_x, top_bar_h),
            Vec2::new(center_w, win_h - top_bar_h - composer_h),
        );

        let preview_rect = Rect::from_min_size(
            Vec2::new(win_w - preview_w, 0.0),
            Vec2::new(preview_w, win_h),
        );

        let pal = ctx.theme;

        // 3. Render Components
        let active_conv = self.active_conversation().cloned();

        // Left Sidebar
        render_sidebar(
            &mut ctx.cmd_buf,
            sidebar_rect,
            &self.conversations,
            &self.active_conv_id,
            &self.ui,
            &pal,
        );

        // Top Navigation Bar
        render_top_bar(
            &mut ctx.cmd_buf,
            top_bar_rect,
            active_conv.as_ref(),
            self.selected_model,
            &self.ui,
            &pal,
        );

        // Center Message Stream
        let chat_res = render_chat_view(
            &mut ctx.cmd_buf,
            chat_view_rect,
            active_conv.as_ref(),
            &self.streaming,
            &self.ui,
            &pal,
        );

        if let Some(starter) = chat_res.clicked_starter_prompt {
            self.send_message(&starter);
        }
        if let Some(copy_txt) = chat_res.clicked_copy_msg {
            self.copy_to_clipboard(&copy_txt);
        }
        if chat_res.clicked_regenerate {
            self.regenerate_response();
        }
        if let Some(art) = chat_res.clicked_view_artifact {
            self.set_active_artifact(art);
        }

        // Bottom Message Composer
        render_composer(
            &mut ctx.cmd_buf,
            composer_rect,
            &self.streaming,
            &self.ui,
            &pal,
        );

        // Optional Right Preview Panel (Canvas / Artifacts)
        if self.ui.preview_panel_open {
            render_preview_panel(
                &mut ctx.cmd_buf,
                preview_rect,
                self.ui.active_artifact.as_ref(),
                &self.ui,
                &pal,
            );
        }

        // Model Switcher Dropdown (Floating Overlay)
        if self.ui.model_dropdown_open {
            render_model_dropdown(&mut ctx.cmd_buf, top_bar_rect, self.selected_model, &pal);
        }

        // Toast Feedback Notification Banner
        if let Some((msg, _)) = &self.ui.toast_message {
            let toast_w = 260.0;
            let toast_h = 32.0;
            let toast_rect = Rect::from_min_size(
                Vec2::new(center_x + (center_w - toast_w) * 0.5, top_bar_h + 16.0),
                Vec2::new(toast_w, toast_h),
            );
            ctx.cmd_buf.push(DrawCommand::Rect {
                rect: toast_rect,
                fill: pal.canvas,
                radius: CornerRadius::same(6.0),
                stroke: Stroke::new(1.0, pal.accent),
            });
            ctx.cmd_buf.push(DrawCommand::Text {
                pos: Vec2::new(toast_rect.min.x + 14.0, toast_rect.min.y + 8.0),
                text: format!("✓  {}", msg),
                color: pal.accent,
                size: 11.5,
                strong: true,
            });
        }
    }

    fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        _button: PointerButton,
        _ctx: &mut AppContext,
    ) -> bool {
        let win_w = self.window_size.x;
        let win_h = self.window_size.y;
        let sidebar_w = if self.ui.sidebar_expanded {
            250.0
        } else {
            58.0
        };
        let preview_w = if self.ui.preview_panel_open {
            420.0
        } else {
            0.0
        };
        let center_x = sidebar_w;
        let center_w = (win_w - sidebar_w - preview_w).max(300.0);
        let top_bar_h = 48.0;

        // Model dropdown selection hit test
        if self.ui.model_dropdown_open {
            let menu_w = 260.0;
            let menu_x = center_x + (center_w - menu_w) * 0.5;
            let menu_y = top_bar_h + 4.0;
            let mut item_y = menu_y + 8.0;
            for model in ChatModel::all() {
                let item_rect = Rect::from_min_size(
                    Vec2::new(menu_x + 6.0, item_y),
                    Vec2::new(menu_w - 12.0, 36.0),
                );
                if item_rect.contains(pos) {
                    self.set_model(model);
                    return true;
                }
                item_y += 40.0;
            }
            self.ui.model_dropdown_open = false;
            return true;
        }

        // Top bar model pill toggle hit test
        let model_pill_w = 210.0;
        let model_pill_rect = Rect::from_min_size(
            Vec2::new(center_x + (center_w - model_pill_w) * 0.5, 9.0),
            Vec2::new(model_pill_w, 30.0),
        );
        if model_pill_rect.contains(pos) {
            self.ui.model_dropdown_open = !self.ui.model_dropdown_open;
            return true;
        }

        // Top bar preview toggle hit test
        let prev_btn_rect = Rect::from_min_size(
            Vec2::new(center_x + center_w - 18.0 - 110.0, 10.0),
            Vec2::new(110.0, 28.0),
        );
        if prev_btn_rect.contains(pos) {
            self.toggle_preview_panel();
            return true;
        }

        // Sidebar collapse button hit test
        if self.ui.sidebar_expanded {
            let collapse_btn =
                Rect::from_min_size(Vec2::new(sidebar_w - 32.0, 14.0), Vec2::new(24.0, 24.0));
            if collapse_btn.contains(pos) {
                self.toggle_sidebar();
                return true;
            }
        } else {
            let expand_btn = Rect::from_min_size(Vec2::new(8.0, 12.0), Vec2::new(42.0, 32.0));
            if expand_btn.contains(pos) {
                self.toggle_sidebar();
                return true;
            }
        }

        // Sidebar New Chat button hit test
        let new_chat_btn = Rect::from_min_size(
            Vec2::new(if self.ui.sidebar_expanded { 14.0 } else { 8.0 }, 54.0),
            Vec2::new(
                sidebar_w - if self.ui.sidebar_expanded { 28.0 } else { 16.0 },
                36.0,
            ),
        );
        if new_chat_btn.contains(pos) {
            self.create_new_conversation();
            return true;
        }

        // Sidebar conversation list hit test
        if pos.x <= sidebar_w && pos.y > 90.0 && pos.y < win_h - 56.0 {
            let mut list_y = if self.ui.sidebar_expanded {
                152.0
            } else {
                98.0
            };
            let query_lower = self.ui.search_query.to_lowercase();
            for conv in &self.conversations {
                if !query_lower.is_empty() && !conv.title.to_lowercase().contains(&query_lower) {
                    continue;
                }
                let item_h = if self.ui.sidebar_expanded { 48.0 } else { 36.0 };
                let item_r = Rect::from_min_size(
                    Vec2::new(if self.ui.sidebar_expanded { 14.0 } else { 8.0 }, list_y),
                    Vec2::new(
                        sidebar_w - if self.ui.sidebar_expanded { 28.0 } else { 16.0 },
                        item_h,
                    ),
                );
                if item_r.contains(pos) {
                    self.select_conversation(conv.id.clone());
                    return true;
                }
                list_y += item_h + 4.0;
            }
        }

        // Composer Send / Stop button hit test
        let composer_y = win_h - 100.0;
        let card_w = (center_w - 80.0).min(780.0).max(320.0);
        let card_x = center_x + (center_w - card_w) * 0.5;
        let btn_r = Rect::from_min_size(
            Vec2::new(card_x + card_w - 78.0 - 12.0, composer_y + 44.0 - 1.0),
            Vec2::new(78.0, 26.0),
        );
        if btn_r.contains(pos) {
            if self.streaming.is_active {
                self.stop_streaming();
            } else {
                let text = self.ui.composer_text.clone();
                self.send_message(&text);
            }
            return true;
        }

        // Composer attachment demo button hit test
        let attach_r = Rect::from_min_size(
            Vec2::new(card_x + 12.0, composer_y + 44.0),
            Vec2::new(110.0, 24.0),
        );
        if attach_r.contains(pos) {
            self.set_toast("📎 Attachments available in full release");
            return true;
        }

        // Preview panel close button hit test
        if self.ui.preview_panel_open && pos.x >= win_w - preview_w {
            let close_r = Rect::from_min_size(Vec2::new(win_w - 28.0, 10.0), Vec2::new(20.0, 20.0));
            if close_r.contains(pos) {
                self.ui.preview_panel_open = false;
                return true;
            }
            let copy_r = Rect::from_min_size(Vec2::new(win_w - 94.0, 10.0), Vec2::new(60.0, 24.0));
            if copy_r.contains(pos) {
                if let Some(art) = self.ui.active_artifact.clone() {
                    self.copy_to_clipboard(&art.code);
                }
                return true;
            }
        }

        // Welcome screen prompt starter cards hit test
        if let Some(conv) = self.active_conversation() {
            if conv.messages.is_empty() {
                let center_y = top_bar_h + (win_h - top_bar_h - 100.0) * 0.35;
                let start_y = center_y + 54.0;
                let col_w = (center_w - 80.0).min(780.0).max(320.0);
                let col_x = center_x + (center_w - col_w) * 0.5;
                let card_w = (col_w - 16.0) * 0.5;
                let starters = [
                    "How do I structure a concurrent ray-box intersection in Rust using Rayon?",
                    "Can you break down the visual layer stack for a 2.5D cyberpunk alleyway?",
                    "Outline the key factions for a sci-fi universe after the Great Aurora.",
                    "কাঠামো ফ্রেমওয়ার্কে বাংলা টেক্সট রেন্ডারিং কেমন কাজ করে?",
                ];
                for (i, prompt) in starters.iter().enumerate() {
                    let row = i / 2;
                    let col = i % 2;
                    let cx = col_x + col as f32 * (card_w + 16.0);
                    let cy = start_y + row as f32 * (72.0 + 12.0);
                    let rect = Rect::from_min_size(Vec2::new(cx, cy), Vec2::new(card_w, 72.0));
                    if rect.contains(pos) {
                        self.send_message(prompt);
                        return true;
                    }
                }
            }
        }

        false
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, _ctx: &mut AppContext) -> bool {
        let win_w = self.window_size.x;
        let preview_w = if self.ui.preview_panel_open {
            420.0
        } else {
            0.0
        };

        if self.ui.preview_panel_open && pos.x >= win_w - preview_w {
            self.ui.preview_scroll_y = (self.ui.preview_scroll_y - delta.y * 24.0).max(0.0);
            return true;
        }

        // Central chat scroll
        self.ui.scroll_y = (self.ui.scroll_y - delta.y * 24.0).max(0.0);
        true
    }

    fn handle_key_down(&mut self, key: Key, modifiers: Modifiers, _ctx: &mut AppContext) -> bool {
        // Ctrl+N: New Chat
        if modifiers.ctrl && matches!(key, Key::Character('n') | Key::Character('N')) {
            self.create_new_conversation();
            return true;
        }

        // Ctrl+K: Focus Search
        if modifiers.ctrl && matches!(key, Key::Character('k') | Key::Character('K')) {
            self.ui.search_focused = true;
            self.ui.composer_focused = false;
            return true;
        }

        // Escape: Dismiss overlays
        if matches!(key, Key::Escape) {
            if self.ui.model_dropdown_open {
                self.ui.model_dropdown_open = false;
                return true;
            }
            if self.ui.preview_panel_open {
                self.ui.preview_panel_open = false;
                return true;
            }
        }

        // Enter: Send Message
        if matches!(key, Key::Enter) {
            if self.ui.composer_focused && !self.ui.composer_text.trim().is_empty() {
                let text = self.ui.composer_text.clone();
                self.send_message(&text);
                return true;
            }
        }

        // Backspace: Delete character
        if matches!(key, Key::Backspace) {
            if self.ui.composer_focused && !self.ui.composer_text.is_empty() {
                self.ui.composer_text.pop();
                return true;
            }
            if self.ui.search_focused && !self.ui.search_query.is_empty() {
                self.ui.search_query.pop();
                return true;
            }
        }

        false
    }

    fn handle_text_input(&mut self, text: &str, _ctx: &mut AppContext) -> bool {
        if self.ui.composer_focused {
            self.ui.composer_text.push_str(text);
            return true;
        }
        if self.ui.search_focused {
            self.ui.search_query.push_str(text);
            return true;
        }
        false
    }
}
