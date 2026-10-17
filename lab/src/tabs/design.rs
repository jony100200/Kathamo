//! Design System, KolponaUi controls, bilingual typography, and Tabler vector icons showcase.

use crate::app::UiLabApp;
use ui_core::*;
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::*;

pub fn render_design_system_tab(
    app: &mut UiLabApp,
    cmd_buf: &mut CommandBuffer,
    content_rect: Rect,
    p: &ThemePalette,
) {
    cmd_buf.push(DrawCommand::Rect {
        rect: content_rect,
        fill: p.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });

    let margin = 20.0;
    let inner_width = content_rect.width() - margin * 4.0;
    let col_w = (inner_width / 3.0).floor();
    let col_h = content_rect.height() - margin * 2.0;

    // -------------------------------------------------------------
    // Column 1: Unified Studio UI API (KolponaUi)
    // -------------------------------------------------------------
    let col1_rect = Rect::from_min_size(
        Vec2::new(content_rect.min.x + margin, content_rect.min.y + margin),
        Vec2::new(col_w, col_h),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: col1_rect,
        fill: p.surface,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, p.border),
    });

    let pointer = app.pointer;
    let mut ui = KolponaUi::new(cmd_buf, &mut app.icon_atlas, p, col1_rect, pointer);
    ui.heading("1. STUDIO UI API (KolponaUi)");
    ui.label("Applications use Kolpona's unified controls:");

    ui.panel("Interactive Studio Actions", |ui| {
        ui.button("Generate (Chobi AI)");
        ui.button_secondary("Preview 3D Mesh");
        ui.button_toolbar(StudioIcon::Play);
        ui.checkbox("Snap to grid", &mut app.checkbox_snap);
        let options = ["Chobi 2D", "Jogot Realtime", "Rupayon 3D", "Drisho GenAI"];
        ui.dropdown("Workspace", &mut app.dropdown_selected, &options);
        ui.label("Export Progress:");
        ui.progress(0.72);
        ui.scrubber("Exposure", &mut app.scrubber_exposure, -5.0, 5.0);
        ui.badge("Verified Commercial Gate", true);
    });

    // -------------------------------------------------------------
    // Column 2: Bilingual Typography (Fluent 2 Scale)
    // -------------------------------------------------------------
    let col2_rect = Rect::from_min_size(
        Vec2::new(col1_rect.max.x + margin, content_rect.min.y + margin),
        Vec2::new(col_w, col_h),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: col2_rect,
        fill: p.surface,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, p.border),
    });

    let mut ty = col2_rect.min.y + 16.0;
    let tx = col2_rect.min.x + 16.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "2. FLUENT 2 TYPOGRAPHY & BENGALI".into(),
        size: 16.0,
        color: p.accent,
        strong: true,
    });
    ty += 32.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Hero (36px): Kolpona (কল্পনা)".into(),
        size: 36.0,
        color: p.text,
        strong: true,
    });
    ty += 48.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Workspace (28px): Studio Operating Core".into(),
        size: 28.0,
        color: p.text,
        strong: true,
    });
    ty += 40.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Panel Title (20px): Chobi (ছবি) • Rupayon (রূপায়ণ)".into(),
        size: 20.0,
        color: p.text,
        strong: true,
    });
    ty += 32.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Section (16px): Dolil (দলিল) • Pothik (পথিক) • Jogot (জগত)".into(),
        size: 16.0,
        color: p.accent_hover,
        strong: true,
    });
    ty += 28.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Body (14px): Commercial-grade Rust UI framework with zero flicker.".into(),
        size: 14.0,
        color: p.text,
        strong: false,
    });
    ty += 32.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Monospace (12px): Coordinates: X=1440.00 Y=900.00 | Frame: 0.43ms".into(),
        size: 12.0,
        color: p.warning,
        strong: false,
    });
    ty += 24.0;

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(tx, ty),
        text: "Caption (12px): OFL Fonts: Inter, JetBrains Mono, Noto Sans Bengali".into(),
        size: 12.0,
        color: p.text_muted,
        strong: false,
    });

    // -------------------------------------------------------------
    // Column 3: Tabler Curated Vector Icon Atlas
    // -------------------------------------------------------------
    let col3_rect = Rect::from_min_size(
        Vec2::new(col2_rect.max.x + margin, content_rect.min.y + margin),
        Vec2::new(col_w, col_h),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: col3_rect,
        fill: p.surface,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, p.border),
    });

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(col3_rect.min.x + 16.0, col3_rect.min.y + 16.0),
        text: "3. TABLER VECTOR ICONS (Zero-Alloc)".into(),
        size: 16.0,
        color: p.accent,
        strong: true,
    });
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(col3_rect.min.x + 16.0, col3_rect.min.y + 38.0),
        text: "Parsed once into cached paths · recolored by theme".into(),
        size: 12.0,
        color: p.text_muted,
        strong: false,
    });

    let icons = [
        (StudioIcon::Play, "Play"),
        (StudioIcon::Pause, "Pause"),
        (StudioIcon::Stop, "Stop"),
        (StudioIcon::Timeline, "Timeline"),
        (StudioIcon::Layers, "Layers"),
        (StudioIcon::Search, "Search"),
        (StudioIcon::Settings, "Settings"),
        (StudioIcon::Sparkles, "GenAI"),
        (StudioIcon::Bezier, "Bezier"),
        (StudioIcon::Pencil, "Pencil"),
        (StudioIcon::Pointer, "Select"),
        (StudioIcon::Save, "Save"),
        (StudioIcon::Folder, "Folder"),
        (StudioIcon::Palette, "Color"),
        (StudioIcon::Plus, "Add"),
        (StudioIcon::Check, "Confirm"),
    ];

    let mut iy = col3_rect.min.y + 65.0;
    let mut ix = col3_rect.min.x + 16.0;
    let cell_w = (col_w - 48.0) * 0.5;

    for (i, (icon, name)) in icons.iter().enumerate() {
        let card_rect = Rect::from_min_size(Vec2::new(ix, iy), Vec2::new(cell_w, 36.0));
        cmd_buf.push(DrawCommand::Rect {
            rect: card_rect,
            fill: p.elevated,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, p.border),
        });

        let icon_rect = Rect::from_min_size(
            Vec2::new(card_rect.min.x + 8.0, card_rect.min.y + 10.0),
            Vec2::new(16.0, 16.0),
        );
        app.icon_atlas
            .render(cmd_buf, *icon, icon_rect, p.accent_hover);

        cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(card_rect.min.x + 32.0, card_rect.min.y + 11.0),
            text: name.to_string(),
            size: 12.0,
            color: p.text,
            strong: false,
        });

        if i % 2 == 1 {
            ix = col3_rect.min.x + 16.0;
            iy += 42.0;
        } else {
            ix += cell_w + 12.0;
        }
    }
}
