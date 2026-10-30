//! Studio design tokens, theme palettes, typography scale, and Material 3 elevation system.

pub mod metrics;
pub mod typography;

pub use metrics::*;
pub use typography::*;

use ui_core::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    StudioGraphite,
    Light,
    /// Calm blue-black workspace with a warm copper accent.
    MidnightGraphite,
    /// Warm light paper surfaces for daylight work.
    WarmPaper,
}

impl ThemeMode {
    pub const fn is_dark(self) -> bool {
        !matches!(self, ThemeMode::Light | ThemeMode::WarmPaper)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemePalette {
    pub canvas: Color,        // Surface Level 0 (Deepest background)
    pub surface: Color,       // Surface Level 1 (Panel backgrounds)
    pub elevated: Color,      // Surface Level 2 (Cards, popovers, dropdowns)
    pub elevated_high: Color, // Surface Level 3 (Highest modals, tooltips)
    pub border: Color,        // Dividers, panel outlines
    pub text: Color,          // Primary readable text (≥7:1 contrast)
    pub text_muted: Color,    // Secondary text
    pub accent: Color,        // Primary brand/focus accent
    pub accent_hover: Color,  // Interactive hover state
    pub danger: Color,        // Destructive actions, errors
    pub success: Color,       // Verification passes, ready status
    pub warning: Color,       // Warnings, running jobs
}

impl ThemePalette {
    /// Warm charcoal workspace: neutral surfaces and copper accents, not blue-tinted panels.
    pub const fn dark() -> Self {
        Self {
            canvas: Color::rgb(16, 18, 19),
            surface: Color::rgb(25, 27, 28),
            elevated: Color::rgb(36, 39, 40),
            elevated_high: Color::rgb(49, 52, 53),
            border: Color::rgb(65, 69, 70),
            text: Color::rgb(242, 239, 232),
            text_muted: Color::rgb(168, 172, 171),
            accent: Color::rgb(221, 170, 104),
            accent_hover: Color::rgb(239, 191, 126),
            danger: Color::rgb(222, 102, 95),
            success: Color::rgb(116, 192, 153),
            warning: Color::rgb(229, 176, 94),
        }
    }

    /// Unified Studio Neutral / Graphite theme shared with Chobi and KS-MotionFlow.
    pub const fn studio_graphite() -> Self {
        Self {
            canvas: Color::rgb(0x1C, 0x1C, 0x1F),  // Deep inset well #1C1C1F
            surface: Color::rgb(0x22, 0x22, 0x26), // Warm dark graphite chrome #222226
            elevated: Color::rgb(0x2A, 0x2A, 0x2F), // Distinct toolbar surface #2A2A2F
            elevated_high: Color::rgb(0x32, 0x32, 0x38), // Rich graphite panel body #323238
            border: Color::rgb(0x3D, 0x3D, 0x46),  // 1px panel boundary #3D3D46
            text: Color::rgb(0xEA, 0xEA, 0xEE),    // Crisp readable text #EAEAEE
            text_muted: Color::rgb(0xB2, 0xB2, 0xBC), // Clean secondary labels #B2B2BC
            accent: Color::rgb(0x38, 0x75, 0xB0),  // Vibrant studio blue #3875B0
            accent_hover: Color::rgb(0x5B, 0xA0, 0xE0), // Focused outline #5BA0E0
            danger: Color::rgb(220, 65, 65),       // Actionable error
            success: Color::rgb(55, 180, 115),     // Ready/verified state
            warning: Color::rgb(235, 160, 45),     // In-progress notification
        }
    }

    pub const fn light() -> Self {
        Self {
            canvas: Color::rgb(245, 247, 252),
            surface: Color::rgb(255, 255, 255),
            elevated: Color::rgb(238, 242, 250),
            elevated_high: Color::rgb(228, 234, 245),
            border: Color::rgb(215, 222, 235),
            text: Color::rgb(20, 25, 38),
            text_muted: Color::rgb(100, 112, 133),
            accent: Color::rgb(60, 100, 240),
            accent_hover: Color::rgb(80, 120, 255),
            danger: Color::rgb(215, 60, 60),
            success: Color::rgb(40, 165, 105),
            warning: Color::rgb(210, 130, 20),
        }
    }

    /// Midnight Graphite: canvas #10141B, panel #171D27, elevated #202837, accent #E5A073.
    pub const fn midnight_graphite() -> Self {
        Self {
            canvas: Color::rgb(0x10, 0x14, 0x1B),
            surface: Color::rgb(0x17, 0x1D, 0x27),
            elevated: Color::rgb(0x20, 0x28, 0x37),
            elevated_high: Color::rgb(0x29, 0x32, 0x43),
            border: Color::rgb(0x31, 0x3D, 0x4C),
            text: Color::rgb(0xE9, 0xEE, 0xF5),
            text_muted: Color::rgb(0xA4, 0xB1, 0xC2),
            accent: Color::rgb(0xE5, 0xA0, 0x73),
            accent_hover: Color::rgb(0xF0, 0xB5, 0x8D),
            danger: Color::rgb(0xEA, 0x77, 0x7E),
            success: Color::rgb(0x68, 0xC5, 0x9E),
            warning: Color::rgb(0xE9, 0xBE, 0x6D),
        }
    }

    /// Warm Paper: cream surfaces, ink text and a deeper copper accent that stays legible.
    pub const fn warm_paper() -> Self {
        Self {
            canvas: Color::rgb(0xF4, 0xEF, 0xE6),
            surface: Color::rgb(0xFB, 0xF8, 0xF2),
            elevated: Color::rgb(0xFF, 0xFD, 0xF9),
            elevated_high: Color::rgb(0xFF, 0xFF, 0xFF),
            border: Color::rgb(0xDD, 0xD3, 0xC4),
            text: Color::rgb(0x26, 0x22, 0x1D),
            text_muted: Color::rgb(0x6B, 0x62, 0x57),
            accent: Color::rgb(0xA8, 0x5A, 0x2A),
            accent_hover: Color::rgb(0x8E, 0x4A, 0x20),
            danger: Color::rgb(0xB3, 0x3A, 0x40),
            success: Color::rgb(0x2E, 0x7D, 0x5B),
            warning: Color::rgb(0x9A, 0x6A, 0x12),
        }
    }

    /// Android Material 3 inspired surface elevation mapper (Levels 0 through 4).
    pub fn surface_at_elevation(&self, level: u8) -> Color {
        match level {
            0 => self.canvas,
            1 => self.surface,
            2 => self.elevated,
            _ => self.elevated_high,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StudioTheme {
    pub mode: ThemeMode,
    pub palette: ThemePalette,
}

impl Default for StudioTheme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            palette: ThemePalette::dark(),
        }
    }
}

impl StudioTheme {
    pub fn new(mode: ThemeMode) -> Self {
        let palette = match mode {
            ThemeMode::Dark => ThemePalette::dark(),
            ThemeMode::StudioGraphite => ThemePalette::studio_graphite(),
            ThemeMode::Light => ThemePalette::light(),
            ThemeMode::MidnightGraphite => ThemePalette::midnight_graphite(),
            ThemeMode::WarmPaper => ThemePalette::warm_paper(),
        };
        Self { mode, palette }
    }

    pub fn extended(&self) -> ExtendedTokens {
        ExtendedTokens::for_mode(self.mode)
    }
}

/// Tokens beyond the base palette. Kept separate so existing `ThemePalette` literals stay valid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtendedTokens {
    /// Keyboard focus ring.
    pub focus: Color,
    /// Selected row / text selection fill.
    pub selection: Color,
    /// Neutral informational status.
    pub info: Color,
    /// "Agent is working" status. Distinct from the accent so brand colour never means busy.
    pub working: Color,
    /// Text drawn on top of an accent fill.
    pub on_accent: Color,
    /// Modal backdrop.
    pub scrim: Color,
}

impl ExtendedTokens {
    pub const fn for_mode(mode: ThemeMode) -> Self {
        if mode.is_dark() {
            Self {
                focus: Color::rgb(0xF0, 0xB5, 0x8D),
                selection: Color::rgba(0xE5, 0xA0, 0x73, 0x2E),
                info: Color::rgb(0x8F, 0xAE, 0xE8),
                working: Color::rgb(0x6F, 0xC3, 0xD6),
                on_accent: Color::rgb(0x1A, 0x12, 0x0C),
                scrim: Color::rgba(0x05, 0x07, 0x0B, 0xB8),
            }
        } else {
            Self {
                focus: Color::rgb(0x8E, 0x4A, 0x20),
                selection: Color::rgba(0xA8, 0x5A, 0x2A, 0x26),
                info: Color::rgb(0x2F, 0x5C, 0xA8),
                working: Color::rgb(0x1F, 0x72, 0x87),
                on_accent: Color::rgb(0xFF, 0xFB, 0xF5),
                scrim: Color::rgba(0x26, 0x22, 0x1D, 0x66),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_palettes_exceed_wcag_contrast_requirements() {
        for mode in [
            ThemeMode::Dark,
            ThemeMode::StudioGraphite,
            ThemeMode::Light,
            ThemeMode::MidnightGraphite,
            ThemeMode::WarmPaper,
        ] {
            let t = StudioTheme::new(mode);
            let text_contrast = t.palette.text.contrast_ratio(t.palette.canvas);
            assert!(
                text_contrast >= 7.0,
                "Text contrast {:.2} must meet WCAG AAA (>= 7.0) for mode {:?}",
                text_contrast,
                mode
            );
        }
    }

    #[test]
    fn surface_elevations_increase_depth_monotonically() {
        for p in [
            ThemePalette::dark(),
            ThemePalette::studio_graphite(),
            ThemePalette::midnight_graphite(),
        ] {
            assert_ne!(p.surface_at_elevation(0), p.surface_at_elevation(1));
            assert_ne!(p.surface_at_elevation(1), p.surface_at_elevation(2));
        }
    }

    #[test]
    fn workspace_themes_keep_muted_text_and_status_legible() {
        for mode in [ThemeMode::MidnightGraphite, ThemeMode::WarmPaper] {
            let t = StudioTheme::new(mode);
            let p = t.palette;
            let x = t.extended();
            let checks = [
                ("text_muted/canvas", p.text_muted.contrast_ratio(p.canvas)),
                ("text_muted/surface", p.text_muted.contrast_ratio(p.surface)),
                ("accent/surface", p.accent.contrast_ratio(p.surface)),
                ("success/surface", p.success.contrast_ratio(p.surface)),
                ("danger/surface", p.danger.contrast_ratio(p.surface)),
                ("working/surface", x.working.contrast_ratio(p.surface)),
                ("on_accent/accent", x.on_accent.contrast_ratio(p.accent)),
            ];
            for (name, ratio) in checks {
                assert!(
                    ratio >= 4.5,
                    "{name} contrast {ratio:.2} < 4.5 for {mode:?}"
                );
            }
        }
    }
}
