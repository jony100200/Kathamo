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

pub fn category_color(category: &str, palette: &ThemePalette) -> Color {
    match category {
        "Source" => Color::rgb(0x38, 0x8E, 0x3C), // Forest Green
        "Processing" => Color::rgb(0x19, 0x76, 0xD2), // Cobalt Blue
        "Material" => Color::rgb(0xF5, 0x7C, 0x00), // Vibrant Amber/Orange
        "Output" => Color::rgb(0x7B, 0x1F, 0xA2), // Deep Purple
        "Utility" => Color::rgb(0x60, 0x7D, 0x8B), // Slate Gray
        _ => palette.accent,
    }
}

pub fn pin_type_color(pin_type: &str, palette: &ThemePalette) -> Color {
    match pin_type {
        "Image" => Color::rgb(0x4C, 0xAF, 0x50),
        "PbrSet" => Color::rgb(0xFF, 0x98, 0x00),
        "Float" => Color::rgb(0x21, 0x96, 0xF3),
        "Video" => Color::rgb(0x9C, 0x27, 0xB0),
        "Any" => Color::rgb(0x9E, 0x9E, 0x9E),
        _ => palette.accent,
    }
}
