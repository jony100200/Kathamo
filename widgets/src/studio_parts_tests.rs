use super::*;
use crate::studio::{Studio, Tone};
use ui_core::{Rect, Vec2};
use ui_platform::PointerState;
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::{ExtendedTokens, ThemeMode, ThemePalette};

fn make_studio<'a>(buf: &'a mut CommandBuffer, pointer: &'a PointerState) -> Studio<'a> {
    Studio::new(
        buf,
        ThemePalette::midnight_graphite(),
        ExtendedTokens::for_mode(ThemeMode::MidnightGraphite),
        pointer,
    )
}

fn click(x: f32, y: f32) -> PointerState {
    PointerState {
        position: Vec2::new(x, y),
        primary_clicked: true,
        ..Default::default()
    }
}

#[test]
fn test_chip() {
    let mut buf = CommandBuffer::default();
    let ptr = PointerState::default();
    let mut s = make_studio(&mut buf, &ptr);
    let w = chip(&mut s, Vec2::new(10.0, 10.0), "7B-Instruct", Tone::Neutral);
    assert!(w > 20.0);
    assert!(buf.len() >= 2);
}

#[test]
fn test_search_field() {
    let mut buf = CommandBuffer::default();
    let ptr1 = click(20.0, 20.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 36.0));
    assert!(search_field(&mut s1, rect, "cargo test", "Search...", true).clicked);
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    assert!(!search_field(&mut s2, rect, "a".repeat(100).as_str(), "Search...", false).clicked);
    assert!(buf2
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text.ends_with('…'))));
}

#[test]
fn test_segmented() {
    let mut buf = CommandBuffer::default();
    let ptr1 = click(100.0, 15.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 32.0));
    assert_eq!(
        segmented(
            &mut s1,
            rect,
            &["Models", "Agents", "Long-tab-label-exceeding-space"],
            0
        ),
        Some(1)
    );
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    assert_eq!(segmented(&mut s2, rect, &["One", "Two"], 0), None);
}

#[test]
fn test_channel_row() {
    let mut buf = CommandBuffer::default();
    let ptr1 = click(50.0, 20.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(220.0, 36.0));
    assert!(channel_row(&mut s1, rect, "general-discussion", 5, true).clicked);
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    let miss = channel_row(
        &mut s2,
        rect,
        "channel-with-a-very-long-name-that-must-truncate",
        120,
        false,
    );
    assert!(!miss.clicked);
    assert!(buf2
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "99+")));
}

#[test]
fn test_task_card() {
    let mut buf = CommandBuffer::default();
    let ptr1 = click(50.0, 50.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(260.0, 110.0));
    assert!(
        task_card(
            &mut s1,
            rect,
            "Refactor shader pipeline",
            "Jony",
            "In Progress",
            Tone::Working,
            "P0 • 2h ago"
        )
        .clicked
    );
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    task_card(
        &mut s2,
        rect,
        &"Very long task title ".repeat(5),
        "Assignee Name",
        "Todo",
        Tone::Neutral,
        "P1 • yesterday",
    );
    assert!(buf2
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text.ends_with('…'))));
}

#[test]
fn test_agent_card() {
    let mut buf = CommandBuffer::default();
    let ptr1 = click(40.0, 40.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(280.0, 90.0));
    assert!(
        agent_card(
            &mut s1,
            rect,
            "Codebase Researcher",
            "OWN",
            "Ready",
            Tone::Good,
            "Analyzes ast and graph"
        )
        .clicked
    );
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    agent_card(
        &mut s2,
        rect,
        "Agent With Long Identifier",
        "LOCAL",
        "Busy",
        Tone::Working,
        &"Extremely long detail paragraph ".repeat(4),
    );
    assert!(buf2
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text.ends_with('…'))));
}

#[test]
fn test_model_card() {
    let mut buf = CommandBuffer::default();
    let ptr1 = click(50.0, 50.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 110.0));
    assert!(
        model_card(
            &mut s1,
            rect,
            "DeepSeek-Coder",
            "DeepSeek",
            "33B",
            "Q4_K_M",
            "Loaded",
            Tone::Good,
            true
        )
        .clicked
    );
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    model_card(
        &mut s2,
        rect,
        "Mistral-Large-Instruct-2407-Extended-Long-Context-Model",
        "Mistral AI",
        "123B",
        "FP16",
        "Off",
        Tone::Neutral,
        false,
    );
    assert!(buf2
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text.ends_with('…'))));
}

#[test]
fn test_composer() {
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(320.0, 100.0));
    let mut buf = CommandBuffer::default();
    let ptr_send = click(rect.max.x - 30.0, rect.max.y - 25.0);
    let mut s1 = make_studio(&mut buf, &ptr_send);
    let out_send = composer(
        &mut s1,
        rect,
        "Hello model",
        "Type a message...",
        true,
        false,
    );
    assert!(out_send.send && !out_send.stop);
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let mut s2 = make_studio(&mut buf2, &ptr_send);
    let out_stop = composer(&mut s2, rect, "Working...", "Type...", false, true);
    assert!(out_stop.stop && !out_stop.send);

    let ptr_attach = click(rect.min.x + 20.0, rect.max.y - 25.0);
    let mut buf3 = CommandBuffer::default();
    let mut s3 = make_studio(&mut buf3, &ptr_attach);
    assert!(composer(&mut s3, rect, "", "Placeholder", false, false).attach);
}

#[test]
fn test_code_block() {
    let mut buf = CommandBuffer::default();
    let lines = vec![
        "fn main() {".into(),
        "    println!(\"hello\");".into(),
        "}".into(),
    ];
    let ptr1 = click(300.0 - 25.0, 15.0);
    let mut s1 = make_studio(&mut buf, &ptr1);
    let (height, copy_hit) = code_block(&mut s1, 0.0, 0.0, 300.0, "rust", &lines);
    assert!(height > 60.0 && copy_hit.clicked);
    assert!(buf.len() > 0);

    let mut buf2 = CommandBuffer::default();
    let ptr2 = PointerState::default();
    let mut s2 = make_studio(&mut buf2, &ptr2);
    let (_, miss_copy) = code_block(
        &mut s2,
        0.0,
        0.0,
        200.0,
        "python-with-very-long-language-tag",
        &["x = ".to_string() + &"1".repeat(200)],
    );
    assert!(!miss_copy.clicked);
    assert!(buf2
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text.ends_with('…'))));
}

#[test]
fn test_message_block() {
    let mut buf_user = CommandBuffer::default();
    let ptr = PointerState::default();
    let mut s_user = make_studio(&mut buf_user, &ptr);
    let h_user = message_block(
        &mut s_user,
        0.0,
        0.0,
        400.0,
        "User",
        MessageRole::User,
        "10:45 AM",
        "Please analyze the memory layout",
        false,
    );
    assert!(h_user > 40.0 && buf_user.len() > 0);

    let mut buf_model = CommandBuffer::default();
    let mut s_model = make_studio(&mut buf_model, &ptr);
    let h_model = message_block(
        &mut s_model,
        0.0,
        0.0,
        400.0,
        "Jogot Model",
        MessageRole::Model,
        "10:46 AM",
        "Pipeline buffer structure",
        true,
    );
    assert!(h_model > 40.0);
    let dots = buf_model
        .commands()
        .iter()
        .filter(|c| matches!(c, DrawCommand::Rect { radius, .. } if radius.top_left == 2.0))
        .count();
    assert!(dots >= 3);
}
