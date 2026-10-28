use super::types::{GraphNode, NodeConnection, NodeGroup, NodePin, PresetNode};
use ui_core::{Color, Rect, Vec2};

impl PresetNode {
    pub fn all() -> &'static [PresetNode] {
        &[
            PresetNode::TextureSource,
            PresetNode::StylizeColor,
            PresetNode::MakeSeamless,
            PresetNode::PbrGenerator,
            PresetNode::HeightToNormal,
            PresetNode::PresentationVideo,
            PresetNode::Reroute,
        ]
    }

    pub fn create_node(&self, id: String, pos: Vec2) -> GraphNode {
        match self {
            PresetNode::TextureSource => GraphNode {
                id,
                title: "Texture Source".into(),
                pos,
                size: Vec2::new(180.0, 110.0),
                inputs: vec![],
                outputs: vec![
                    NodePin {
                        id: "out_rgb".into(),
                        name: "Base Image".into(),
                        is_output: true,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "out_alpha".into(),
                        name: "Alpha Mask".into(),
                        is_output: true,
                        pin_type: "Mask".into(),
                    },
                ],
                category: "Source".into(),
                is_collapsed: false,
                preview_color: Some(Color::rgb(245, 158, 11)),
            },
            PresetNode::StylizeColor => GraphNode {
                id,
                title: "Stylize & Color".into(),
                pos,
                size: Vec2::new(180.0, 130.0),
                inputs: vec![
                    NodePin {
                        id: "in_img".into(),
                        name: "Image In".into(),
                        is_output: false,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "in_mask".into(),
                        name: "Grade Mask".into(),
                        is_output: false,
                        pin_type: "Mask".into(),
                    },
                ],
                outputs: vec![NodePin {
                    id: "out_graded".into(),
                    name: "Color Result".into(),
                    is_output: true,
                    pin_type: "Image".into(),
                }],
                category: "Filter".into(),
                is_collapsed: false,
                preview_color: Some(Color::rgb(168, 85, 247)),
            },
            PresetNode::MakeSeamless => GraphNode {
                id,
                title: "Make Seamless".into(),
                pos,
                size: Vec2::new(180.0, 120.0),
                inputs: vec![NodePin {
                    id: "in_raw".into(),
                    name: "Input Map".into(),
                    is_output: false,
                    pin_type: "Image".into(),
                }],
                outputs: vec![
                    NodePin {
                        id: "out_seamless".into(),
                        name: "Seamless Tile".into(),
                        is_output: true,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "out_score".into(),
                        name: "Seam QA Score".into(),
                        is_output: true,
                        pin_type: "Float".into(),
                    },
                ],
                category: "Processing".into(),
                is_collapsed: false,
                preview_color: Some(Color::rgb(16, 185, 129)),
            },
            PresetNode::PbrGenerator => GraphNode {
                id,
                title: "PBR Generator".into(),
                pos,
                size: Vec2::new(180.0, 170.0),
                inputs: vec![NodePin {
                    id: "in_base".into(),
                    name: "BaseColor".into(),
                    is_output: false,
                    pin_type: "Image".into(),
                }],
                outputs: vec![
                    NodePin {
                        id: "out_normal".into(),
                        name: "Normal Map".into(),
                        is_output: true,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "out_roughness".into(),
                        name: "Roughness".into(),
                        is_output: true,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "out_height".into(),
                        name: "Height / Disp".into(),
                        is_output: true,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "out_ao".into(),
                        name: "Ambient Occl".into(),
                        is_output: true,
                        pin_type: "Image".into(),
                    },
                ],
                category: "PBR".into(),
                is_collapsed: false,
                preview_color: Some(Color::rgb(59, 130, 246)),
            },
            PresetNode::HeightToNormal => GraphNode {
                id,
                title: "Height to Normal".into(),
                pos,
                size: Vec2::new(180.0, 120.0),
                inputs: vec![NodePin {
                    id: "height_in".into(),
                    name: "Height (Grayscale)".into(),
                    pin_type: "Image".into(),
                    is_output: false,
                }],
                outputs: vec![NodePin {
                    id: "normal_out".into(),
                    name: "Tangent Normal".into(),
                    pin_type: "Image".into(),
                    is_output: true,
                }],
                category: "PBR".into(),
                is_collapsed: false,
                preview_color: Some(Color::rgb(59, 130, 246)),
            },
            PresetNode::PresentationVideo => GraphNode {
                id,
                title: "Presentation & Video".into(),
                pos,
                size: Vec2::new(190.0, 110.0),
                inputs: vec![
                    NodePin {
                        id: "in_norm".into(),
                        name: "Normal In".into(),
                        is_output: false,
                        pin_type: "Image".into(),
                    },
                    NodePin {
                        id: "in_rough".into(),
                        name: "Rough In".into(),
                        is_output: false,
                        pin_type: "Image".into(),
                    },
                ],
                outputs: vec![NodePin {
                    id: "out_turntable".into(),
                    name: "Video Reel".into(),
                    is_output: true,
                    pin_type: "Video".into(),
                }],
                category: "Export".into(),
                is_collapsed: false,
                preview_color: Some(Color::rgb(236, 72, 153)),
            },
            PresetNode::Reroute => GraphNode {
                id,
                title: "Reroute".into(),
                pos,
                size: Vec2::new(32.0, 32.0),
                inputs: vec![NodePin {
                    id: "in".into(),
                    name: "".into(),
                    pin_type: "Any".into(),
                    is_output: false,
                }],
                outputs: vec![NodePin {
                    id: "out".into(),
                    name: "".into(),
                    pin_type: "Any".into(),
                    is_output: true,
                }],
                category: "Utility".into(),
                is_collapsed: false,
                preview_color: None,
            },
        }
    }
}

pub fn create_default_graph() -> (Vec<GraphNode>, Vec<NodeConnection>, Vec<NodeGroup>) {
    let nodes = vec![
        PresetNode::TextureSource.create_node("node_source".into(), Vec2::new(60.0, 80.0)),
        PresetNode::StylizeColor.create_node("node_stylize".into(), Vec2::new(300.0, 80.0)),
        PresetNode::MakeSeamless.create_node("node_seamless".into(), Vec2::new(540.0, 80.0)),
        PresetNode::PbrGenerator.create_node("node_pbr".into(), Vec2::new(780.0, 60.0)),
        PresetNode::PresentationVideo.create_node("node_presentation".into(), Vec2::new(1020.0, 100.0)),
    ];

    let connections = vec![
        NodeConnection {
            from_node: "node_source".into(),
            from_pin: "out_rgb".into(),
            to_node: "node_stylize".into(),
            to_pin: "in_img".into(),
        },
        NodeConnection {
            from_node: "node_stylize".into(),
            from_pin: "out_graded".into(),
            to_node: "node_seamless".into(),
            to_pin: "in_raw".into(),
        },
        NodeConnection {
            from_node: "node_seamless".into(),
            from_pin: "out_seamless".into(),
            to_node: "node_pbr".into(),
            to_pin: "in_base".into(),
        },
        NodeConnection {
            from_node: "node_pbr".into(),
            from_pin: "out_normal".into(),
            to_node: "node_presentation".into(),
            to_pin: "in_norm".into(),
        },
    ];

    let groups = vec![NodeGroup {
        id: "group_pipeline".into(),
        title: "Texture Generation Pipeline".into(),
        bounds: Rect::from_min_size(Vec2::new(40.0, 40.0), Vec2::new(940.0, 220.0)),
        color: Color::rgb(59, 130, 246),
    }];

    (nodes, connections, groups)
}
