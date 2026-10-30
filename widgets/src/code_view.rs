//! Monospace code lines with token colouring (Rust, C#, C-like languages).
//!
//! A small deterministic lexer, no grammar files: comments, strings, numbers,
//! keywords, type names and attributes. Colours derive from the theme palette,
//! so code follows dark and light themes automatically.

use ui_core::{Color, Vec2};
use ui_render::{CommandBuffer, DrawCommand, Face};
use ui_themes::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Plain,
    Keyword,
    Type,
    String,
    Number,
    Comment,
    Attribute,
}

const KEYWORDS: &[&str] = &[
    "as",
    "async",
    "await",
    "bool",
    "break",
    "case",
    "class",
    "const",
    "continue",
    "crate",
    "else",
    "enum",
    "false",
    "fn",
    "for",
    "foreach",
    "if",
    "impl",
    "in",
    "int",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "namespace",
    "new",
    "null",
    "override",
    "private",
    "pub",
    "public",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "string",
    "struct",
    "super",
    "switch",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "using",
    "var",
    "void",
    "where",
    "while",
];

/// Splits one line into coloured tokens. Byte ranges always fall on char boundaries.
pub fn tokenize(line: &str) -> Vec<(&str, TokenKind)> {
    let mut out: Vec<(&str, TokenKind)> = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < line.len() {
        let rest = &line[i..];
        let c = rest.chars().next().unwrap_or(' ');
        let (len, kind) = if rest.starts_with("//") {
            (rest.len(), TokenKind::Comment)
        } else if rest.starts_with("#[") || rest.starts_with("#![") {
            (
                rest.find(']').map_or(rest.len(), |e| e + 1),
                TokenKind::Attribute,
            )
        } else if c == '"' {
            let mut end = 1;
            while end < rest.len() {
                match bytes[i + end] {
                    b'\\' => end += 2,
                    b'"' => {
                        end += 1;
                        break;
                    }
                    _ => end += 1,
                }
            }
            (end.min(rest.len()), TokenKind::String)
        } else if c.is_ascii_digit() {
            let n = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '_'))
                .unwrap_or(rest.len());
            (n, TokenKind::Number)
        } else if c.is_alphabetic() || c == '_' {
            let n = rest
                .find(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
                .unwrap_or(rest.len());
            let word = &rest[..n];
            let kind = if KEYWORDS.contains(&word) {
                TokenKind::Keyword
            } else if word.starts_with(|ch: char| ch.is_uppercase()) {
                TokenKind::Type
            } else {
                TokenKind::Plain
            };
            (n, kind)
        } else {
            (c.len_utf8(), TokenKind::Plain)
        };
        // Merge neighbouring plain tokens so a line is a handful of draw commands.
        match out.last_mut() {
            Some((prev, TokenKind::Plain)) if kind == TokenKind::Plain => {
                let start = i - prev.len();
                *prev = &line[start..i + len];
            }
            _ => out.push((&line[i..i + len], kind)),
        }
        i += len;
    }
    out
}

/// Theme-derived colour for a token kind.
pub fn token_color(kind: TokenKind, palette: &ThemePalette) -> Color {
    match kind {
        TokenKind::Plain => palette.text,
        TokenKind::Keyword => palette.accent,
        TokenKind::Type => palette.success.lerp(palette.text, 0.25),
        TokenKind::String => palette.warning.lerp(palette.text, 0.35),
        TokenKind::Number => palette.danger.lerp(palette.text, 0.3),
        TokenKind::Comment => palette.text_muted.with_alpha(170),
        TokenKind::Attribute => palette.text_muted,
    }
}

/// Draws one highlighted line in the monospace face starting at `pos`.
pub fn render_code_line(
    buf: &mut CommandBuffer,
    line: &str,
    pos: Vec2,
    size: f32,
    palette: &ThemePalette,
) {
    let mut x = pos.x;
    for (text, kind) in tokenize(line) {
        if !text.trim().is_empty() {
            buf.push(DrawCommand::Code {
                text: text.to_string(),
                pos: Vec2::new(x, pos.y),
                size,
                color: token_color(kind, palette),
            });
        }
        x += ui_render::measure_face(text, size, Face::Mono);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_line_splits_into_expected_kinds() {
        let toks = tokenize("pub fn hit(&self, ray: &Ray) -> f32 { 1.0 } // hot");
        let kinds: Vec<_> = toks.iter().map(|t| t.1).collect();
        assert_eq!(toks[0], ("pub", TokenKind::Keyword));
        assert!(toks.contains(&("Ray", TokenKind::Type)));
        assert!(toks.contains(&("1.0", TokenKind::Number)));
        assert_eq!(*toks.last().unwrap(), ("// hot", TokenKind::Comment));
        assert!(kinds.contains(&TokenKind::Plain));
        let joined: String = toks.iter().map(|t| t.0).collect();
        assert_eq!(joined, "pub fn hit(&self, ray: &Ray) -> f32 { 1.0 } // hot");
    }

    #[test]
    fn strings_attributes_and_unicode_are_safe() {
        let toks = tokenize("#[derive(Clone)] let s = \"a \\\" b\"; // বাংলা");
        assert_eq!(toks[0], ("#[derive(Clone)]", TokenKind::Attribute));
        assert!(toks.contains(&("\"a \\\" b\"", TokenKind::String)));
        assert_eq!(toks.last().unwrap().1, TokenKind::Comment);
        assert!(tokenize("let x = \"unterminated").last().unwrap().1 == TokenKind::String);
    }
}
