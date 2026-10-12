//! Font manifest parser and metadata models.
//!
//! Replicates ArtCraft (storytold/craft-fonts) manifest architecture:
//! Decouples font asset binaries, licensing, and checksums from application code.

use std::path::{Path, PathBuf};

/// Individual font asset metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAssetMeta {
    pub name: String,
    pub family: String,
    pub style: String,
    pub weight: u16,
    pub role: String,
    pub file: String,
    pub license: String,
    pub license_file: String,
    pub checksum_sha256: String,
    pub size_bytes: usize,
    pub upstream: String,
    pub preload: bool,
    pub scripts: Vec<String>,
}

/// Default font assignments for studio roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontDefaults {
    pub primary_ui: String,
    pub primary_ui_bold: String,
    pub monospace: String,
    pub code_alternate: String,
    pub script_fallback_bengali: String,
    pub emoji_fallback: String,
    pub display: String,
}

impl Default for FontDefaults {
    fn default() -> Self {
        Self {
            primary_ui: "Inter".into(),
            primary_ui_bold: "Inter SemiBold".into(),
            monospace: "JetBrains Mono".into(),
            code_alternate: "IBM Plex Mono".into(),
            script_fallback_bengali: "Noto Sans Bengali".into(),
            emoji_fallback: "Noto Emoji".into(),
            display: "Space Grotesk".into(),
        }
    }
}

/// Collection of fonts defined in Fonts/manifest.toml.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontManifest {
    pub version: String,
    pub defaults: FontDefaults,
    pub fonts: Vec<FontAssetMeta>,
}

impl Default for FontManifest {
    fn default() -> Self {
        Self::builtin_canonical()
    }
}

impl FontManifest {
    /// Canonical built-in font manifest.
    pub fn builtin_canonical() -> Self {
        Self {
            version: "1.0.0".into(),
            defaults: FontDefaults::default(),
            fonts: vec![
                FontAssetMeta {
                    name: "Inter".into(),
                    family: "Inter".into(),
                    style: "Regular".into(),
                    weight: 400,
                    role: "primary_ui".into(),
                    file: "binaries/Inter-Regular.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/Inter-LICENSE.txt".into(),
                    checksum_sha256:
                        "40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82".into(),
                    size_bytes: 411640,
                    upstream: "https://github.com/rsms/inter".into(),
                    preload: true,
                    scripts: vec!["latin".into(), "cyrillic".into(), "greek".into()],
                },
                FontAssetMeta {
                    name: "Inter SemiBold".into(),
                    family: "Inter".into(),
                    style: "SemiBold".into(),
                    weight: 600,
                    role: "primary_ui_bold".into(),
                    file: "binaries/Inter-SemiBold.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/Inter-LICENSE.txt".into(),
                    checksum_sha256:
                        "78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3".into(),
                    size_bytes: 419744,
                    upstream: "https://github.com/rsms/inter".into(),
                    preload: false,
                    scripts: vec!["latin".into(), "cyrillic".into(), "greek".into()],
                },
                FontAssetMeta {
                    name: "JetBrains Mono".into(),
                    family: "JetBrains Mono".into(),
                    style: "Regular".into(),
                    weight: 400,
                    role: "monospace".into(),
                    file: "binaries/JetBrainsMono-Regular.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/JetBrainsMono-LICENSE.txt".into(),
                    checksum_sha256:
                        "e6fd0d7e91550b3ed2b735d4312474362c4716edc4fc0577a0f61ed782d5aed1".into(),
                    size_bytes: 270224,
                    upstream: "https://github.com/JetBrains/JetBrainsMono".into(),
                    preload: false,
                    scripts: vec!["latin".into(), "cyrillic".into(), "symbols".into()],
                },
                FontAssetMeta {
                    name: "IBM Plex Mono".into(),
                    family: "IBM Plex Mono".into(),
                    style: "Regular".into(),
                    weight: 400,
                    role: "code_alternate".into(),
                    file: "binaries/IBMPlexMono-Regular.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/IBMPlexMono-LICENSE.txt".into(),
                    checksum_sha256:
                        "6a3412f058c7d8dfd9170c41e85ade48e5156ecb89356110ca57a0a27734af46".into(),
                    size_bytes: 135580,
                    upstream: "https://github.com/IBM/plex".into(),
                    preload: false,
                    scripts: vec!["latin".into(), "cyrillic".into()],
                },
                FontAssetMeta {
                    name: "Noto Sans Bengali".into(),
                    family: "Noto Sans Bengali".into(),
                    style: "Regular".into(),
                    weight: 400,
                    role: "script_fallback_bengali".into(),
                    file: "binaries/NotoSansBengali.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/NotoSansBengali-LICENSE.txt".into(),
                    checksum_sha256:
                        "dcd42978094e584a849c84a51450eeac40c8826057d566ea6d4b9627a403a05a".into(),
                    size_bytes: 463668,
                    upstream: "https://github.com/notofonts/bengali".into(),
                    preload: false,
                    scripts: vec!["bengali".into()],
                },
                FontAssetMeta {
                    name: "Noto Emoji".into(),
                    family: "Noto Emoji".into(),
                    style: "Regular".into(),
                    weight: 400,
                    role: "emoji_fallback".into(),
                    file: "binaries/NotoEmoji.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/NotoSansBengali-LICENSE.txt".into(),
                    checksum_sha256:
                        "de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551".into(),
                    size_bytes: 1982596,
                    upstream: "https://github.com/googlefonts/noto-emoji".into(),
                    preload: false,
                    scripts: vec!["emoji".into(), "symbols".into()],
                },
                FontAssetMeta {
                    name: "Space Grotesk".into(),
                    family: "Space Grotesk".into(),
                    style: "Bold".into(),
                    weight: 700,
                    role: "display".into(),
                    file: "binaries/SpaceGrotesk-Bold.ttf".into(),
                    license: "OFL-1.1".into(),
                    license_file: "licenses/SpaceGrotesk-LICENSE.txt".into(),
                    checksum_sha256:
                        "acad6de1fc93436f5c0f1f4137751ef04f1aea3063e7036535970ffcfbd79f72".into(),
                    size_bytes: 136676,
                    upstream: "https://github.com/floriankarsten/space-grotesk".into(),
                    preload: false,
                    scripts: vec!["latin".into()],
                },
            ],
        }
    }

    /// Finds a font asset by exact name or role.
    pub fn find_by_name(&self, name: &str) -> Option<&FontAssetMeta> {
        self.fonts
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn find_by_role(&self, role: &str) -> Option<&FontAssetMeta> {
        self.fonts
            .iter()
            .find(|f| f.role.eq_ignore_ascii_case(role))
    }

    /// Resolves the root directory of font assets.
    /// Priority:
    /// 1. KATHAMO_FONTS_DIR or KOLPONA_FONTS_DIR environment variable
    /// 2. `./assets/fonts` or `../assets/fonts` relative to Kathamo
    /// 3. Standard fallback roots
    pub fn resolve_font_directory() -> PathBuf {
        if let Ok(dir) = std::env::var("KATHAMO_FONTS_DIR").or_else(|_| std::env::var("KOLPONA_FONTS_DIR")) {
            let p = PathBuf::from(dir);
            if p.exists() {
                return p;
            }
        }
        if let Ok(dir) = std::env::var("CRAFT_FONTS_DIR") {
            let p = PathBuf::from(dir);
            if p.exists() {
                return p;
            }
        }

        let mut roots = Vec::new();
        roots.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/..")));
        roots.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")));
        if let Ok(cwd) = std::env::current_dir() {
            roots.extend(cwd.ancestors().map(Path::to_path_buf));
        }

        for root in roots {
            for sub in ["assets/fonts", "Fonts/binaries", "fonts", "Fonts"] {
                let candidate = root.join(sub);
                if candidate.exists() && (candidate.join("Inter-Regular.ttf").is_file() || candidate.join("manifest.toml").is_file()) {
                    return candidate;
                }
            }
        }

        PathBuf::from("assets/fonts")
    }
}
