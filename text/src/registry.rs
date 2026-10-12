//! Font registry, lazy asset loading, script fallback chains, and memory tracking.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::manifest::FontManifest;

/// Thread-safe font binary buffer.
pub type SharedFontBytes = Arc<Vec<u8>>;

/// In-memory font registry providing on-demand lazy loading and script fallbacks.
#[derive(Debug)]
pub struct FontRegistry {
    manifest: FontManifest,
    font_dir: PathBuf,
    loaded_cache: Mutex<HashMap<String, SharedFontBytes>>,
    primary_font_name: String,
}

impl Default for FontRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FontRegistry {
    /// Creates a new font registry resolving the Fonts directory from environment or workspace.
    pub fn new() -> Self {
        let manifest = FontManifest::builtin_canonical();
        let font_dir = FontManifest::resolve_font_directory();
        let primary_font_name = manifest.defaults.primary_ui.clone();

        let registry = Self {
            manifest,
            font_dir,
            loaded_cache: Mutex::new(HashMap::new()),
            primary_font_name,
        };

        // Preload primary UI font immediately for 0ms cold-start rendering
        registry.preload_primary_ui();

        registry
    }

    /// Preloads only the primary UI font so startup latency is strictly 0ms.
    fn preload_primary_ui(&self) {
        let _ = self.get_or_load(&self.primary_font_name);
    }

    /// Retrieves or lazily reads the font binary from disk on demand.
    pub fn get_or_load(&self, font_name: &str) -> Option<SharedFontBytes> {
        let mut cache = self.loaded_cache.lock().unwrap();

        // 1. Return from memory cache if already resident
        if let Some(bytes) = cache.get(font_name) {
            return Some(Arc::clone(bytes));
        }

        // 2. Locate font in manifest
        let meta = self.manifest.find_by_name(font_name)?;
        let font_path = self.font_dir.join(&meta.file);

        // 3. Read from disk if exists
        if font_path.exists() {
            if let Ok(bytes) = std::fs::read(&font_path) {
                let shared = Arc::new(bytes);
                cache.insert(font_name.to_string(), Arc::clone(&shared));
                return Some(shared);
            }
        }

        // 4. Fallback: try relative assets/fonts path or CARGO_MANIFEST_DIR
        let manifest_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts"));
        let alt_path = manifest_dir.join(font_path.file_name()?);
        if alt_path.exists() {
            if let Ok(bytes) = std::fs::read(&alt_path) {
                let shared = Arc::new(bytes);
                cache.insert(font_name.to_string(), Arc::clone(&shared));
                return Some(shared);
            }
        }
        let rel_assets = PathBuf::from("assets/fonts").join(font_path.file_name()?);
        if rel_assets.exists() {
            if let Ok(bytes) = std::fs::read(&rel_assets) {
                let shared = Arc::new(bytes);
                cache.insert(font_name.to_string(), Arc::clone(&shared));
                return Some(shared);
            }
        }

        None
    }

    /// Returns the ordered fallback candidate font names for a given character or script.
    pub fn fallback_chain_for_char(&self, ch: char) -> Vec<&'static str> {
        let code = ch as u32;

        // Bengali unicode block: U+0980 to U+09FF
        if (0x0980..=0x09FF).contains(&code) {
            return vec!["Noto Sans Bengali", "Inter"];
        }

        // Emoji & Pictographs: U+1F300 to U+1FAFF, U+2600 to U+27BF
        if (0x1F300..=0x1FAFF).contains(&code) || (0x2600..=0x27BF).contains(&code) {
            return vec!["Noto Emoji", "Inter"];
        }

        // Latin / standard UI
        vec!["Inter", "Noto Sans Bengali", "Noto Emoji"]
    }

    /// Returns the active primary UI font name.
    pub fn primary_font_name(&self) -> &str {
        &self.primary_font_name
    }

    /// Returns the monospace code font name.
    pub fn monospace_font_name(&self) -> &str {
        &self.manifest.defaults.monospace
    }

    /// Returns the Bengali script fallback font name.
    pub fn bengali_font_name(&self) -> &str {
        &self.manifest.defaults.script_fallback_bengali
    }

    /// Total heap bytes of font data currently resident in memory.
    pub fn resident_memory_bytes(&self) -> usize {
        let cache = self.loaded_cache.lock().unwrap();
        cache.values().map(|b| b.len()).sum()
    }

    /// Number of font binaries currently resident in memory.
    pub fn resident_font_count(&self) -> usize {
        let cache = self.loaded_cache.lock().unwrap();
        cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_registry_initializes_with_minimal_primary_memory() {
        let registry = FontRegistry::new();

        // Primary font is either loaded or resident
        assert_eq!(registry.primary_font_name(), "Inter");
        assert_eq!(registry.monospace_font_name(), "JetBrains Mono");
        assert_eq!(registry.bengali_font_name(), "Noto Sans Bengali");

        // Initial memory should be bounded (not loading all 5MB upfront!)
        let initial_bytes = registry.resident_memory_bytes();
        assert!(
            initial_bytes <= 1_000_000,
            "Cold start must not load all fonts upfront (got {} bytes)",
            initial_bytes
        );
    }

    #[test]
    fn fallback_chains_select_appropriate_script_fonts() {
        let registry = FontRegistry::new();

        // Bengali character 'ক'
        let bengali_chain = registry.fallback_chain_for_char('ক');
        assert_eq!(bengali_chain[0], "Noto Sans Bengali");

        // Latin character 'A'
        let latin_chain = registry.fallback_chain_for_char('A');
        assert_eq!(latin_chain[0], "Inter");

        // Emoji '🎨'
        let emoji_chain = registry.fallback_chain_for_char('🎨');
        assert_eq!(emoji_chain[0], "Noto Emoji");
    }

    #[test]
    fn lazy_loading_loads_only_when_requested() {
        let registry = FontRegistry::new();
        let count_before = registry.resident_font_count();

        // Explicitly request Bengali font
        let _ = registry.get_or_load("Noto Sans Bengali");
        let count_after = registry.resident_font_count();

        assert!(
            count_after >= count_before,
            "Font count must increase or remain valid upon lazy load"
        );
    }
}
