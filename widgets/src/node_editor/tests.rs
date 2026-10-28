use super::editor::NodeEditor;
use super::types::{category_color, NodeAlignment, PresetNode, ToolbarAction};
use crate::theme::ThemePalette;
use ui_core::{Rect, Vec2};
use ui_platform::{Key, PointerButton};

#[test]
fn offscreen_nodes_do_not_generate_gpu_draw_commands() {
    use super::render::render_nodes;
    use ui_render::{CommandBuffer, DrawCommand};
    let editor = NodeEditor::default();
    let original = editor.nodes[0].clone();
    let visible = original.clone();
    let mut invisible = original.clone();
    invisible.id = "offscreen".into();
    invisible.title = "Offscreen Marker".into();
    invisible.pos = Vec2::new(40_000.0, 40_000.0);
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1920.0, 1080.0));
    let mut buffer = CommandBuffer::new();
    render_nodes(
        &mut buffer, &[visible, invisible], &[], Vec2::ZERO, 1.0,
        viewport.min, viewport, &ThemePalette::dark(),
    );
    assert!(!buffer.commands().iter().any(|cmd| matches!(
        cmd, DrawCommand::Text { text, .. } if text == "Offscreen Marker"
    )));
}

#[test]
fn graph_224_nodes_at_1080p_has_bounded_draw_commands() {
    use super::render::render_nodes;
    use ui_render::CommandBuffer;
    let editor = NodeEditor::default();
    let template = editor.nodes[0].clone();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1920.0, 1080.0));
    let mut nodes = Vec::with_capacity(224);
    for i in 0..224 {
        let mut node = template.clone();
        node.id = format!("stress_{i}");
        node.pos = Vec2::new((i % 16) as f32 * 230.0, (i / 16) as f32 * 160.0);
        nodes.push(node);
    }
    let mut buffer = CommandBuffer::new();
    let start = std::time::Instant::now();
    render_nodes(
        &mut buffer, &nodes, &[], Vec2::ZERO, 0.30,
        viewport.min, viewport, &ThemePalette::dark(),
    );
    let elapsed = start.elapsed();
    assert!(buffer.len() > 224, "No visible node drawing");
    assert!(buffer.len() < 10_000, "Command buffer grew unexpectedly");
    println!("224 visible graph nodes at 1920x1080: {} draw commands, CPU encode {:?}", buffer.len(), elapsed);
}

#[test]
fn test_node_editor_defaults() {
    let editor = NodeEditor::default();
    assert_eq!(editor.nodes.len(), 5);
    assert_eq!(editor.connections.len(), 4);
    assert!(editor.selected_node.is_none());
}

#[test]
fn test_topological_execution_order() {
    let editor = NodeEditor::default();
    let order = editor.execution_order();
    assert_eq!(order.len(), 5);
    let idx_source = order.iter().position(|id| id == "node_source").unwrap();
    let idx_stylize = order.iter().position(|id| id == "node_stylize").unwrap();
    let idx_seamless = order.iter().position(|id| id == "node_seamless").unwrap();
    let idx_pbr = order.iter().position(|id| id == "node_pbr").unwrap();
    let idx_video = order.iter().position(|id| id == "node_presentation").unwrap();

    assert!(idx_source < idx_stylize);
    assert!(idx_stylize < idx_seamless);
    assert!(idx_seamless < idx_pbr);
    assert!(idx_pbr < idx_video);

    let manifest = editor.export_pipeline_manifest();
    assert!(manifest.contains("Rupayan Flow / Sutro Visual Pipeline Manifest"));
    assert!(manifest.contains("Execution Schedule:"));
}

#[test]
fn test_node_dragging() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));

    let hit = editor.hit_test_node(Vec2::new(100.0, 140.0));
    assert_eq!(hit, Some("node_source".into()));

    let screen_click = editor.world_to_screen(Vec2::new(100.0, 140.0), viewport.min);
    editor.handle_mouse_down(screen_click, PointerButton::Primary, viewport);
    assert_eq!(editor.selected_node, Some("node_source".into()));
    assert!(editor.dragging_node.is_some());

    let screen_move = screen_click + Vec2::new(50.0, 30.0);
    editor.handle_mouse_move(screen_move, Vec2::new(50.0, 30.0), viewport);
    editor.handle_mouse_up(screen_move, PointerButton::Primary, viewport);

    assert!(editor.dragging_node.is_none());
    let source_node = editor.nodes.iter().find(|n| n.id == "node_source").unwrap();
    assert_eq!(source_node.pos, Vec2::new(110.0, 110.0));
}

#[test]
fn test_pin_wiring_and_connection() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));

    editor.connections.clear();
    assert_eq!(editor.connections.len(), 0);

    let from_node = editor.nodes.iter().find(|n| n.id == "node_source").unwrap();
    let out_pos = editor.output_pin_world_pos(from_node, 0);

    let pin_hit = editor.hit_test_pin(out_pos);
    assert_eq!(
        pin_hit,
        Some(("node_source".into(), "out_rgb".into(), true))
    );

    let screen_start = editor.world_to_screen(out_pos, viewport.min);
    editor.handle_mouse_down(screen_start, PointerButton::Primary, viewport);
    assert!(editor.pending_wire.is_some());

    let to_node = editor.nodes.iter().find(|n| n.id == "node_stylize").unwrap();
    let in_pos = editor.input_pin_world_pos(to_node, 0);
    let screen_end = editor.world_to_screen(in_pos, viewport.min);

    editor.handle_mouse_move(screen_end, Vec2::ZERO, viewport);
    editor.handle_mouse_up(screen_end, PointerButton::Primary, viewport);

    assert!(editor.pending_wire.is_none());
    assert_eq!(editor.connections.len(), 1);
    assert_eq!(editor.connections[0].from_node, "node_source");
    assert_eq!(editor.connections[0].to_node, "node_stylize");
}

#[test]
fn test_type_safety_rejects_mismatched_pins() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));
    editor.connections.clear();

    let from_node = editor.nodes.iter().find(|n| n.id == "node_source").unwrap();
    let out_pos = editor.output_pin_world_pos(from_node, 1); // "Mask" type

    let to_node = editor.nodes.iter().find(|n| n.id == "node_stylize").unwrap();
    let in_pos = editor.input_pin_world_pos(to_node, 0); // "Image" type

    let screen_start = editor.world_to_screen(out_pos, viewport.min);
    editor.handle_mouse_down(screen_start, PointerButton::Primary, viewport);

    let screen_end = editor.world_to_screen(in_pos, viewport.min);
    editor.handle_mouse_move(screen_end, Vec2::ZERO, viewport);
    editor.handle_mouse_up(screen_end, PointerButton::Primary, viewport);

    assert_eq!(editor.connections.len(), 0);
}

#[test]
fn test_spawn_preset_and_toolbar() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));
    let initial_count = editor.nodes.len();

    let t_rects = NodeEditor::toolbar_rects(viewport);
    assert!(t_rects.len() >= 8);

    let (_, rect, _): &(ToolbarAction, Rect, &'static str) = t_rects
        .iter()
        .find(|(act, _, _)| *act == ToolbarAction::AddTexture)
        .unwrap();
    let click_pos = rect.center();

    editor.handle_mouse_down(click_pos, PointerButton::Primary, viewport);
    assert_eq!(editor.nodes.len(), initial_count + 1);
}

#[test]
fn test_delete_selected_node_cleans_connections() {
    let mut editor = NodeEditor::default();
    editor.select_node("node_seamless", false);
    assert_eq!(editor.selected_node, Some("node_seamless".into()));

    editor.handle_key_down(Key::Delete);
    assert_eq!(editor.nodes.len(), 4);
    assert!(editor.nodes.iter().all(|n| n.id != "node_seamless"));
    assert_eq!(editor.connections.len(), 2);
    assert_eq!(editor.connections[0].from_node, "node_source");
    assert_eq!(editor.connections[0].to_node, "node_stylize");
    assert_eq!(editor.connections[1].from_node, "node_pbr");
    assert_eq!(editor.connections[1].to_node, "node_presentation");
}

#[test]
fn test_zoom_and_pan() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));

    let initial_pan = editor.pan;
    editor.handle_mouse_move_with_buttons(
        Vec2::new(200.0, 200.0),
        Vec2::new(20.0, -10.0),
        viewport,
        true,
    );
    assert_ne!(editor.pan, initial_pan);

    let initial_zoom = editor.zoom;
    editor.handle_scroll(Vec2::new(0.0, 1.0), Vec2::new(400.0, 300.0), viewport);
    assert!(editor.zoom > initial_zoom);
}

#[test]
fn test_category_header_colors() {
    let palette = ThemePalette::dark();
    let source_col = category_color("Source", &palette);
    let proc_col = category_color("Processing", &palette);
    let mat_col = category_color("Material", &palette);
    let out_col = category_color("Output", &palette);
    let util_col = category_color("Utility", &palette);

    assert_ne!(source_col, proc_col);
    assert_ne!(proc_col, mat_col);
    assert_ne!(mat_col, out_col);
    assert_ne!(out_col, util_col);
}

#[test]
fn test_marquee_box_selection() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));

    editor.clear_selection();
    assert_eq!(editor.selected_nodes.len(), 0);

    let start_screen = editor.world_to_screen(Vec2::new(50.0, 80.0), viewport.min);
    editor.handle_mouse_down(start_screen, PointerButton::Primary, viewport);
    assert!(editor.marquee_start.is_some());

    let move_screen = editor.world_to_screen(Vec2::new(600.0, 300.0), viewport.min);
    editor.handle_mouse_move(move_screen, Vec2::ZERO, viewport);
    editor.handle_mouse_up(move_screen, PointerButton::Primary, viewport);

    assert!(editor.marquee_start.is_none());
    assert!(editor.is_node_selected("node_source"));
    assert!(editor.is_node_selected("node_stylize"));
}

#[test]
fn test_multi_node_dragging() {
    let mut editor = NodeEditor::default();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 800.0));

    editor.select_node("node_source", false);
    editor.select_node("node_stylize", true);
    assert_eq!(editor.selected_nodes.len(), 2);

    let src_pos_before = editor
        .nodes
        .iter()
        .find(|n| n.id == "node_source")
        .unwrap()
        .pos;
    let stylize_pos_before = editor
        .nodes
        .iter()
        .find(|n| n.id == "node_stylize")
        .unwrap()
        .pos;

    let click_world = src_pos_before + Vec2::new(20.0, 20.0);
    let click_screen = editor.world_to_screen(click_world, viewport.min);
    editor.handle_mouse_down_with_modifiers(click_screen, PointerButton::Primary, viewport, true);

    let move_screen = click_screen + Vec2::new(40.0, 30.0);
    editor.handle_mouse_move(move_screen, Vec2::new(40.0, 30.0), viewport);
    editor.handle_mouse_up(move_screen, PointerButton::Primary, viewport);

    let src_pos_after = editor
        .nodes
        .iter()
        .find(|n| n.id == "node_source")
        .unwrap()
        .pos;
    let stylize_pos_after = editor
        .nodes
        .iter()
        .find(|n| n.id == "node_stylize")
        .unwrap()
        .pos;

    assert_eq!(src_pos_after - src_pos_before, Vec2::new(40.0, 30.0));
    assert_eq!(stylize_pos_after - stylize_pos_before, Vec2::new(40.0, 30.0));
}

#[test]
fn test_node_alignment() {
    let mut editor = NodeEditor::default();
    editor.select_node("node_source", false);
    editor.select_node("node_stylize", true);

    editor.align_selected(NodeAlignment::Left);
    let src_x = editor
        .nodes
        .iter()
        .find(|n| n.id == "node_source")
        .unwrap()
        .pos
        .x;
    let stylize_x = editor
        .nodes
        .iter()
        .find(|n| n.id == "node_stylize")
        .unwrap()
        .pos
        .x;
    assert_eq!(src_x, stylize_x);
}

#[test]
fn test_search_to_add_filter_and_spawn() {
    let mut editor = NodeEditor::default();
    editor.open_search_menu(Vec2::new(200.0, 200.0));
    assert!(editor.search_state.is_open);

    editor.search_state.query = "pbr".to_string();
    let filtered = editor.filter_presets();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0], PresetNode::PbrGenerator);

    let spawned = editor.handle_key_down(Key::Enter);
    assert!(spawned);
    assert!(!editor.search_state.is_open);
    assert!(editor.nodes.iter().any(|n| n.title.contains("PBR")));
}

#[test]
fn test_reroute_knot() {
    let mut editor = NodeEditor::default();
    let id = editor.spawn_preset(PresetNode::Reroute, Vec2::new(200.0, 200.0));
    let node = editor.nodes.iter().find(|n| n.id == id).unwrap();
    assert!(node.is_reroute());
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.inputs[0].pin_type, "Any");
    assert_eq!(node.outputs[0].pin_type, "Any");
}
