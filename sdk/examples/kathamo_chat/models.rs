//! Kathamo Chat — Data Models & Architecture
//!
//! Complies with Rob Pike's Rule 5 ("Data Dominates") and KS Application
//! Architecture Standard ("Content is Data, Systems are Code").

use ui_widgets::rich_text::ChatRole;

/// Supported simulated AI models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatModel {
    ClaudeFast,
    LlamaLocal,
    KathamoCore,
}

impl ChatModel {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeFast => "Claude 3.5 Sonnet (Fast)",
            Self::LlamaLocal => "Local Llama 3 (8B)",
            Self::KathamoCore => "Kathamo Agent Core",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::ClaudeFast => "Claude 3.5",
            Self::LlamaLocal => "Llama-3-8B",
            Self::KathamoCore => "Kathamo Core",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::ClaudeFast => "Cloud • 200K",
            Self::LlamaLocal => "Local GPU • 8K",
            Self::KathamoCore => "Built-in • Fast",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::ClaudeFast, Self::LlamaLocal, Self::KathamoCore]
    }
}

/// A standalone code or document artifact displayed in the side panel.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeArtifact {
    pub id: String,
    pub title: String,
    pub language: String,
    pub code: String,
    pub description: String,
}

/// A single message within a conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessageItem {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub timestamp: String,
    pub is_streaming: bool,
    pub artifact: Option<CodeArtifact>,
}

impl ChatMessageItem {
    pub fn new(
        id: impl Into<String>,
        role: ChatRole,
        content: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            content: content.into(),
            timestamp: timestamp.into(),
            is_streaming: false,
            artifact: None,
        }
    }

    pub fn with_artifact(mut self, artifact: CodeArtifact) -> Self {
        self.artifact = Some(artifact);
        self
    }
}

/// A complete conversation history session.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatConversation {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub model: ChatModel,
    pub updated_at: String,
    pub is_pinned: bool,
    pub messages: Vec<ChatMessageItem>,
}

impl ChatConversation {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        model: ChatModel,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            model,
            updated_at: updated_at.into(),
            is_pinned: false,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessageItem) {
        self.messages.push(message);
    }
}

/// Deterministic typewriter streaming simulator.
#[derive(Debug, Clone)]
pub struct StreamingState {
    pub is_active: bool,
    pub conversation_id: String,
    pub message_id: String,
    pub full_response: String,
    pub current_char_index: usize,
    pub chars_per_tick: usize,
    pub tick_counter: usize,
    pub artifact: Option<CodeArtifact>,
}

impl Default for StreamingState {
    fn default() -> Self {
        Self {
            is_active: false,
            conversation_id: String::new(),
            message_id: String::new(),
            full_response: String::new(),
            current_char_index: 0,
            chars_per_tick: 4,
            tick_counter: 0,
            artifact: None,
        }
    }
}

impl StreamingState {
    pub fn start(
        &mut self,
        conversation_id: impl Into<String>,
        message_id: impl Into<String>,
        full_response: impl Into<String>,
        artifact: Option<CodeArtifact>,
    ) {
        self.is_active = true;
        self.conversation_id = conversation_id.into();
        self.message_id = message_id.into();
        self.full_response = full_response.into();
        self.current_char_index = 0;
        self.tick_counter = 0;
        self.artifact = artifact;
    }

    pub fn stop(&mut self) {
        self.is_active = false;
        self.conversation_id.clear();
        self.message_id.clear();
        self.full_response.clear();
        self.current_char_index = 0;
        self.artifact = None;
    }

    /// Advances the stream by characters, returning newly revealed slice or None if done.
    pub fn advance_step(&mut self) -> Option<String> {
        if !self.is_active {
            return None;
        }

        let total_chars = self.full_response.chars().count();
        if self.current_char_index >= total_chars {
            self.is_active = false;
            return None;
        }

        let next_idx = (self.current_char_index + self.chars_per_tick).min(total_chars);
        self.current_char_index = next_idx;
        let partial: String = self.full_response.chars().take(next_idx).collect();
        Some(partial)
    }
}

/// Transient UI layout and interaction state.
#[derive(Debug, Clone)]
pub struct UiState {
    pub sidebar_expanded: bool,
    pub preview_panel_open: bool,
    pub active_artifact: Option<CodeArtifact>,
    pub search_query: String,
    pub search_focused: bool,
    pub composer_text: String,
    pub composer_focused: bool,
    pub scroll_y: f32,
    pub preview_scroll_y: f32,
    pub model_dropdown_open: bool,
    pub toast_message: Option<(String, usize)>, // (text, frames_remaining)
    pub clipboard_buffer: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sidebar_expanded: true,
            preview_panel_open: false,
            active_artifact: None,
            search_query: String::new(),
            search_focused: false,
            composer_text: String::new(),
            composer_focused: true,
            scroll_y: 0.0,
            preview_scroll_y: 0.0,
            model_dropdown_open: false,
            toast_message: None,
            clipboard_buffer: String::new(),
        }
    }
}
