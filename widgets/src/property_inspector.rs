//! Reusable metadata-driven PropertyInspector for layers, objects, tools, and parameters.
//! Supports typed properties: floats, integers, booleans, enums, colors, Vec2, and text strings.

use crate::color_picker::ColorWheel;
use crate::theme::ThemePalette;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_platform::{PointerButton, PointerState};
use ui_render::{CommandBuffer, DrawCommand};

/// Strongly typed property value supported by the inspector.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Float {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        unit: String,
    },
    Int {
        value: i32,
        min: i32,
        max: i32,
    },
    Bool(bool),
    Text(String),
    Color(Color),
    Enum {
        selected: usize,
        options: Vec<String>,
    },
    Vec2 {
        x: f32,
        y: f32,
        min: f32,
        max: f32,
        step: f32,
    },
}

/// A metadata descriptor for an inspectable parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyItem {
    pub id: String,
    pub label: String,
    pub category: String,
    pub value: PropertyValue,
    pub tooltip: Option<String>,
}

impl PropertyItem {
    pub fn float(
        id: impl Into<String>,
        label: impl Into<String>,
        val: f32,
        min: f32,
        max: f32,
        step: f32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "General".into(),
            value: PropertyValue::Float {
                value: val,
                min,
                max,
                step,
                unit: String::new(),
            },
            tooltip: None,
        }
    }

    pub fn boolean(id: impl Into<String>, label: impl Into<String>, val: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "General".into(),
            value: PropertyValue::Bool(val),
            tooltip: None,
        }
    }

    pub fn color(id: impl Into<String>, label: impl Into<String>, val: Color) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "Appearance".into(),
            value: PropertyValue::Color(val),
            tooltip: None,
        }
    }

    pub fn enum_choice(
        id: impl Into<String>,
        label: impl Into<String>,
        selected: usize,
        options: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "General".into(),
            value: PropertyValue::Enum { selected, options },
            tooltip: None,
        }
    }

    pub fn vec2(
        id: impl Into<String>,
        label: impl Into<String>,
        x: f32,
        y: f32,
        min: f32,
        max: f32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "Transform".into(),
            value: PropertyValue::Vec2 {
                x,
                y,
                min,
                max,
                step: 1.0,
            },
            tooltip: None,
        }
    }

    pub fn integer(
        id: impl Into<String>,
        label: impl Into<String>,
        val: i32,
        min: i32,
        max: i32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "General".into(),
            value: PropertyValue::Int {
                value: val,
                min,
                max,
            },
            tooltip: None,
        }
    }

    pub fn text(id: impl Into<String>, label: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: "General".into(),
            value: PropertyValue::Text(text.into()),
            tooltip: None,
        }
    }

    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.category = cat.into();
        self
    }
}

/// Event emitted when a property changes value.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyChangedEvent {
    pub id: String,
    pub new_value: PropertyValue,
}

/// Dynamic, metadata-driven PropertyInspector.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetadataPropertyInspector {
    pub title: String,
    pub properties: Vec<PropertyItem>,
    pub active_scrubber_id: Option<(String, bool)>, // (prop_id, is_y_coord)
    pub color_wheel: Option<ColorWheel>,
    pub active_color_id: Option<String>,
}

impl MetadataPropertyInspector {
    pub fn new(title: impl Into<String>, properties: Vec<PropertyItem>) -> Self {
        Self {
            title: title.into(),
            properties,
            active_scrubber_id: None,
            color_wheel: None,
            active_color_id: None,
        }
    }

    pub fn set_properties(&mut self, title: impl Into<String>, props: Vec<PropertyItem>) {
        self.title = title.into();
        self.properties = props;
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats = Vec::new();
        for p in &self.properties {
            if !cats.contains(&p.category) {
                cats.push(p.category.clone());
            }
        }
        cats
    }

    pub fn get_float(&self, id: &str) -> Option<f32> {
        self.properties
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| match p.value {
                PropertyValue::Float { value, .. } => Some(value),
                _ => None,
            })
    }

    pub fn get_bool(&self, id: &str) -> Option<bool> {
        self.properties
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| match p.value {
                PropertyValue::Bool(b) => Some(b),
                _ => None,
            })
    }

    pub fn get_color(&self, id: &str) -> Option<Color> {
        self.properties
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| match p.value {
                PropertyValue::Color(c) => Some(c),
                _ => None,
            })
    }

    /// Handles pointer mouse down.
    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        viewport: Rect,
    ) -> Option<PropertyChangedEvent> {
        if !viewport.contains(pos) || button != PointerButton::Primary {
            return None;
        }

        let mut y = viewport.min.y + 42.0;
        let row_h = 32.0;

        for prop in &mut self.properties {
            let row_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + 8.0, y),
                Vec2::new(viewport.width() - 16.0, row_h),
            );
            let val_rect = Rect::from_min_size(
                Vec2::new(row_rect.min.x + 100.0, row_rect.min.y + 2.0),
                Vec2::new(row_rect.width() - 104.0, row_h - 4.0),
            );

            if val_rect.contains(pos) {
                match &mut prop.value {
                    PropertyValue::Bool(b) => {
                        *b = !*b;
                        return Some(PropertyChangedEvent {
                            id: prop.id.clone(),
                            new_value: PropertyValue::Bool(*b),
                        });
                    }
                    PropertyValue::Float { .. } => {
                        self.active_scrubber_id = Some((prop.id.clone(), false));
                        return None;
                    }
                    PropertyValue::Vec2 { .. } => {
                        let is_y = pos.x > val_rect.min.x + val_rect.width() * 0.5;
                        self.active_scrubber_id = Some((prop.id.clone(), is_y));
                        return None;
                    }
                    PropertyValue::Enum { selected, options } => {
                        *selected = (*selected + 1) % options.len().max(1);
                        return Some(PropertyChangedEvent {
                            id: prop.id.clone(),
                            new_value: PropertyValue::Enum {
                                selected: *selected,
                                options: options.clone(),
                            },
                        });
                    }
                    PropertyValue::Color(_c) => {
                        if self.active_color_id.as_deref() == Some(&prop.id) {
                            self.active_color_id = None;
                            self.color_wheel = None;
                        } else {
                            self.active_color_id = Some(prop.id.clone());
                            let wheel = ColorWheel::new(48.0);
                            self.color_wheel = Some(wheel);
                        }
                        return None;
                    }
                    _ => {}
                }
            }
            y += row_h + 4.0;
        }

        None
    }

    /// Handles pointer dragging for numeric scrubbers.
    pub fn handle_mouse_move(
        &mut self,
        _pos: Vec2,
        delta: Vec2,
        ptr: &PointerState,
    ) -> Option<PropertyChangedEvent> {
        if !ptr.primary_down {
            self.active_scrubber_id = None;
            return None;
        }

        if let Some((ref prop_id, is_y)) = self.active_scrubber_id.clone() {
            if let Some(prop) = self.properties.iter_mut().find(|p| p.id == *prop_id) {
                match &mut prop.value {
                    PropertyValue::Float {
                        value,
                        min,
                        max,
                        step,
                        ..
                    } => {
                        let change = delta.x * *step;
                        *value = (*value + change).clamp(*min, *max);
                        return Some(PropertyChangedEvent {
                            id: prop.id.clone(),
                            new_value: prop.value.clone(),
                        });
                    }
                    PropertyValue::Vec2 {
                        x,
                        y,
                        min,
                        max,
                        step,
                    } => {
                        let change = delta.x * *step;
                        if is_y {
                            *y = (*y + change).clamp(*min, *max);
                        } else {
                            *x = (*x + change).clamp(*min, *max);
                        }
                        return Some(PropertyChangedEvent {
                            id: prop.id.clone(),
                            new_value: prop.value.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// Handles mouse release.
    pub fn handle_mouse_up(&mut self, _pos: Vec2, _button: PointerButton) {
        self.active_scrubber_id = None;
    }

    /// Renders the property sheet into the command buffer.
    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Background
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // Header
        let header_h = 34.0;
        let header_rect = Rect::from_min_size(viewport.min, Vec2::new(viewport.width(), header_h));
        buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        buf.push(DrawCommand::Text {
            text: self.title.to_uppercase(),
            pos: Vec2::new(header_rect.min.x + 10.0, header_rect.min.y + 10.0),
            size: 11.5,
            color: palette.text,
            strong: true,
        });

        // Properties list
        let mut y = viewport.min.y + header_h + 8.0;
        let row_h = 28.0;

        for prop in &self.properties {
            let row_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + 8.0, y),
                Vec2::new(viewport.width() - 16.0, row_h),
            );
            let val_rect = Rect::from_min_size(
                Vec2::new(row_rect.min.x + 95.0, row_rect.min.y),
                Vec2::new(row_rect.width() - 95.0, row_h),
            );

            // Property Label
            buf.push(DrawCommand::Text {
                text: ui_render::fit_text(
                    &prop.label,
                    11.5,
                    false,
                    val_rect.min.x - row_rect.min.x - 10.0,
                ),
                pos: Vec2::new(row_rect.min.x + 4.0, row_rect.min.y + 7.0),
                size: 11.5,
                color: palette.text_muted,
                strong: false,
            });

            // Property Value Editor
            match &prop.value {
                PropertyValue::Float { value, unit, .. } => {
                    let is_active = self
                        .active_scrubber_id
                        .as_ref()
                        .is_some_and(|(id, _)| id == &prop.id);
                    buf.push(DrawCommand::Rect {
                        rect: val_rect,
                        fill: palette.canvas,
                        radius: CornerRadius::same(4.0),
                        stroke: if is_active {
                            Stroke::new(1.5, palette.accent)
                        } else {
                            Stroke::new(1.0, palette.border)
                        },
                    });
                    buf.push(DrawCommand::Text {
                        text: format!("{:.2}{}", value, unit),
                        pos: Vec2::new(val_rect.min.x + 8.0, val_rect.min.y + 7.0),
                        size: 11.5,
                        color: palette.text,
                        strong: true,
                    });
                }
                PropertyValue::Bool(b) => {
                    let check_box = Rect::from_min_size(
                        Vec2::new(val_rect.min.x, val_rect.min.y + 4.0),
                        Vec2::new(20.0, 20.0),
                    );
                    buf.push(DrawCommand::Rect {
                        rect: check_box,
                        fill: if *b { palette.accent } else { palette.canvas },
                        radius: CornerRadius::same(4.0),
                        stroke: Stroke::new(1.0, palette.border),
                    });
                    if *b {
                        buf.push(DrawCommand::Text {
                            text: "✓".into(),
                            pos: Vec2::new(check_box.min.x + 5.0, check_box.min.y + 3.0),
                            size: 12.0,
                            color: Color::WHITE,
                            strong: true,
                        });
                    }
                }
                PropertyValue::Color(c) => {
                    let swatch_rect = Rect::from_min_size(
                        Vec2::new(val_rect.min.x, val_rect.min.y + 2.0),
                        Vec2::new(row_h - 4.0, row_h - 4.0),
                    );
                    buf.push(DrawCommand::Rect {
                        rect: swatch_rect,
                        fill: *c,
                        radius: CornerRadius::same(4.0),
                        stroke: Stroke::new(1.0, palette.border),
                    });
                    buf.push(DrawCommand::Text {
                        text: format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b),
                        pos: Vec2::new(swatch_rect.max.x + 8.0, val_rect.min.y + 7.0),
                        size: 11.5,
                        color: palette.text,
                        strong: false,
                    });
                }
                PropertyValue::Enum { selected, options } => {
                    buf.push(DrawCommand::Rect {
                        rect: val_rect,
                        fill: palette.canvas,
                        radius: CornerRadius::same(4.0),
                        stroke: Stroke::new(1.0, palette.border),
                    });
                    let opt_name = options.get(*selected).map_or("", |s| s.as_str());
                    buf.push(DrawCommand::Text {
                        text: format!("{} ▾", opt_name),
                        pos: Vec2::new(val_rect.min.x + 8.0, val_rect.min.y + 7.0),
                        size: 11.5,
                        color: palette.text,
                        strong: false,
                    });
                }
                PropertyValue::Vec2 { x, y, .. } => {
                    let half_w = (val_rect.width() - 4.0) * 0.5;
                    let x_rect =
                        Rect::from_min_size(val_rect.min, Vec2::new(half_w, val_rect.height()));
                    let y_rect = Rect::from_min_size(
                        Vec2::new(val_rect.min.x + half_w + 4.0, val_rect.min.y),
                        Vec2::new(half_w, val_rect.height()),
                    );

                    buf.push(DrawCommand::Rect {
                        rect: x_rect,
                        fill: palette.canvas,
                        radius: CornerRadius::same(4.0),
                        stroke: Stroke::new(1.0, palette.border),
                    });
                    buf.push(DrawCommand::Text {
                        text: format!("X: {:.1}", x),
                        pos: Vec2::new(x_rect.min.x + 6.0, x_rect.min.y + 7.0),
                        size: 11.0,
                        color: palette.text,
                        strong: false,
                    });

                    buf.push(DrawCommand::Rect {
                        rect: y_rect,
                        fill: palette.canvas,
                        radius: CornerRadius::same(4.0),
                        stroke: Stroke::new(1.0, palette.border),
                    });
                    buf.push(DrawCommand::Text {
                        text: format!("Y: {:.1}", y),
                        pos: Vec2::new(y_rect.min.x + 6.0, y_rect.min.y + 7.0),
                        size: 11.0,
                        color: palette.text,
                        strong: false,
                    });
                }
                PropertyValue::Text(txt) => {
                    buf.push(DrawCommand::Rect {
                        rect: val_rect,
                        fill: palette.canvas,
                        radius: CornerRadius::same(4.0),
                        stroke: Stroke::new(1.0, palette.border),
                    });
                    buf.push(DrawCommand::Text {
                        text: txt.clone(),
                        pos: Vec2::new(val_rect.min.x + 8.0, val_rect.min.y + 7.0),
                        size: 11.5,
                        color: palette.text,
                        strong: false,
                    });
                }
                _ => {}
            }

            y += row_h + 6.0;
        }

        buf.push(DrawCommand::ClipPop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_themes::ThemePalette;

    #[test]
    fn test_property_inspector_metadata_items() {
        let mut inspector = MetadataPropertyInspector::new("Transform", Vec::new());
        let props = vec![
            PropertyItem::float("scale", "Scale", 1.0, 0.1, 10.0, 0.1).with_category("Transform"),
            PropertyItem::integer("seed", "Seed", 42, 0, 9999).with_category("Generation"),
            PropertyItem::boolean("hdr", "HDR Output", true).with_category("Render"),
            PropertyItem::color("tint", "Tint Color", Color::WHITE).with_category("Appearance"),
            PropertyItem::vec2("offset", "Offset", 10.0, 20.0, -100.0, 100.0)
                .with_category("Transform"),
            PropertyItem::text("name", "Layer Name", "Background").with_category("General"),
        ];

        inspector.set_properties("Layer Properties", props);
        assert_eq!(inspector.title, "Layer Properties");
        assert_eq!(inspector.properties.len(), 6);
        assert_eq!(inspector.categories().len(), 5);
    }

    #[test]
    fn test_scrubber_drag_modifies_float() {
        let mut inspector = MetadataPropertyInspector::new("Test", Vec::new());
        inspector.set_properties(
            "Test",
            vec![PropertyItem::float(
                "roughness",
                "Roughness",
                0.5,
                0.0,
                1.0,
                0.05,
            )],
        );

        let mut ptr = PointerState {
            position: Vec2::new(180.0, 50.0),
            ..Default::default()
        };
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 400.0));

        // Click to start scrub
        let _event =
            inspector.handle_mouse_down(Vec2::new(180.0, 50.0), PointerButton::Primary, viewport);
        assert!(inspector.active_scrubber_id.is_some());
        assert_eq!(
            inspector.active_scrubber_id.as_ref().unwrap().0,
            "roughness"
        );

        // Drag right with primary down
        ptr.primary_down = true;
        let change_event =
            inspector.handle_mouse_move(Vec2::new(190.0, 50.0), Vec2::new(2.0, 0.0), &ptr);
        assert!(change_event.is_some());
        let ev = change_event.unwrap();
        assert_eq!(ev.id, "roughness");
        if let PropertyValue::Float { value, .. } = ev.new_value {
            assert!((value - 0.6).abs() < 1e-4);
        } else {
            panic!("Expected float value");
        }
    }

    #[test]
    fn test_property_inspector_rendering_emits_commands() {
        let mut inspector = MetadataPropertyInspector::new("Inspect", Vec::new());
        inspector.set_properties(
            "Inspect",
            vec![
                PropertyItem::boolean("active", "Active", true),
                PropertyItem::color("base", "Base Color", Color::rgb(255, 0, 0)),
            ],
        );

        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 400.0));

        inspector.render(&mut buf, viewport, &palette);
        assert!(!buf.commands().is_empty());
    }
}
