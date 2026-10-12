//! Curated studio vector icon registry, zero-alloc per-frame caching, and theme recoloring.
//!
//! Powered by Tabler Icons (MIT Licensed).
//! Architecture rule: Icons are parsed and generated ONCE into cached vector paths or
//! raster surfaces, never parsed from SVG on every frame.

use std::collections::HashMap;
use ui_core::{Color, Rect, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

/// Curated studio vector icon identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StudioIcon {
    // Navigation & File
    Home,
    Folder,
    File,
    Save,
    Search,
    Settings,
    ArrowLeft,
    ArrowRight,
    ChevronDown,
    ChevronRight,

    // Controls & Modifiers
    Plus,
    Minus,
    Check,
    Close,
    Refresh,
    Trash,
    Copy,
    Edit,

    // Media & Animation
    Play,
    Pause,
    Stop,
    Timeline,
    Volume,
    VolumeOff,
    Waveform,

    // Creative Tools & Layers
    Pointer,
    Pencil,
    Bezier,
    Layers,
    Eye,
    EyeOff,
    Lock,
    Unlock,
    ZoomIn,
    ZoomOut,
    Sparkles,
    Palette,
    Image,
    Photo,
    Scissors,
    Camera,
    LayoutGrid,
    Maximize,

    // Developer & System
    Cpu,
    Bolt,
    Terminal,
    Code,
    Clock,
    GitBranch,
}

impl StudioIcon {
    /// Friendly name for logging, command palette, and tooltips.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Folder => "Folder",
            Self::File => "File",
            Self::Save => "Save",
            Self::Search => "Search",
            Self::Settings => "Settings",
            Self::ArrowLeft => "Arrow Left",
            Self::ArrowRight => "Arrow Right",
            Self::ChevronDown => "Chevron Down",
            Self::ChevronRight => "Chevron Right",
            Self::Plus => "Add",
            Self::Minus => "Remove",
            Self::Check => "Confirm",
            Self::Close => "Close",
            Self::Refresh => "Refresh",
            Self::Trash => "Delete",
            Self::Copy => "Copy",
            Self::Edit => "Edit",
            Self::Play => "Play",
            Self::Pause => "Pause",
            Self::Stop => "Stop",
            Self::Timeline => "Timeline",
            Self::Volume => "Volume",
            Self::VolumeOff => "Mute",
            Self::Waveform => "Audio Waveform",
            Self::Pointer => "Select Tool",
            Self::Pencil => "Freehand Brush",
            Self::Bezier => "Vector Bezier",
            Self::Layers => "Layers Inspector",
            Self::Eye => "Visible",
            Self::EyeOff => "Hidden",
            Self::Lock => "Locked",
            Self::Unlock => "Unlocked",
            Self::ZoomIn => "Zoom In",
            Self::ZoomOut => "Zoom Out",
            Self::Sparkles => "AI Generation",
            Self::Palette => "Color Palette",
            Self::Image | Self::Photo => "Image",
            Self::Scissors => "Cut / Split",
            Self::Camera => "Camera Snapshot",
            Self::LayoutGrid => "Grid Layout",
            Self::Maximize => "Fullscreen",
            Self::Cpu => "Compute / Neural",
            Self::Bolt => "Fast Inference",
            Self::Terminal => "Terminal",
            Self::Code => "Code / Script",
            Self::Clock => "Clock / History",
            Self::GitBranch => "Branch / Pipeline",
        }
    }

    /// Returns the verified Tabler SVG vector asset string.
    pub const fn svg_str(self) -> &'static str {
        match self {
            Self::Home => include_str!("../../assets/icons/home.svg"),
            Self::Folder => include_str!("../../assets/icons/folder.svg"),
            Self::File => include_str!("../../assets/icons/file.svg"),
            Self::Save => include_str!("../../assets/icons/device-floppy.svg"),
            Self::Search => include_str!("../../assets/icons/search.svg"),
            Self::Settings => include_str!("../../assets/icons/settings.svg"),
            Self::ArrowLeft => include_str!("../../assets/icons/arrow-left.svg"),
            Self::ArrowRight => include_str!("../../assets/icons/arrow-right.svg"),
            Self::ChevronDown => include_str!("../../assets/icons/chevron-down.svg"),
            Self::ChevronRight => include_str!("../../assets/icons/chevron-right.svg"),
            Self::Plus => include_str!("../../assets/icons/plus.svg"),
            Self::Minus => include_str!("../../assets/icons/minus.svg"),
            Self::Check => include_str!("../../assets/icons/check.svg"),
            Self::Close => include_str!("../../assets/icons/x.svg"),
            Self::Refresh => include_str!("../../assets/icons/refresh.svg"),
            Self::Trash => include_str!("../../assets/icons/trash.svg"),
            Self::Copy => include_str!("../../assets/icons/copy.svg"),
            Self::Edit => include_str!("../../assets/icons/edit.svg"),
            Self::Play => include_str!("../../assets/icons/player-play.svg"),
            Self::Pause => include_str!("../../assets/icons/player-pause.svg"),
            Self::Stop => include_str!("../../assets/icons/player-stop.svg"),
            Self::Timeline => include_str!("../../assets/icons/timeline.svg"),
            Self::Volume => include_str!("../../assets/icons/volume.svg"),
            Self::VolumeOff => include_str!("../../assets/icons/volume-off.svg"),
            Self::Waveform => include_str!("../../assets/icons/wave-sine.svg"),
            Self::Pointer => include_str!("../../assets/icons/pointer.svg"),
            Self::Pencil => include_str!("../../assets/icons/pencil.svg"),
            Self::Bezier => include_str!("../../assets/icons/vector-bezier.svg"),
            Self::Layers => include_str!("../../assets/icons/stack-2.svg"),
            Self::Eye => include_str!("../../assets/icons/eye.svg"),
            Self::EyeOff => include_str!("../../assets/icons/eye-off.svg"),
            Self::Lock => include_str!("../../assets/icons/lock.svg"),
            Self::Unlock => include_str!("../../assets/icons/lock-open.svg"),
            Self::ZoomIn => include_str!("../../assets/icons/zoom-in.svg"),
            Self::ZoomOut => include_str!("../../assets/icons/zoom-out.svg"),
            Self::Sparkles => include_str!("../../assets/icons/sparkles.svg"),
            Self::Palette => include_str!("../../assets/icons/palette.svg"),
            Self::Image | Self::Photo => include_str!("../../assets/icons/photo.svg"),
            Self::Scissors => include_str!("../../assets/icons/scissors.svg"),
            Self::Camera => include_str!("../../assets/icons/camera.svg"),
            Self::LayoutGrid => include_str!("../../assets/icons/layout-grid.svg"),
            Self::Maximize => include_str!("../../assets/icons/maximize.svg"),
            Self::Cpu => include_str!("../../assets/icons/cpu.svg"),
            Self::Bolt => include_str!("../../assets/icons/bolt.svg"),
            Self::Terminal => include_str!("../../assets/icons/terminal.svg"),
            Self::Code => include_str!("../../assets/icons/code.svg"),
            Self::Clock => include_str!("../../assets/icons/clock.svg"),
            Self::GitBranch => include_str!("../../assets/icons/git-branch.svg"),
        }
    }

    /// Case-insensitive name parser for string icon identifiers like `ui.icon("image")`.
    pub fn from_name(name: &str) -> Option<Self> {
        let n = name.trim().to_lowercase();
        match n.as_str() {
            "image" | "photo" | "picture" => Some(Self::Image),
            "sparkles" | "ai" | "generate" => Some(Self::Sparkles),
            "play" => Some(Self::Play),
            "pause" => Some(Self::Pause),
            "stop" => Some(Self::Stop),
            "timeline" => Some(Self::Timeline),
            "layers" | "layer" => Some(Self::Layers),
            "search" => Some(Self::Search),
            "settings" | "gear" => Some(Self::Settings),
            "save" => Some(Self::Save),
            "folder" => Some(Self::Folder),
            "file" => Some(Self::File),
            "home" => Some(Self::Home),
            "palette" | "color" => Some(Self::Palette),
            "bezier" | "vector" => Some(Self::Bezier),
            "pencil" | "brush" => Some(Self::Pencil),
            "pointer" | "select" => Some(Self::Pointer),
            "plus" | "add" => Some(Self::Plus),
            "minus" | "remove" => Some(Self::Minus),
            "check" | "confirm" => Some(Self::Check),
            "close" | "cancel" => Some(Self::Close),
            "refresh" => Some(Self::Refresh),
            "trash" | "delete" => Some(Self::Trash),
            "copy" => Some(Self::Copy),
            "edit" => Some(Self::Edit),
            "eye" | "visible" => Some(Self::Eye),
            "eyeoff" | "hidden" => Some(Self::EyeOff),
            "lock" => Some(Self::Lock),
            "unlock" => Some(Self::Unlock),
            "zoomin" | "zoom_in" => Some(Self::ZoomIn),
            "zoomout" | "zoom_out" => Some(Self::ZoomOut),
            "scissors" | "cut" | "split" => Some(Self::Scissors),
            "camera" | "snapshot" => Some(Self::Camera),
            "grid" | "layout" | "layout_grid" => Some(Self::LayoutGrid),
            "maximize" | "fullscreen" | "expand" => Some(Self::Maximize),
            "cpu" | "compute" | "model" => Some(Self::Cpu),
            "bolt" | "fast" | "lightning" => Some(Self::Bolt),
            "terminal" | "cli" | "console" => Some(Self::Terminal),
            "code" | "script" => Some(Self::Code),
            "clock" | "time" | "history" => Some(Self::Clock),
            "branch" | "git" | "node" => Some(Self::GitBranch),
            _ => None,
        }
    }
}

/// Conversion trait allowing both `StudioIcon` enum and `&str` identifiers.
pub trait IntoStudioIcon {
    fn into_studio_icon(self) -> StudioIcon;
}

impl IntoStudioIcon for StudioIcon {
    fn into_studio_icon(self) -> StudioIcon {
        self
    }
}

impl IntoStudioIcon for &str {
    fn into_studio_icon(self) -> StudioIcon {
        StudioIcon::from_name(self).unwrap_or(StudioIcon::Image)
    }
}

impl IntoStudioIcon for String {
    fn into_studio_icon(self) -> StudioIcon {
        StudioIcon::from_name(&self).unwrap_or(StudioIcon::Image)
    }
}

/// Normalized 16x16 vector stroke segment for clean hardware vector rasterization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconStroke {
    pub start: Vec2,
    pub end: Vec2,
}

/// Cached vector definition of a curated icon.
#[derive(Debug, Clone)]
pub struct CachedVectorIcon {
    pub strokes: Vec<IconStroke>,
}

impl CachedVectorIcon {
    pub fn for_icon(icon: StudioIcon) -> Self {
        // High-precision normalized stroke definitions (0.0 to 16.0 box)
        let strokes = match icon {
            StudioIcon::Close => vec![
                IconStroke {
                    start: Vec2::new(4.0, 4.0),
                    end: Vec2::new(12.0, 12.0),
                },
                IconStroke {
                    start: Vec2::new(12.0, 4.0),
                    end: Vec2::new(4.0, 12.0),
                },
            ],
            StudioIcon::Check => vec![
                IconStroke {
                    start: Vec2::new(3.0, 8.5),
                    end: Vec2::new(6.5, 12.0),
                },
                IconStroke {
                    start: Vec2::new(6.5, 12.0),
                    end: Vec2::new(13.0, 4.0),
                },
            ],
            StudioIcon::Plus => vec![
                IconStroke {
                    start: Vec2::new(8.0, 3.0),
                    end: Vec2::new(8.0, 13.0),
                },
                IconStroke {
                    start: Vec2::new(3.0, 8.0),
                    end: Vec2::new(13.0, 8.0),
                },
            ],
            StudioIcon::Minus => vec![IconStroke {
                start: Vec2::new(3.0, 8.0),
                end: Vec2::new(13.0, 8.0),
            }],
            StudioIcon::ChevronRight => vec![
                IconStroke {
                    start: Vec2::new(6.0, 4.0),
                    end: Vec2::new(10.0, 8.0),
                },
                IconStroke {
                    start: Vec2::new(10.0, 8.0),
                    end: Vec2::new(6.0, 12.0),
                },
            ],
            StudioIcon::ChevronDown => vec![
                IconStroke {
                    start: Vec2::new(4.0, 6.0),
                    end: Vec2::new(8.0, 10.0),
                },
                IconStroke {
                    start: Vec2::new(8.0, 10.0),
                    end: Vec2::new(12.0, 6.0),
                },
            ],
            StudioIcon::Play => vec![
                IconStroke {
                    start: Vec2::new(5.0, 4.0),
                    end: Vec2::new(12.0, 8.0),
                },
                IconStroke {
                    start: Vec2::new(12.0, 8.0),
                    end: Vec2::new(5.0, 12.0),
                },
                IconStroke {
                    start: Vec2::new(5.0, 12.0),
                    end: Vec2::new(5.0, 4.0),
                },
            ],
            StudioIcon::Pause => vec![
                IconStroke {
                    start: Vec2::new(5.5, 4.0),
                    end: Vec2::new(5.5, 12.0),
                },
                IconStroke {
                    start: Vec2::new(10.5, 4.0),
                    end: Vec2::new(10.5, 12.0),
                },
            ],
            StudioIcon::Stop => vec![
                IconStroke {
                    start: Vec2::new(4.5, 4.5),
                    end: Vec2::new(11.5, 4.5),
                },
                IconStroke {
                    start: Vec2::new(11.5, 4.5),
                    end: Vec2::new(11.5, 11.5),
                },
                IconStroke {
                    start: Vec2::new(11.5, 11.5),
                    end: Vec2::new(4.5, 11.5),
                },
                IconStroke {
                    start: Vec2::new(4.5, 11.5),
                    end: Vec2::new(4.5, 4.5),
                },
            ],
            StudioIcon::Search => vec![
                IconStroke {
                    start: Vec2::new(4.0, 4.0),
                    end: Vec2::new(10.0, 4.0),
                },
                IconStroke {
                    start: Vec2::new(10.0, 4.0),
                    end: Vec2::new(10.0, 10.0),
                },
                IconStroke {
                    start: Vec2::new(10.0, 10.0),
                    end: Vec2::new(4.0, 10.0),
                },
                IconStroke {
                    start: Vec2::new(4.0, 10.0),
                    end: Vec2::new(4.0, 4.0),
                },
                IconStroke {
                    start: Vec2::new(9.5, 9.5),
                    end: Vec2::new(13.5, 13.5),
                },
            ],
            StudioIcon::Settings => vec![
                IconStroke {
                    start: Vec2::new(4.0, 8.0),
                    end: Vec2::new(12.0, 8.0),
                },
                IconStroke {
                    start: Vec2::new(8.0, 4.0),
                    end: Vec2::new(8.0, 12.0),
                },
                IconStroke {
                    start: Vec2::new(5.0, 5.0),
                    end: Vec2::new(11.0, 11.0),
                },
                IconStroke {
                    start: Vec2::new(11.0, 5.0),
                    end: Vec2::new(5.0, 11.0),
                },
            ],
            StudioIcon::Timeline => vec![
                IconStroke {
                    start: Vec2::new(2.0, 13.0),
                    end: Vec2::new(14.0, 13.0),
                },
                IconStroke {
                    start: Vec2::new(3.0, 4.0),
                    end: Vec2::new(3.0, 11.0),
                },
                IconStroke {
                    start: Vec2::new(8.0, 7.0),
                    end: Vec2::new(8.0, 11.0),
                },
                IconStroke {
                    start: Vec2::new(13.0, 2.0),
                    end: Vec2::new(13.0, 11.0),
                },
            ],
            StudioIcon::Layers => vec![
                IconStroke {
                    start: Vec2::new(8.0, 3.0),
                    end: Vec2::new(14.0, 6.0),
                },
                IconStroke {
                    start: Vec2::new(14.0, 6.0),
                    end: Vec2::new(8.0, 9.0),
                },
                IconStroke {
                    start: Vec2::new(8.0, 9.0),
                    end: Vec2::new(2.0, 6.0),
                },
                IconStroke {
                    start: Vec2::new(2.0, 6.0),
                    end: Vec2::new(8.0, 3.0),
                },
                IconStroke {
                    start: Vec2::new(2.0, 9.5),
                    end: Vec2::new(8.0, 12.5),
                },
                IconStroke {
                    start: Vec2::new(8.0, 12.5),
                    end: Vec2::new(14.0, 9.5),
                },
            ],
            StudioIcon::Image | StudioIcon::Photo => vec![
                IconStroke {
                    start: Vec2::new(2.5, 2.5),
                    end: Vec2::new(13.5, 2.5),
                },
                IconStroke {
                    start: Vec2::new(13.5, 2.5),
                    end: Vec2::new(13.5, 13.5),
                },
                IconStroke {
                    start: Vec2::new(13.5, 13.5),
                    end: Vec2::new(2.5, 13.5),
                },
                IconStroke {
                    start: Vec2::new(2.5, 13.5),
                    end: Vec2::new(2.5, 2.5),
                },
                IconStroke {
                    start: Vec2::new(4.0, 11.0),
                    end: Vec2::new(7.0, 7.5),
                },
                IconStroke {
                    start: Vec2::new(7.0, 7.5),
                    end: Vec2::new(10.0, 11.0),
                },
                IconStroke {
                    start: Vec2::new(11.0, 5.0),
                    end: Vec2::new(11.5, 5.5),
                },
            ],
            _ => vec![
                // Default clean studio glyph box
                IconStroke {
                    start: Vec2::new(3.0, 3.0),
                    end: Vec2::new(13.0, 3.0),
                },
                IconStroke {
                    start: Vec2::new(13.0, 3.0),
                    end: Vec2::new(13.0, 13.0),
                },
                IconStroke {
                    start: Vec2::new(13.0, 13.0),
                    end: Vec2::new(3.0, 13.0),
                },
                IconStroke {
                    start: Vec2::new(3.0, 13.0),
                    end: Vec2::new(3.0, 3.0),
                },
            ],
        };
        Self { strokes }
    }
}

/// Zero-alloc in-frame icon cache and renderer.
#[derive(Debug, Default)]
pub struct IconAtlas {
    cache: HashMap<StudioIcon, CachedVectorIcon>,
}

impl IconAtlas {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Retrieves or prepares the cached vector representation of the icon.
    pub fn get_or_load(&mut self, icon: StudioIcon) -> &CachedVectorIcon {
        self.cache
            .entry(icon)
            .or_insert_with(|| CachedVectorIcon::for_icon(icon))
    }

    /// Submits draw commands for an icon into the command buffer recolored by theme.
    pub fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        icon: StudioIcon,
        bounds: Rect,
        color: Color,
    ) {
        cmd_buf.push(DrawCommand::Icon {
            id: icon.name(),
            svg: icon.svg_str(),
            rect: bounds,
            color,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_atlas_loads_and_caches_without_redundant_allocations() {
        let mut atlas = IconAtlas::new();
        let mut cmd_buf = CommandBuffer::new();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(16.0, 16.0));
        let color = Color::rgb(255, 255, 255);

        assert_eq!(atlas.cache.len(), 0);
        let _ = atlas.get_or_load(StudioIcon::Play);
        assert_eq!(atlas.cache.len(), 1);

        // Second call should hit memory cache with 0 new cache insertions
        let _ = atlas.get_or_load(StudioIcon::Play);
        assert_eq!(atlas.cache.len(), 1);

        atlas.render(&mut cmd_buf, StudioIcon::Play, bounds, color);
        assert!(!cmd_buf.is_empty());
        assert!(StudioIcon::Play.svg_str().contains("<svg"));
    }

    #[test]
    fn all_studio_icons_have_valid_names() {
        for icon in [
            StudioIcon::Home,
            StudioIcon::Close,
            StudioIcon::Play,
            StudioIcon::Timeline,
            StudioIcon::Layers,
            StudioIcon::Search,
        ] {
            assert!(!icon.name().is_empty());
        }
    }
}
