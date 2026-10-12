//! Visual node graph editor: draggable nodes, typed pins, and cubic bezier links.

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct NodePin {
    pub id: String,
    pub name: String,
    pub is_output: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub pos: Vec2, // Canvas coordinates
    pub size: Vec2,
    pub inputs: Vec<NodePin>,
    pub outputs: Vec<NodePin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeConnection {
    pub from_node: String,
    pub from_pin: String,
    pub to_node: String,
    pub to_pin: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeEditor {
    pub nodes: Vec<GraphNode>,
    pub connections: Vec<NodeConnection>,
    pub selected_node: Option<String>,
}

impl Default for NodeEditor {
    fn default() -> Self {
        Self {
            nodes: vec![
                GraphNode {
                    id: "node_texture".into(),
                    title: "Texture Sampler".into(),
                    pos: Vec2::new(80.0, 100.0),
                    size: Vec2::new(170.0, 120.0),
                    inputs: vec![],
                    outputs: vec![
                        NodePin {
                            id: "out_rgb".into(),
                            name: "RGB".into(),
                            is_output: true,
                        },
                        NodePin {
                            id: "out_alpha".into(),
                            name: "Alpha".into(),
                            is_output: true,
                        },
                    ],
                },
                GraphNode {
                    id: "node_color_correct".into(),
                    title: "Color Grading".into(),
                    pos: Vec2::new(320.0, 120.0),
                    size: Vec2::new(170.0, 140.0),
                    inputs: vec![
                        NodePin {
                            id: "in_image".into(),
                            name: "Image".into(),
                            is_output: false,
                        },
                        NodePin {
                            id: "in_mask".into(),
                            name: "Mask".into(),
                            is_output: false,
                        },
                    ],
                    outputs: vec![NodePin {
                        id: "out_result".into(),
                        name: "Result".into(),
                        is_output: true,
                    }],
                },
                GraphNode {
                    id: "node_output".into(),
                    title: "Composite Output".into(),
                    pos: Vec2::new(560.0, 140.0),
                    size: Vec2::new(160.0, 90.0),
                    inputs: vec![NodePin {
                        id: "in_final".into(),
                        name: "Final".into(),
                        is_output: false,
                    }],
                    outputs: vec![],
                },
            ],
            connections: vec![
                NodeConnection {
                    from_node: "node_texture".into(),
                    from_pin: "out_rgb".into(),
                    to_node: "node_color_correct".into(),
                    to_pin: "in_image".into(),
                },
                NodeConnection {
                    from_node: "node_color_correct".into(),
                    from_pin: "out_result".into(),
                    to_node: "node_output".into(),
                    to_pin: "in_final".into(),
                },
            ],
            selected_node: None,
        }
    }
}

impl NodeEditor {
    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Background
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.canvas,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Draw connections first (behind nodes) using cubic bezier wires
        for conn in &self.connections {
            if let (Some(from_n), Some(to_n)) = (
                self.nodes.iter().find(|n| n.id == conn.from_node),
                self.nodes.iter().find(|n| n.id == conn.to_node),
            ) {
                let start = Vec2::new(
                    viewport.min.x + from_n.pos.x + from_n.size.x,
                    viewport.min.y + from_n.pos.y + 45.0,
                );
                let end = Vec2::new(
                    viewport.min.x + to_n.pos.x,
                    viewport.min.y + to_n.pos.y + 45.0,
                );
                let dist = (end.x - start.x).abs() * 0.5;
                let ctrl1 = Vec2::new(start.x + dist, start.y);
                let ctrl2 = Vec2::new(end.x - dist, end.y);

                buf.push(DrawCommand::CubicBezier {
                    start,
                    ctrl1,
                    ctrl2,
                    end,
                    stroke: Stroke::new(2.5, palette.accent),
                });
            }
        }

        // Draw nodes
        for node in &self.nodes {
            let node_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + node.pos.x, viewport.min.y + node.pos.y),
                node.size,
            );

            let is_sel = self.selected_node.as_deref() == Some(&node.id);

            // Node card background
            buf.push(DrawCommand::Rect {
                rect: node_rect,
                fill: palette.surface,
                radius: CornerRadius::same(8.0),
                stroke: if is_sel {
                    Stroke::new(2.0, palette.accent)
                } else {
                    Stroke::new(1.0, palette.border)
                },
            });

            // Node header
            let header_rect = Rect::from_min_size(node_rect.min, Vec2::new(node.size.x, 28.0));
            buf.push(DrawCommand::Rect {
                rect: header_rect,
                fill: palette.elevated,
                radius: CornerRadius {
                    top_left: 8.0,
                    top_right: 8.0,
                    bottom_left: 0.0,
                    bottom_right: 0.0,
                },
                stroke: Stroke::NONE,
            });

            buf.push(DrawCommand::Text {
                text: node.title.clone(),
                pos: Vec2::new(header_rect.min.x + 10.0, header_rect.min.y + 7.0),
                size: 12.0,
                color: palette.text,
                strong: true,
            });

            // Input pins
            let mut y = node_rect.min.y + 40.0;
            for in_pin in &node.inputs {
                // Pin circle
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(node_rect.min.x - 4.0, y + 2.0),
                        Vec2::new(8.0, 8.0),
                    ),
                    fill: palette.accent,
                    radius: CornerRadius::same(4.0),
                    stroke: Stroke::new(1.0, palette.surface),
                });
                buf.push(DrawCommand::Text {
                    text: in_pin.name.clone(),
                    pos: Vec2::new(node_rect.min.x + 12.0, y),
                    size: 11.0,
                    color: palette.text_muted,
                    strong: false,
                });
                y += 24.0;
            }

            // Output pins
            let mut y_out = node_rect.min.y + 40.0;
            for out_pin in &node.outputs {
                buf.push(DrawCommand::Rect {
                    rect: Rect::from_min_size(
                        Vec2::new(node_rect.max.x - 4.0, y_out + 2.0),
                        Vec2::new(8.0, 8.0),
                    ),
                    fill: palette.success,
                    radius: CornerRadius::same(4.0),
                    stroke: Stroke::new(1.0, palette.surface),
                });
                buf.push(DrawCommand::Text {
                    text: out_pin.name.clone(),
                    pos: Vec2::new(node_rect.max.x - 55.0, y_out),
                    size: 11.0,
                    color: palette.text_muted,
                    strong: false,
                });
                y_out += 24.0;
            }
        }

        buf.push(DrawCommand::ClipPop);
    }
}
