use crate::theme::ThemePalette;
use ui_core::{Color, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    SearchAdd,
    AddTexture,
    AddStylize,
    AddFilter,
    AddSeamless,
    AddPbr,
    AddHeightToNormal,
    AddVideo,
    AddExport,
    AddReroute,
    DeleteSelected,
    AlignLeft,
    AlignTop,
    ResetView,
    ToggleMinimap,
}

impl ToolbarAction {
    pub fn display_name(&self) -> &'static str {
        match self {
            ToolbarAction::SearchAdd => "+ Add Node...",
            ToolbarAction::AddTexture => "+ Texture",
            ToolbarAction::AddStylize => "+ Stylize",
            ToolbarAction::AddFilter => "+ Filter",
            ToolbarAction::AddSeamless => "+ Seamless",
            ToolbarAction::AddPbr => "+ PBR Maps",
            ToolbarAction::AddHeightToNormal => "+ Normal Map",
            ToolbarAction::AddVideo => "+ Present",
            ToolbarAction::AddExport => "+ Export",
            ToolbarAction::AddReroute => "+ Reroute",
            ToolbarAction::DeleteSelected => "Delete",
            ToolbarAction::AlignLeft => "Align Left",
            ToolbarAction::AlignTop => "Align Top",
            ToolbarAction::ResetView => "Reset Pan",
            ToolbarAction::ToggleMinimap => "Minimap",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetNode {
    TextureSource,
    StylizeColor,
    MakeSeamless,
    PbrGenerator,
    HeightToNormal,
    PresentationVideo,
    Reroute,
}

impl PresetNode {
    pub fn title(&self) -> &'static str {
        match self {
            PresetNode::TextureSource => "Texture Source",
            PresetNode::StylizeColor => "Color Grade / Stylize",
            PresetNode::MakeSeamless => "Seamless Autocorrelator",
            PresetNode::PbrGenerator => "PBR Generator (7-Map)",
            PresetNode::HeightToNormal => "Height to Normal",
            PresetNode::PresentationVideo => "Presentation Video",
            PresetNode::Reroute => "Reroute",
        }
    }

    pub fn display_name(&self) -> &'static str {
        self.title()
    }

    pub fn category(&self) -> &'static str {
        match self {
            PresetNode::TextureSource => "Source",
            PresetNode::StylizeColor => "Processing",
            PresetNode::MakeSeamless => "Processing",
            PresetNode::PbrGenerator => "Material",
            PresetNode::HeightToNormal => "Material",
            PresetNode::PresentationVideo => "Output",
            PresetNode::Reroute => "Utility",
        }
    }

    pub fn search_description(&self) -> &'static str {
        match self {
            PresetNode::TextureSource => "Load image or procedural noise generator",
            PresetNode::StylizeColor => "Adjust hue, saturation, contrast and brightness",
            PresetNode::MakeSeamless => "Remove tiling seams via FFT autocorrelation",
            PresetNode::PbrGenerator => "Bake albedo, normal, roughness, metallic, AO",
            PresetNode::HeightToNormal => "Compute Sobel normal vectors from grayscale height",
            PresetNode::PresentationVideo => "Render turntable preview / showcase reel",
            PresetNode::Reroute => "Passive cable rerouting waypoint knot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeAlignment {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodePin {
    pub id: String,
    pub name: String,
    pub pin_type: String,
    pub is_output: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeGroup {
    pub id: String,
    pub title: String,
    pub bounds: Rect,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub pos: Vec2,
    pub size: Vec2,
    pub inputs: Vec<NodePin>,
    pub outputs: Vec<NodePin>,
    pub category: String,
    pub is_collapsed: bool,
    pub preview_color: Option<Color>,
}

impl GraphNode {
    pub fn is_reroute(&self) -> bool {
        self.category == "Utility" && self.title == "Reroute"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeConnection {
    pub from_node: String,
    pub from_pin: String,
    pub to_node: String,
    pub to_pin: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingWire {
    pub from_node: String,
    pub from_pin: String,
    pub is_output: bool,
    pub current_world_pos: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraggingNode {
    pub node_id: String,
    pub offset: Vec2,
}

/// Header/accent colour for a node category.
///
/// Every category gets its own hue; lightness and saturation follow the theme
/// (brighter on dark canvases, deeper on light ones), so headers stay readable
/// in every palette. Unknown categories get a stable hue derived from the name.
pub fn category_color(category: &str, palette: &ThemePalette) -> Color {
    let key = category.to_ascii_lowercase();
    let (hue, saturation) = match key.as_str() {
        "source" | "input" | "image" => (135.0, 1.0),
        "processing" | "process" => (212.0, 1.0),
        "filter" | "adjust" | "adjustment" => (176.0, 1.0),
        "composite" | "blend" => (248.0, 1.0),
        "material" | "pbr" => (32.0, 1.0),
        "shader" => (330.0, 1.0),
        "output" | "export" => (285.0, 1.0),
        "ai" | "generate" | "diffusion" => (12.0, 1.0),
        "utility" | "math" => (205.0, 0.35),
        _ => (name_hue(&key), 0.9),
    };
    themed_hue(hue, saturation, palette)
}

/// Wire and pin colour for a socket type, themed like [`category_color`].
pub fn pin_type_color(pin_type: &str, palette: &ThemePalette) -> Color {
    let key = pin_type.to_ascii_lowercase();
    let (hue, saturation) = match key.as_str() {
        "image" => (135.0, 1.0),
        "mask" => (55.0, 1.0),
        "float" | "number" => (212.0, 1.0),
        "color" => (345.0, 1.0),
        "pbrset" | "pbrmaterial" | "material" => (32.0, 1.0),
        "shaderwgsl" | "shader" => (330.0, 1.0),
        "video" => (285.0, 1.0),
        "any" => (0.0, 0.0),
        _ => (name_hue(&key), 0.9),
    };
    themed_hue(hue, saturation, palette)
}

/// Stable hue for a name: one of 24 steps around the wheel (FNV-1a hash).
fn name_hue(name: &str) -> f32 {
    let hash = name.bytes().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    });
    (hash % 24) as f32 * 15.0
}

/// HSL colour whose lightness and saturation suit the palette's canvas.
fn themed_hue(hue: f32, saturation_scale: f32, palette: &ThemePalette) -> Color {
    let dark = palette.canvas.luminance() < 0.2;
    let (saturation, lightness) = if dark { (0.62, 0.62) } else { (0.66, 0.40) };
    hsl(hue, saturation * saturation_scale, lightness)
}

fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    let to_u8 = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgb(to_u8(r), to_u8(g), to_u8(b))
}
