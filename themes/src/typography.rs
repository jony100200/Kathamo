//! Studio typography tokens, Fluent 2 typography scale, font definitions, and font binary accessors.

/// Microsoft Fluent 2 inspired restrained typography scale (in logical pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluentTypographyScale {
    /// 12px - Caption / metadata, badges, keyboard shortcuts
    Caption,
    /// 14px - Normal interface text, inspectors, list items
    Body,
    /// 16px - Section labels, prominent control headers
    Section,
    /// 20px - Panel titles, modal window headers
    PanelTitle,
    /// 28px - Workspace titles, major studio hub headers
    WorkspaceTitle,
    /// 36px - Home hero titles, brand title cards (32-40px range)
    HeroTitle,
    /// 12px - Code, coordinates, values, and diagnostic readouts
    Monospace,
}

impl FluentTypographyScale {
    pub const fn pixels(self) -> f32 {
        match self {
            Self::Caption => 12.0,
            Self::Body => 14.0,
            Self::Section => 16.0,
            Self::PanelTitle => 20.0,
            Self::WorkspaceTitle => 28.0,
            Self::HeroTitle => 36.0,
            Self::Monospace => 12.0,
        }
    }

    /// User-adjustable interface scaling (e.g. 1.0 = 100%, 1.25 = 125%, 1.5 = 150%).
    pub fn scaled(self, scale_factor: f32) -> f32 {
        (self.pixels() * scale_factor).round()
    }
}

/// Backwards-compatible alias for earlier scale enum.
pub type TypographyScale = FluentTypographyScale;

/// Bilingual studio title structure for English and Bengali script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilingualLabel {
    pub english: &'static str,
    pub bengali: &'static str,
}

impl BilingualLabel {
    pub const fn new(english: &'static str, bengali: &'static str) -> Self {
        Self { english, bengali }
    }

    pub fn display_string(&self, show_both: bool) -> String {
        if show_both {
            format!("{} ({})", self.english, self.bengali)
        } else {
            self.english.to_string()
        }
    }
}

/// Canonical studio application bilingual labels.
pub const STUDIO_LABELS: &[BilingualLabel] = &[
    BilingualLabel::new("Kolpona", "কল্পনা"),
    BilingualLabel::new("Chobi", "ছবি"),
    BilingualLabel::new("Dolil", "দলিল"),
    BilingualLabel::new("Jogot", "জগত"),
    BilingualLabel::new("Rupayon", "রূপায়ণ"),
    BilingualLabel::new("Drisho", "দৃশ্য"),
    BilingualLabel::new("Pothik", "পথিক"),
    BilingualLabel::new("Upadan", "উপাদান"),
];

/// Embedded studio font binaries loaded from the shared assets directory.
pub struct StudioFontBinaries {
    pub inter_regular: &'static [u8],
    pub inter_semibold: &'static [u8],
    pub space_grotesk_bold: &'static [u8],
    pub jetbrains_mono: &'static [u8],
    pub ibm_plex_mono: &'static [u8],
    pub noto_sans_bengali: &'static [u8],
}

impl StudioFontBinaries {
    pub const fn get() -> Self {
        Self {
            inter_regular: include_bytes!("../../assets/fonts/Inter-Regular.ttf"),
            inter_semibold: include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"),
            space_grotesk_bold: include_bytes!("../../assets/fonts/SpaceGrotesk-Bold.ttf"),
            jetbrains_mono: include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
            ibm_plex_mono: include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"),
            noto_sans_bengali: include_bytes!("../../assets/fonts/NotoSansBengali.ttf"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_font_binaries_are_valid_and_non_empty() {
        let fonts = StudioFontBinaries::get();
        assert!(
            fonts.inter_regular.len() > 100_000,
            "Inter-Regular must be present"
        );
        assert!(
            fonts.inter_semibold.len() > 100_000,
            "Inter-SemiBold must be present"
        );
        assert!(
            fonts.space_grotesk_bold.len() > 50_000,
            "SpaceGrotesk-Bold must be present"
        );
        assert!(
            fonts.jetbrains_mono.len() > 50_000,
            "JetBrainsMono must be present"
        );
        assert!(
            fonts.ibm_plex_mono.len() > 50_000,
            "IBMPlexMono must be present"
        );
        assert!(
            fonts.noto_sans_bengali.len() > 100_000,
            "NotoSansBengali must be present"
        );
    }

    #[test]
    fn fluent_typography_scale_complies_with_fluent_2_guidance() {
        assert_eq!(FluentTypographyScale::Caption.pixels(), 12.0);
        assert_eq!(FluentTypographyScale::Body.pixels(), 14.0);
        assert_eq!(FluentTypographyScale::Section.pixels(), 16.0);
        assert_eq!(FluentTypographyScale::PanelTitle.pixels(), 20.0);
        assert_eq!(FluentTypographyScale::WorkspaceTitle.pixels(), 28.0);
        assert_eq!(FluentTypographyScale::HeroTitle.pixels(), 36.0);

        // Monotonic progression
        assert!(FluentTypographyScale::Caption.pixels() < FluentTypographyScale::Body.pixels());
        assert!(FluentTypographyScale::Body.pixels() < FluentTypographyScale::Section.pixels());
        assert!(
            FluentTypographyScale::Section.pixels() < FluentTypographyScale::PanelTitle.pixels()
        );
        assert!(
            FluentTypographyScale::PanelTitle.pixels()
                < FluentTypographyScale::WorkspaceTitle.pixels()
        );
        assert!(
            FluentTypographyScale::WorkspaceTitle.pixels()
                < FluentTypographyScale::HeroTitle.pixels()
        );
    }

    #[test]
    fn bilingual_labels_format_correctly() {
        let kolpona = &STUDIO_LABELS[0];
        assert_eq!(kolpona.display_string(true), "Kolpona (কল্পনা)");
        assert_eq!(kolpona.display_string(false), "Kolpona");
    }
}
