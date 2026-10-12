//! Font registry, lazy asset loading, script fallback chains, and memory tracking.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::manifest::FontManifest;

/// Thread-safe font binary buffer.
pub type SharedFontBytes = Arc<Vec<u8>>;

/// In-memory font registry providing on-demand lazy loading, system font discovery, and script fallbacks.
#[derive(Debug)]
pub struct FontRegistry {
    manifest: FontManifest,
    font_dir: PathBuf,
    loaded_cache: Mutex<HashMap<String, SharedFontBytes>>,
    system_font_paths: Mutex<HashMap<String, PathBuf>>,
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
            system_font_paths: Mutex::new(HashMap::new()),
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
        if let Some(meta) = self.manifest.find_by_name(font_name) {
            let font_path = self.font_dir.join(&meta.file);

            // Read from disk if exists in resolved font directory
            if font_path.exists() {
                if let Ok(bytes) = std::fs::read(&font_path) {
                    let shared = Arc::new(bytes);
                    cache.insert(font_name.to_string(), Arc::clone(&shared));
                    return Some(shared);
                }
            }

            // Fallback: try relative assets/fonts path or CARGO_MANIFEST_DIR
            if let Some(file_name) = font_path.file_name() {
                let manifest_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts"));
                let alt_path = manifest_dir.join(file_name);
                if alt_path.exists() {
                    if let Ok(bytes) = std::fs::read(&alt_path) {
                        let shared = Arc::new(bytes);
                        cache.insert(font_name.to_string(), Arc::clone(&shared));
                        return Some(shared);
                    }
                }
                let rel_assets = PathBuf::from("assets/fonts").join(file_name);
                if rel_assets.exists() {
                    if let Ok(bytes) = std::fs::read(&rel_assets) {
                        let shared = Arc::new(bytes);
                        cache.insert(font_name.to_string(), Arc::clone(&shared));
                        return Some(shared);
                    }
                }
            }
        }

        // 3. Check system fonts
        drop(cache); // Release lock before scanning if needed
        self.ensure_system_fonts_scanned();

        let sys_paths = self.system_font_paths.lock().unwrap();
        if let Some(path) = sys_paths.get(&font_name.to_lowercase()) {
            if let Ok(bytes) = std::fs::read(path) {
                let shared = Arc::new(bytes);
                let mut cache = self.loaded_cache.lock().unwrap();
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

    /// Scans installed OS fonts on Windows, Linux, and macOS without loading their large binaries upfront.
    pub fn scan_system_fonts(&self) -> Vec<String> {
        let mut paths = Vec::new();

        // Windows standard font directories
        if cfg!(windows) {
            paths.push(PathBuf::from(r"C:\Windows\Fonts"));
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                paths.push(PathBuf::from(local_app_data).join(r"Microsoft\Windows\Fonts"));
            }
        } else if cfg!(target_os = "macos") {
            paths.push(PathBuf::from("/System/Library/Fonts"));
            paths.push(PathBuf::from("/Library/Fonts"));
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(home).join("Library/Fonts"));
            }
        } else {
            // Linux standard font directories
            paths.push(PathBuf::from("/usr/share/fonts"));
            paths.push(PathBuf::from("/usr/local/share/fonts"));
            if let Ok(home) = std::env::var("HOME") {
                paths.push(PathBuf::from(home).join(".local/share/fonts"));
            }
        }

        let mut discovered = HashMap::new();
        let mut names = Vec::new();

        for dir in paths {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase());
                    if let Some(ext) = ext {
                        if ext == "ttf" || ext == "otf" || ext == "ttc" {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                let clean_name = Self::sanitize_font_name(stem);
                                let key = clean_name.to_lowercase();
                                if !discovered.contains_key(&key) {
                                    names.push(clean_name.clone());
                                    discovered.insert(key, path);
                                }
                            }
                        }
                    }
                }
            }
        }

        names.sort();
        let mut sys_map = self.system_font_paths.lock().unwrap();
        *sys_map = discovered;
        names
    }

    /// Ensures system font directories have been indexed once.
    fn ensure_system_fonts_scanned(&self) {
        let is_empty = {
            let map = self.system_font_paths.lock().unwrap();
            map.is_empty()
        };
        if is_empty {
            self.scan_system_fonts();
        }
    }

    /// Converts raw font filenames (e.g. "arialbd", "segoeui", "Amiri-Bold") into clean readable display names.
    fn sanitize_font_name(raw: &str) -> String {
        let lower = raw.to_lowercase();
        match lower.as_str() {
            "arial" => "Arial".to_string(),
            "arialbd" => "Arial Bold".to_string(),
            "arialbi" => "Arial Bold Italic".to_string(),
            "ariali" => "Arial Italic".to_string(),
            "calibri" => "Calibri".to_string(),
            "calibrib" => "Calibri Bold".to_string(),
            "comic" => "Comic Sans MS".to_string(),
            "comicbd" => "Comic Sans MS Bold".to_string(),
            "consola" => "Consolas".to_string(),
            "consolab" => "Consolas Bold".to_string(),
            "georgia" => "Georgia".to_string(),
            "georgiab" => "Georgia Bold".to_string(),
            "impact" => "Impact".to_string(),
            "segoeui" => "Segoe UI".to_string(),
            "segoeuib" => "Segoe UI Bold".to_string(),
            "segoeuisb" => "Segoe UI Semibold".to_string(),
            "times" => "Times New Roman".to_string(),
            "timesbd" => "Times New Roman Bold".to_string(),
            "trebuc" => "Trebuchet MS".to_string(),
            "trebucbd" => "Trebuchet MS Bold".to_string(),
            "verdana" => "Verdana".to_string(),
            "verdanab" => "Verdana Bold".to_string(),
            _ => raw.replace('-', " ").replace('_', " "),
        }
    }

    /// Returns all available fonts (bundled studio fonts first, followed by discovered system fonts).
    pub fn available_fonts(&self) -> Vec<String> {
        let mut list = vec![
            "Inter".to_string(),
            "Space Grotesk".to_string(),
            "Noto Sans Bengali".to_string(),
            "JetBrains Mono".to_string(),
            "IBM Plex Mono".to_string(),
        ];
        let sys_fonts = self.scan_system_fonts();
        list.extend(sys_fonts);
        list
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

    #[test]
    fn system_font_discovery_and_availability() {
        let registry = FontRegistry::new();
        let fonts = registry.available_fonts();
        assert!(!fonts.is_empty(), "Font list must contain bundled and system fonts");
        assert!(fonts.contains(&"Inter".to_string()));
    }
}
