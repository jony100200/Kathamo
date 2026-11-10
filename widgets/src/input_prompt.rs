//! Input prompt: a small modal that asks for one typed value (text or a number) or one choice.
//!
//! The host opens it with [`InputPrompt::open`], forwards keys, text and clicks, and receives a
//! [`PromptOutcome`] when the user submits or cancels. Keys and clicks are only meaningful while
//! [`InputPrompt::is_open`] is true.

use crate::theme::ThemePalette;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_platform::{Key, PointerButton};
use ui_render::{CommandBuffer, DrawCommand};

const DIALOG_W: f32 = 420.0;
const FIELD_H: f32 = 34.0;
const CHOICE_H: f32 = 32.0;
const BUTTON_W: f32 = 88.0;
const BUTTON_H: f32 = 30.0;
/// Title, message and the gap above the input.
const HEAD_H: f32 = 72.0;
/// Error line plus the gaps around it, and the button row with its bottom padding.
const FOOT_H: f32 = 94.0;

/// What the prompt asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptKind {
    /// Free text. With `required`, an empty or blank answer is refused.
    Text { required: bool },
    /// A number, optionally bounded (both ends inclusive).
    Number { min: Option<f64>, max: Option<f64> },
    /// One of these options, picked with the arrows or a click.
    Choice(Vec<String>),
}

/// A value the user submitted.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptValue {
    Text(String),
    Number(f64),
    Choice { index: usize, label: String },
}

/// How the prompt ended. The prompt is closed when this is returned.
#[derive(Debug, Clone, PartialEq)]
pub enum PromptOutcome {
    Submitted(PromptValue),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputPrompt {
    pub is_open: bool,
    pub title: String,
    pub message: String,
    pub kind: PromptKind,
    /// Entry buffer for text and number prompts.
    pub text: String,
    /// Selected option for choice prompts.
    pub choice: usize,
    /// Why the last submit was refused, shown under the input.
    pub error: Option<String>,
    pub confirm_label: String,
    pub cancel_label: String,
}

impl Default for InputPrompt {
    fn default() -> Self {
        Self {
            is_open: false,
            title: String::new(),
            message: String::new(),
            kind: PromptKind::Text { required: false },
            text: String::new(),
            choice: 0,
            error: None,
            confirm_label: "OK".into(),
            cancel_label: "Cancel".into(),
        }
    }
}

/// Rectangles of one laid-out prompt. Rendering and hit-testing both use it.
struct PromptLayout {
    dialog: Rect,
    /// The text or number field, for text and number prompts.
    field: Rect,
    /// One row per option, for choice prompts.
    rows: Vec<Rect>,
    confirm: Rect,
    cancel: Rect,
}

impl InputPrompt {
    /// Opens the prompt for `kind`, with an empty entry and the first option selected.
    pub fn open(&mut self, title: impl Into<String>, message: impl Into<String>, kind: PromptKind) {
        self.is_open = true;
        self.title = title.into();
        self.message = message.into();
        self.kind = kind;
        self.text.clear();
        self.choice = 0;
        self.error = None;
    }

    /// Closes the prompt without an outcome.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    fn body_height(&self) -> f32 {
        match &self.kind {
            PromptKind::Choice(options) => options.len() as f32 * CHOICE_H,
            _ => FIELD_H,
        }
    }

    fn layout(&self, screen_size: Vec2) -> PromptLayout {
        let height = HEAD_H + self.body_height() + FOOT_H;
        let dialog = Rect::from_min_size(
            Vec2::new((screen_size.x - DIALOG_W) * 0.5, screen_size.y * 0.3),
            Vec2::new(DIALOG_W, height),
        );
        let body_top = dialog.min.y + HEAD_H;
        let field = Rect::from_min_size(
            Vec2::new(dialog.min.x + 16.0, body_top),
            Vec2::new(DIALOG_W - 32.0, FIELD_H),
        );
        let rows = match &self.kind {
            PromptKind::Choice(options) => (0..options.len())
                .map(|i| {
                    Rect::from_min_size(
                        Vec2::new(dialog.min.x + 16.0, body_top + i as f32 * CHOICE_H),
                        Vec2::new(DIALOG_W - 32.0, CHOICE_H - 2.0),
                    )
                })
                .collect(),
            _ => Vec::new(),
        };
        let button_y = dialog.max.y - 16.0 - BUTTON_H;
        let confirm = Rect::from_min_size(
            Vec2::new(dialog.max.x - 16.0 - BUTTON_W, button_y),
            Vec2::new(BUTTON_W, BUTTON_H),
        );
        let cancel = Rect::from_min_size(
            Vec2::new(confirm.min.x - 8.0 - BUTTON_W, button_y),
            Vec2::new(BUTTON_W, BUTTON_H),
        );
        PromptLayout {
            dialog,
            field,
            rows,
            confirm,
            cancel,
        }
    }

    /// Validates the entry and returns the value, or sets `error` and returns `None`.
    fn submit(&mut self) -> Option<PromptValue> {
        let value = match &self.kind {
            PromptKind::Text { required } => {
                if *required && self.text.trim().is_empty() {
                    self.error = Some("Enter a value to continue.".into());
                    return None;
                }
                PromptValue::Text(self.text.clone())
            }
            PromptKind::Number { min, max } => {
                let number = match self.text.trim().parse::<f64>() {
                    Ok(n) if n.is_finite() => n,
                    _ => {
                        self.error = Some("Enter a number.".into());
                        return None;
                    }
                };
                if min.is_some_and(|lo| number < lo) {
                    self.error = Some(format!("Must be at least {}.", min.unwrap_or_default()));
                    return None;
                }
                if max.is_some_and(|hi| number > hi) {
                    self.error = Some(format!("Must be at most {}.", max.unwrap_or_default()));
                    return None;
                }
                PromptValue::Number(number)
            }
            PromptKind::Choice(options) => {
                let label = options.get(self.choice)?.clone();
                PromptValue::Choice {
                    index: self.choice,
                    label,
                }
            }
        };
        Some(value)
    }

    fn confirm(&mut self) -> Option<PromptOutcome> {
        let value = self.submit()?;
        self.close();
        Some(PromptOutcome::Submitted(value))
    }

    fn cancel(&mut self) -> Option<PromptOutcome> {
        self.close();
        Some(PromptOutcome::Cancelled)
    }

    fn option_count(&self) -> usize {
        match &self.kind {
            PromptKind::Choice(options) => options.len(),
            _ => 0,
        }
    }

    /// Keys while open: Enter submits, Escape cancels, Backspace edits, and the arrows move a
    /// choice. Returns the outcome when the prompt ends. Returns `None` when closed.
    pub fn handle_key_down(&mut self, key: &Key) -> Option<PromptOutcome> {
        if !self.is_open {
            return None;
        }
        match key {
            Key::Enter => self.confirm(),
            Key::Escape => self.cancel(),
            Key::Backspace => {
                if !matches!(self.kind, PromptKind::Choice(_)) {
                    self.text.pop();
                    self.error = None;
                }
                None
            }
            Key::ArrowUp | Key::ArrowLeft => {
                self.choice = self.choice.saturating_sub(1);
                None
            }
            Key::ArrowDown | Key::ArrowRight => {
                let last = self.option_count().saturating_sub(1);
                self.choice = (self.choice + 1).min(last);
                None
            }
            _ => None,
        }
    }

    /// Typed characters go to the entry. Number entries accept only characters that can
    /// appear in a number. Returns true when the prompt is open.
    pub fn handle_text_input(&mut self, text: &str) -> bool {
        if !self.is_open {
            return false;
        }
        match self.kind {
            PromptKind::Text { .. } => {
                self.text.extend(text.chars().filter(|c| !c.is_control()));
                self.error = None;
            }
            PromptKind::Number { .. } => {
                self.text
                    .extend(text.chars().filter(|c| {
                        c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')
                    }));
                self.error = None;
            }
            PromptKind::Choice(_) => {}
        }
        true
    }

    /// A click on a choice row selects it. The confirm button submits, Cancel or a click
    /// outside the dialog cancels. Returns the outcome when the prompt ends.
    pub fn handle_pointer_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        screen_size: Vec2,
    ) -> Option<PromptOutcome> {
        if !self.is_open || button != PointerButton::Primary {
            return None;
        }
        let layout = self.layout(screen_size);
        if layout.confirm.contains(pos) {
            return self.confirm();
        }
        if layout.cancel.contains(pos) {
            return self.cancel();
        }
        if let Some(index) = layout.rows.iter().position(|r| r.contains(pos)) {
            self.choice = index;
            self.error = None;
            return None;
        }
        if !layout.dialog.contains(pos) {
            return self.cancel();
        }
        None
    }

    pub fn render(&self, buf: &mut CommandBuffer, screen_size: Vec2, palette: &ThemePalette) {
        if !self.is_open {
            return;
        }
        let layout = self.layout(screen_size);
        let dialog = layout.dialog;

        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::ZERO, screen_size),
            fill: Color::BLACK.with_alpha(120),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::SoftShadow {
            rect: dialog,
            radius: CornerRadius::same(10.0),
            color: Color::rgba(0, 0, 0, 140),
            offset: Vec2::new(0.0, 12.0),
            blur: 32.0,
            spread: 0.0,
        });
        buf.push(DrawCommand::Backdrop {
            rect: dialog,
            radius: CornerRadius::same(10.0),
            tint: palette.surface.with_alpha(215),
        });
        buf.push(DrawCommand::Rect {
            rect: dialog,
            fill: Color::TRANSPARENT,
            radius: CornerRadius::same(10.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        buf.push(DrawCommand::Text {
            text: self.title.clone(),
            pos: Vec2::new(dialog.min.x + 16.0, dialog.min.y + 14.0),
            size: 13.5,
            color: palette.text,
            strong: true,
        });
        buf.push(DrawCommand::Text {
            text: self.message.clone(),
            pos: Vec2::new(dialog.min.x + 16.0, dialog.min.y + 40.0),
            size: 11.5,
            color: palette.text_muted,
            strong: false,
        });

        match &self.kind {
            PromptKind::Choice(options) => {
                for (i, (option, row)) in options.iter().zip(&layout.rows).enumerate() {
                    let selected = i == self.choice;
                    buf.push(DrawCommand::Rect {
                        rect: *row,
                        fill: if selected {
                            palette.accent.with_alpha(45)
                        } else {
                            palette.elevated
                        },
                        radius: CornerRadius::same(6.0),
                        stroke: if selected {
                            Stroke::new(1.0, palette.accent)
                        } else {
                            Stroke::NONE
                        },
                    });
                    buf.push(DrawCommand::Text {
                        text: option.clone(),
                        pos: Vec2::new(row.min.x + 10.0, row.min.y + 8.0),
                        size: 12.0,
                        color: palette.text,
                        strong: selected,
                    });
                }
            }
            _ => {
                let field = layout.field;
                buf.push(DrawCommand::Rect {
                    rect: field,
                    fill: palette.elevated,
                    radius: CornerRadius::same(6.0),
                    stroke: Stroke::new(1.0, palette.accent),
                });
                buf.push(DrawCommand::Text {
                    text: self.text.clone(),
                    pos: Vec2::new(field.min.x + 12.0, field.min.y + 10.0),
                    size: 13.0,
                    color: palette.text,
                    strong: false,
                });
            }
        }

        if let Some(error) = &self.error {
            let error_y = layout
                .field
                .max
                .y
                .max(layout.rows.last().map_or(layout.field.max.y, |r| r.max.y))
                + 8.0;
            buf.push(DrawCommand::Text {
                text: error.clone(),
                pos: Vec2::new(dialog.min.x + 16.0, error_y),
                size: 11.5,
                color: palette.danger,
                strong: false,
            });
        }

        buf.push(DrawCommand::Rect {
            rect: layout.cancel,
            fill: palette.elevated,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        buf.push(DrawCommand::Text {
            text: self.cancel_label.clone(),
            pos: Vec2::new(layout.cancel.min.x + 14.0, layout.cancel.min.y + 8.0),
            size: 12.0,
            color: palette.text,
            strong: false,
        });
        buf.push(DrawCommand::Rect {
            rect: layout.confirm,
            fill: palette.accent,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::Text {
            text: self.confirm_label.clone(),
            pos: Vec2::new(layout.confirm.min.x + 14.0, layout.confirm.min.y + 8.0),
            size: 12.0,
            color: palette.canvas,
            strong: true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Vec2 = Vec2::new(1280.0, 800.0);

    fn open(kind: PromptKind) -> InputPrompt {
        let mut p = InputPrompt::default();
        p.open("Rename", "Enter a name", kind);
        p
    }

    fn centre_of(rect: Rect) -> Vec2 {
        rect.center()
    }

    #[test]
    fn text_prompt_submits_what_was_typed() {
        let mut p = open(PromptKind::Text { required: false });
        assert!(p.handle_text_input("Scene"));
        p.handle_text_input("\n2");
        assert_eq!(p.text, "Scene2", "control characters are dropped");
        p.handle_key_down(&Key::Backspace);
        assert_eq!(
            p.handle_key_down(&Key::Enter),
            Some(PromptOutcome::Submitted(PromptValue::Text("Scene".into())))
        );
        assert!(!p.is_open);
    }

    #[test]
    fn required_text_refuses_blank_and_keeps_the_prompt_open() {
        let mut p = open(PromptKind::Text { required: true });
        p.handle_text_input("   ");
        assert_eq!(p.handle_key_down(&Key::Enter), None);
        assert!(p.is_open);
        assert!(p.error.is_some());
        p.handle_text_input("ok");
        assert!(p.error.is_none(), "typing clears the error");
        assert!(matches!(
            p.handle_key_down(&Key::Enter),
            Some(PromptOutcome::Submitted(PromptValue::Text(_)))
        ));
    }

    #[test]
    fn number_prompt_checks_the_range() {
        let mut p = open(PromptKind::Number {
            min: Some(1.0),
            max: Some(64.0),
        });
        p.handle_text_input("abc");
        assert_eq!(p.text, "", "letters are not accepted into a number");
        p.handle_text_input("0");
        assert_eq!(p.handle_key_down(&Key::Enter), None);
        assert_eq!(p.error.as_deref(), Some("Must be at least 1."));
        p.handle_key_down(&Key::Backspace);
        p.handle_text_input("128");
        assert_eq!(p.handle_key_down(&Key::Enter), None);
        assert_eq!(p.error.as_deref(), Some("Must be at most 64."));
        for _ in 0..3 {
            p.handle_key_down(&Key::Backspace);
        }
        p.handle_text_input("2.5");
        assert_eq!(
            p.handle_key_down(&Key::Enter),
            Some(PromptOutcome::Submitted(PromptValue::Number(2.5)))
        );
    }

    #[test]
    fn choice_prompt_moves_with_arrows_and_submits_the_index() {
        let mut p = open(PromptKind::Choice(vec![
            "PNG".into(),
            "JPEG".into(),
            "WEBP".into(),
        ]));
        p.handle_key_down(&Key::ArrowDown);
        p.handle_key_down(&Key::ArrowDown);
        p.handle_key_down(&Key::ArrowDown);
        assert_eq!(p.choice, 2, "clamped to the last option");
        p.handle_key_down(&Key::ArrowUp);
        assert_eq!(
            p.handle_key_down(&Key::Enter),
            Some(PromptOutcome::Submitted(PromptValue::Choice {
                index: 1,
                label: "JPEG".into(),
            }))
        );
    }

    #[test]
    fn escape_cancels() {
        let mut p = open(PromptKind::Text { required: false });
        p.handle_text_input("half typed");
        assert_eq!(
            p.handle_key_down(&Key::Escape),
            Some(PromptOutcome::Cancelled)
        );
        assert!(!p.is_open);
    }

    #[test]
    fn closed_prompt_ignores_input() {
        let mut p = InputPrompt::default();
        assert_eq!(p.handle_key_down(&Key::Enter), None);
        assert!(!p.handle_text_input("x"));
        assert_eq!(
            p.handle_pointer_down(Vec2::ZERO, PointerButton::Primary, SCREEN),
            None
        );
    }

    #[test]
    fn mouse_selects_confirms_cancels_and_dismisses() {
        let mut p = open(PromptKind::Choice(vec!["A".into(), "B".into()]));
        let row_b = p.layout(SCREEN).rows[1];
        assert_eq!(
            p.handle_pointer_down(centre_of(row_b), PointerButton::Primary, SCREEN),
            None
        );
        assert_eq!(p.choice, 1);

        let ok = p.layout(SCREEN).confirm;
        assert_eq!(
            p.handle_pointer_down(centre_of(ok), PointerButton::Primary, SCREEN),
            Some(PromptOutcome::Submitted(PromptValue::Choice {
                index: 1,
                label: "B".into(),
            }))
        );

        p.open("Rename", "", PromptKind::Text { required: false });
        let cancel = p.layout(SCREEN).cancel;
        assert_eq!(
            p.handle_pointer_down(centre_of(cancel), PointerButton::Primary, SCREEN),
            Some(PromptOutcome::Cancelled)
        );

        p.open("Rename", "", PromptKind::Text { required: false });
        assert_eq!(
            p.handle_pointer_down(Vec2::new(4.0, 4.0), PointerButton::Primary, SCREEN),
            Some(PromptOutcome::Cancelled),
            "a click outside the dialog cancels"
        );
    }

    #[test]
    fn render_draws_the_choices_and_buttons() {
        let p = open(PromptKind::Choice(vec!["Keep".into(), "Discard".into()]));
        let mut buf = CommandBuffer::default();
        p.render(&mut buf, SCREEN, &ThemePalette::dark());
        let texts: Vec<&str> = buf
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        for expected in ["Rename", "Keep", "Discard", "OK", "Cancel"] {
            assert!(texts.contains(&expected), "missing {expected}");
        }
    }
}
