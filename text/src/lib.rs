//! `ui-text`: Kolpona typography engine, font asset registry, lazy loading, and script fallbacks.
//!
//! Inspired by ArtCraft (storytold/craft-fonts):
//! Separates font binaries, checksums, and licensing from application code.
//! Loads primary UI font immediately for 0ms cold-start; lazily loads script fallbacks on demand.

pub mod manifest;
pub mod registry;

pub use manifest::{FontAssetMeta, FontDefaults, FontManifest};
pub use registry::{FontRegistry, SharedFontBytes};
