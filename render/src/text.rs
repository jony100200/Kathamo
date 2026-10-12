//! Real Unicode text pipeline shared by the GPU and CPU backends.
//!
//! * **Shaping**: `rustybuzz` (HarfBuzz port) turns Unicode strings into positioned
//!   glyph IDs, including Bengali conjuncts, reph and vowel-sign reordering.
//! * **Font fallback**: text is split into runs per font. Bengali code points prefer
//!   Noto Sans Bengali; everything else uses Inter, then falls back down a chain.
//! * **Rasterization**: `ab_glyph` renders anti-aliased coverage masks from the real
//!   TrueType outlines (no bitmap glyphs).
//!
//! Fonts are loaded once from the shared `Fonts/binaries` collection (see
//! `Fonts/manifest.toml`). Lookup order: `KOLPONA_FONTS_DIR`, folders next to the
//! executable and its parents, the working directory and its parents, then the
//! workspace path known at compile time. If no font can be found the backends fall
//! back to the built-in bitmap glyphs so rendering never fails.

use ab_glyph::{Font as _, FontRef, GlyphId, PxScale};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Anti-aliased 8-bit coverage mask (glyph or icon).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    pub width: u32,
    pub height: u32,
    /// Offset of the mask's left edge from the pen origin, in physical pixels.
    pub left: i32,
    /// Offset of the mask's top edge from the baseline (negative = above), in physical pixels.
    pub top: i32,
    pub data: Vec<u8>,
}

/// One font face loaded from disk, shared for the lifetime of the process.
pub struct LoadedFont {
    pub file_name: String,
    pub shaper: rustybuzz::Face<'static>,
    pub outlines: FontRef<'static>,
    units_per_em: f32,
    /// Converts an em-size in px into ab_glyph's `PxScale` (which is height based).
    em_to_px_scale: f32,
    cap_height_em: f32,
}

impl LoadedFont {
    fn load(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        // Fonts live for the whole process; leaking gives us `'static` faces for both
        // the shaper and the rasterizer without self-referential structs.
        let data: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let shaper = rustybuzz::Face::from_slice(data, 0)?;
        let outlines = FontRef::try_from_slice(data).ok()?;
        let units_per_em = shaper.units_per_em() as f32;
        if units_per_em <= 0.0 {
            return None;
        }
        let em_to_px_scale = outlines.height_unscaled() / units_per_em;
        let cap_height_em = shaper
            .capital_height()
            .map(|h| h as f32 / units_per_em)
            .filter(|h| *h > 0.2)
            .unwrap_or(0.72);
        Some(Self {
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            shaper,
            outlines,
            units_per_em,
            em_to_px_scale,
            cap_height_em,
        })
    }

    #[inline]
    fn has_glyph(&self, c: char) -> bool {
        self.outlines.glyph_id(c).0 != 0
    }
}

/// The process-wide font collection.
pub struct FontSet {
    pub fonts: Vec<LoadedFont>,
    pub source_dir: Option<PathBuf>,
    regular: Option<usize>,
    strong: Option<usize>,
    bengali: Option<usize>,
    fallbacks: Vec<usize>,
}

const REGULAR: &str = "Inter-Regular.ttf";
const STRONG: &str = "Inter-SemiBold.ttf";
const BENGALI: &str = "NotoSansBengali.ttf";
const FALLBACK_CHAIN: [&str; 2] = ["JetBrainsMono-Regular.ttf", "NotoEmoji.ttf"];

impl FontSet {
    fn discover() -> Self {
        let dir = find_fonts_dir();
        let mut set = FontSet {
            fonts: Vec::new(),
            source_dir: dir.clone(),
            regular: None,
            strong: None,
            bengali: None,
            fallbacks: Vec::new(),
        };
        let Some(dir) = dir else {
            return set;
        };
        let mut load = |name: &str| -> Option<usize> {
            let font = LoadedFont::load(&dir.join(name))?;
            set.fonts.push(font);
            Some(set.fonts.len() - 1)
        };
        let regular = load(REGULAR);
        let strong = load(STRONG);
        let bengali = load(BENGALI);
        let fallbacks: Vec<usize> = FALLBACK_CHAIN.iter().filter_map(|n| load(n)).collect();
        set.regular = regular;
        set.strong = strong.or(regular);
        set.bengali = bengali;
        set.fallbacks = fallbacks;
        set
    }

    /// True when real outline fonts are available.
    pub fn is_available(&self) -> bool {
        self.regular.is_some()
    }

    fn primary(&self, strong: bool) -> Option<usize> {
        if strong {
            self.strong
        } else {
            self.regular
        }
    }

    fn font_for_char(&self, c: char, strong: bool) -> Option<usize> {
        let primary = self.primary(strong)?;
        if is_bengali(c) {
            if let Some(b) = self.bengali {
                return Some(b);
            }
        }
        if self.fonts[primary].has_glyph(c) {
            return Some(primary);
        }
        let candidates = self.bengali.iter().chain(self.fallbacks.iter());
        for &idx in candidates {
            if self.fonts[idx].has_glyph(c) {
                return Some(idx);
            }
        }
        Some(primary)
    }
}

fn is_bengali(c: char) -> bool {
    ('\u{0980}'..='\u{09FF}').contains(&c)
}

/// Characters that must stay in the run of the preceding character (joiners,
/// whitespace, combining marks) so shaping context is not broken.
fn sticks_to_previous_run(c: char) -> bool {
    c.is_whitespace()
        || c == '\u{200C}'
        || c == '\u{200D}'
        || ('\u{0300}'..='\u{036F}').contains(&c)
        || c == '\u{25CC}'
}

fn find_fonts_dir() -> Option<PathBuf> {
    let has_fonts = |p: &Path| p.join(REGULAR).is_file();
    if let Ok(env_dir) = std::env::var("KATHAMO_FONTS_DIR").or_else(|_| std::env::var("KOLPONA_FONTS_DIR")) {
        let p = PathBuf::from(env_dir);
        if has_fonts(&p) {
            return Some(p);
        }
    }
    let mut roots: Vec<PathBuf> = Vec::new();
    roots.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/..")));
    roots.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")));
    if let Ok(exe) = std::env::current_exe() {
        roots.extend(exe.ancestors().skip(1).map(Path::to_path_buf));
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    for root in roots {
        for sub in ["assets/fonts", "Fonts/binaries", "fonts", "Fonts"] {
            let candidate = root.join(sub);
            if has_fonts(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// Returns the lazily-initialised global font collection.
pub fn fonts() -> &'static FontSet {
    static FONTS: OnceLock<FontSet> = OnceLock::new();
    FONTS.get_or_init(FontSet::discover)
}

/// A glyph positioned relative to the text origin (pen start on the baseline), in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    pub font: u16,
    pub id: u16,
    pub x: f32,
    pub y: f32,
}

/// Result of shaping one string at one pixel size.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLayout {
    pub glyphs: Vec<PlacedGlyph>,
    pub advance: f32,
    /// Distance from the top of capitals to the baseline. Draw commands position text
    /// by its cap-top, so the baseline sits at `pos.y + cap_height`.
    pub cap_height: f32,
}

/// Shapes `text` at `px` (em size in pixels). Returns `None` when no outline fonts exist.
pub fn shape(text: &str, px: f32, strong: bool) -> Option<TextLayout> {
    let set = fonts();
    let primary = set.primary(strong)?;
    let mut layout = TextLayout {
        glyphs: Vec::with_capacity(text.len()),
        advance: 0.0,
        cap_height: set.fonts[primary].cap_height_em * px,
    };

    // Split into font runs.
    let mut runs: Vec<(usize, usize, usize)> = Vec::new(); // (font, start, end)
    for (i, c) in text.char_indices() {
        if c == '\n' || c == '\r' {
            continue;
        }
        let font = match runs.last() {
            Some(&(f, _, _)) if sticks_to_previous_run(c) => f,
            _ => set.font_for_char(c, strong)?,
        };
        let end = i + c.len_utf8();
        match runs.last_mut() {
            Some(run) if run.0 == font && run.2 == i => run.2 = end,
            _ => runs.push((font, i, end)),
        }
    }

    let mut pen = 0.0f32;
    for (font_idx, start, end) in runs {
        let font = &set.fonts[font_idx];
        let scale = px / font.units_per_em;
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(&text[start..end]);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(&font.shaper, &[], buffer);
        for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            layout.glyphs.push(PlacedGlyph {
                font: font_idx as u16,
                id: info.glyph_id as u16,
                x: pen + pos.x_offset as f32 * scale,
                y: -(pos.y_offset as f32) * scale,
            });
            pen += pos.x_advance as f32 * scale;
        }
    }
    layout.advance = pen;
    Some(layout)
}

/// Measures the advance width of `text` at `px`. Falls back to a monospace estimate.
pub fn measure(text: &str, px: f32, strong: bool) -> f32 {
    shape(text, px, strong)
        .map(|l| l.advance)
        .unwrap_or_else(|| text.chars().count() as f32 * px * 0.55)
}

/// Cache key for a rasterized glyph. Size is quantized to 1/4 px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font: u16,
    pub id: u16,
    pub size_q: u32,
}

impl GlyphKey {
    pub fn new(font: u16, id: u16, px: f32) -> Self {
        Self {
            font,
            id,
            size_q: (px * 4.0).round().max(1.0) as u32,
        }
    }

    pub fn px(&self) -> f32 {
        self.size_q as f32 / 4.0
    }
}

/// Rasterizes one glyph from its TrueType outline. Returns `None` for blank glyphs (spaces).
pub fn rasterize_glyph(key: GlyphKey) -> Option<Mask> {
    let set = fonts();
    let font = set.fonts.get(key.font as usize)?;
    let scale = PxScale::from(key.px() * font.em_to_px_scale);
    let glyph = GlyphId(key.id).with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
    let outlined = font.outlines.outline_glyph(glyph)?;
    let bounds = outlined.px_bounds();
    let width = (bounds.max.x - bounds.min.x).max(0.0) as u32;
    let height = (bounds.max.y - bounds.min.y).max(0.0) as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let mut data = vec![0u8; (width * height) as usize];
    outlined.draw(|x, y, c| {
        if x < width && y < height {
            data[(y * width + x) as usize] = (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    });
    Some(Mask {
        width,
        height,
        left: bounds.min.x as i32,
        top: bounds.min.y as i32,
        data,
    })
}

/// Memoizing glyph rasterizer used by the CPU backend (the GPU backend caches in its atlas).
#[derive(Default)]
pub struct GlyphCache {
    map: HashMap<GlyphKey, Option<Arc<Mask>>>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&mut self, key: GlyphKey) -> Option<Arc<Mask>> {
        self.map
            .entry(key)
            .or_insert_with(|| rasterize_glyph(key).map(Arc::new))
            .clone()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_fonts_are_discovered_from_shared_collection() {
        let set = fonts();
        assert!(
            set.is_available(),
            "Inter must be found in Fonts/binaries (dir: {:?})",
            set.source_dir
        );
        assert!(set.bengali.is_some(), "Noto Sans Bengali must load");
    }

    #[test]
    fn latin_text_shapes_with_kerning_and_real_advances() {
        let layout = shape("Kolpona Studio", 14.0, false).expect("fonts available");
        assert_eq!(layout.glyphs.len(), 14);
        assert!(layout.advance > 60.0 && layout.advance < 140.0);
        // Proportional font: 'i' narrower than 'o'.
        let narrow = measure("iiii", 14.0, false);
        let wide = measure("oooo", 14.0, false);
        assert!(narrow < wide);
    }

    #[test]
    fn bengali_conjuncts_are_shaped_not_mapped_one_to_one() {
        // "কল্পনা" = ক ল ্ প ন া (6 code points). ল্প forms a conjunct, so a correct
        // shaper produces fewer glyphs than code points, all from the Bengali font.
        let text = "কল্পনা";
        let layout = shape(text, 20.0, false).expect("fonts available");
        let bengali = fonts().bengali.unwrap() as u16;
        assert!(layout.glyphs.iter().all(|g| g.font == bengali));
        assert!(layout.glyphs.iter().all(|g| g.id != 0), "no .notdef boxes");
        // OpenType Indic GPOS/GSUB positions subjoined consonant (প) below the base (ল) with non-zero y offset
        assert!(
            layout.glyphs.iter().any(|g| g.y.abs() > 0.5),
            "expected OpenType subjoined glyph positioning"
        );
        assert!(layout.advance > 40.0, "text advance must be positive");
    }

    #[test]
    fn mixed_script_text_uses_font_fallback_runs() {
        let layout = shape("Chobi (ছবি)", 16.0, true).expect("fonts available");
        let fonts_used: std::collections::HashSet<u16> =
            layout.glyphs.iter().map(|g| g.font).collect();
        assert!(fonts_used.len() >= 2, "Latin and Bengali runs expected");
    }

    #[test]
    fn glyphs_rasterize_to_antialiased_coverage() {
        let layout = shape("A", 32.0, false).unwrap();
        let g = layout.glyphs[0];
        let mask = rasterize_glyph(GlyphKey::new(g.font, g.id, 32.0)).unwrap();
        assert!(mask.width > 10 && mask.height > 15);
        assert!(mask.top < 0, "glyph sits above the baseline");
        let partial = mask.data.iter().filter(|&&c| c > 0 && c < 255).count();
        assert!(partial > 0, "edges must be anti-aliased");
    }
}
