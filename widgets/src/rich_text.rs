//! Application-neutral Markdown presentation and streaming conversation widgets.
//! Data comes from the consumer; neither widget invokes or implements a model backend.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarkdownView {
    pub source: String,
    pub scroll_y: f32,
}

impl MarkdownView {
    pub fn new(markdown: impl Into<String>) -> Self {
        Self { source: markdown.into(), scroll_y: 0.0 }
    }
    pub fn set_source(&mut self, text: impl Into<String>) { self.source = text.into(); }
    pub fn append(&mut self, chunk: &str) { self.source.push_str(chunk); }

    /// Render headings, lists, quotes, fenced code, and ordinary paragraphs.
    /// This is a display widget, not a full Markdown parser or rich-text editor.
    pub fn render(&self, buf: &mut CommandBuffer, bounds: Rect, pal: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: bounds });
        let mut y = bounds.min.y + 12.0 - self.scroll_y.max(0.0);
        let mut code = false;
        for line in self.source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                code = !code;
                if !trimmed[3..].is_empty() {
                    draw_text(buf, trimmed[3..].to_owned(),
                        Vec2::new(bounds.min.x + 12.0, y), 10.0, pal.text_muted, true);
                }
                y += 21.0;
                continue;
            }
            if trimmed.is_empty() { y += 13.0; continue; }
            let (prefix, text, size, foreground, strong) = if code {
                ("", line, 12.0, pal.accent_hover, false)
            } else if let Some(t) = trimmed.strip_prefix("### ") {
                ("", t, 15.0, pal.text, true)
            } else if let Some(t) = trimmed.strip_prefix("## ") {
                ("", t, 18.0, pal.text, true)
            } else if let Some(t) = trimmed.strip_prefix("# ") {
                ("", t, 23.0, pal.text, true)
            } else if let Some(t) = trimmed.strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* ")) {
                ("• ", t, 13.0, pal.text, false)
            } else if let Some(t) = trimmed.strip_prefix("> ") {
                ("▏ ", t, 13.0, pal.text_muted, false)
            } else {
                ("", line, 13.0, pal.text, false)
            };
            let content = format!("{}{}", prefix, text.replace("**", "").replace("`", ""));
            let max_chars = ((bounds.width() - 32.0) / (size * 0.55))
                .floor().max(1.0) as usize;
            let rows = wrap(&content, max_chars);
            for row in rows {
                if y + size >= bounds.min.y && y < bounds.max.y {
                    if code {
                        buf.push(DrawCommand::Rect {
                            rect: Rect::from_min_size(
                                Vec2::new(bounds.min.x + 8.0, y - 3.0),
                                Vec2::new((bounds.width() - 16.0).max(0.0), size + 7.0)),
                            fill: pal.elevated, radius: CornerRadius::ZERO,
                            stroke: Stroke::NONE,
                        });
                    }
                    draw_text(buf, row, Vec2::new(bounds.min.x + 14.0, y),
                              size, foreground, strong);
                }
                y += size + 6.0;
            }
            if y > bounds.max.y + self.scroll_y.max(0.0) + 100.0 { break; }
        }
        buf.push(DrawCommand::ClipPop);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole { System, User, Assistant, Tool }

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatTranscript {
    pub messages: Vec<ChatMessage>,
    pub scroll_y: f32,
}

impl ChatTranscript {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, id: impl Into<String>, role: ChatRole,
                content: impl Into<String>) -> Result<(), String> {
        let id = id.into();
        if id.is_empty() || self.messages.iter().any(|m| m.id == id) {
            return Err("duplicate or empty message ID".into());
        }
        self.messages.push(ChatMessage {
            id, role, content: content.into(), streaming: false,
        });
        Ok(())
    }

    pub fn start_stream(&mut self, id: impl Into<String>, role: ChatRole) -> Result<(), String> {
        let id = id.into();
        self.push(id.clone(), role, "")?;
        self.messages.last_mut().unwrap().streaming = true;
        Ok(())
    }

    pub fn append_chunk(&mut self, id: &str, chunk: &str) -> Result<(), String> {
        let message = self.messages.iter_mut().find(|m| m.id == id)
            .ok_or("unknown conversation message")?;
        if !message.streaming { return Err("message is not streaming".into()); }
        message.content.push_str(chunk);
        Ok(())
    }

    pub fn finish_stream(&mut self, id: &str) -> Result<(), String> {
        let message = self.messages.iter_mut().find(|m| m.id == id)
            .ok_or("unknown conversation message")?;
        message.streaming = false;
        Ok(())
    }

    pub fn render(&self, buf: &mut CommandBuffer, bounds: Rect, pal: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: bounds });
        let mut y = bounds.min.y + 12.0 - self.scroll_y.max(0.0);
        let width = (bounds.width() - 48.0).max(60.0);
        let chars_per_line = (width / 7.5).floor().max(1.0) as usize;
        for msg in &self.messages {
            let wrapped = wrap(&msg.content, chars_per_line);
            let height = 35.0 + 20.0 * wrapped.len() as f32;
            let card = Rect::from_min_size(Vec2::new(bounds.min.x + 18.0, y),
                Vec2::new(width + 12.0, height));
            if card.intersects(bounds) {
                let (name, color) = match msg.role {
                    ChatRole::User => ("You", pal.accent),
                    ChatRole::Assistant => ("Assistant", pal.success),
                    ChatRole::System => ("System", pal.warning),
                    ChatRole::Tool => ("Tool", pal.text_muted),
                };
                buf.push(DrawCommand::Rect {
                    rect: card, fill: if msg.role == ChatRole::User {
                        pal.elevated_high
                    } else { pal.surface },
                    radius: CornerRadius::same(6.0),
                    stroke: Stroke::new(1.0, pal.border),
                });
                draw_text(buf, format!("{}{}", name,
                    if msg.streaming { "  •" } else { "" }),
                    card.min + Vec2::new(11.0, 8.0), 12.0, color, true);
                for (i, row) in wrapped.into_iter().enumerate() {
                    let yy = card.min.y + 31.0 + i as f32 * 20.0;
                    if yy >= bounds.min.y && yy < bounds.max.y {
                        draw_text(buf, row, Vec2::new(card.min.x + 11.0, yy),
                                  13.0, pal.text, false);
                    }
                }
            }
            y += height + 10.0;
            if y > bounds.max.y + 16.0 { break; }
        }
        buf.push(DrawCommand::ClipPop);
    }
}

fn draw_text(buf: &mut CommandBuffer, text: impl Into<String>,
             pos: Vec2, size: f32, color: Color, strong: bool) {
    buf.push(DrawCommand::Text { text: text.into(), pos, size, color, strong });
}

/// Safe Unicode scalar wrapping with a fallback for unspaced scripts.
fn wrap(input: &str, limit: usize) -> Vec<String> {
    let max = limit.max(1);
    if input.is_empty() { return vec![String::new()]; }
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in input.split_whitespace() {
        let count = word.chars().count();
        if !current.is_empty() && current.chars().count() + 1 + count > max {
            rows.push(std::mem::take(&mut current));
        }
        if count > max {
            if !current.is_empty() { rows.push(std::mem::take(&mut current)); }
            let mut piece = String::new();
            for ch in word.chars() {
                piece.push(ch);
                if piece.chars().count() == max {
                    rows.push(std::mem::take(&mut piece));
                }
            }
            current = piece;
        } else {
            if !current.is_empty() { current.push(' '); }
            current.push_str(word);
        }
    }
    if !current.is_empty() { rows.push(current); }
    if rows.is_empty() { rows.push(String::new()); }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_and_stable_message_ids() {
        let mut log = ChatTranscript::new();
        log.push("u1", ChatRole::User, "হ্যালো বাংলা").unwrap();
        log.start_stream("a1", ChatRole::Assistant).unwrap();
        log.append_chunk("a1", "Hello ").unwrap();
        log.append_chunk("a1", "there").unwrap();
        log.finish_stream("a1").unwrap();
        assert_eq!(log.messages[1].content, "Hello there");
        assert!(log.push("u1", ChatRole::User, "duplicate").is_err());
        assert!(wrap("বাংলা বাংলা বাংলা", 5).iter().all(|s| s.chars().count() <= 5));
    }
}
