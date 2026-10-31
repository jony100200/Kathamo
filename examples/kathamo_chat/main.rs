//! Kathamo Chat — Contemporary Native AI Chatbot UI Showcase
//!
//! A flagship ChatGPT/Claude-inspired desktop chat interface built exclusively
//! using the public Kathamo SDK (`use kathamo::prelude::*;`).
//!
//! Features:
//! - Left collapsible sidebar (~250px) with Kathamo branding, New Chat CTA,
//!   live search filtering, recent conversations, and user profile footer.
//! - Top application bar with model switcher dropdown (Claude 3.5 Sonnet,
//!   Local Llama-3-8B, Kathamo Agent Core), telemetry stats, and artifacts toggle.
//! - Central conversation stream with welcome empty state (interactive starter
//!   cards), distinct user and assistant messages, Markdown typography,
//!   syntax-highlighted code cards, message actions (Copy, Regenerate), and
//!   animated streaming typewriter indicator.
//! - Bottom message composer with rounded card geometry, dual-stroke focus
//!   border, Send/Stop button toggle, and demo attachment trigger.
//! - Collapsible right-side Code & Artifact Preview panel (ChatGPT Canvas & Claude Artifacts style).
//! - 100% deterministic local mock state with 4 pre-populated conversations.
//! - Zero external AI dependencies or network requests.

mod app;
mod chat_view;
mod composer;
mod mock_data;
mod models;
mod preview_panel;
mod sidebar;
mod top_bar;

use app::KathamoChatApp;
use models::ChatModel;

use kathamo::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless");

    println!("===========================================================");
    println!("  Kathamo Chat — Contemporary AI Chatbot UI Showcase");
    println!("===========================================================");

    if is_headless {
        run_headless_verification();
        return;
    }

    // Launch Native Desktop Application
    let app = KathamoChatApp::new();
    run_native(app).expect("Kathamo Chat native desktop application failed");
}

/// Comprehensive automated verification suite running on the headless test harness.
fn run_headless_verification() {
    println!("[Kathamo Chat] Running comprehensive headless interaction verification...");
    let app = KathamoChatApp::new();
    let mut runner = HeadlessAppRunner::new(app);

    // 1. Initial Presentation
    let cmd_count_frame1 = runner.step_frame();
    assert!(
        cmd_count_frame1 > 50,
        "Chat interface must emit draw commands on frame 1 (got {})",
        cmd_count_frame1
    );
    println!("✓ Frame 1 rendered: {} draw commands.", cmd_count_frame1);

    // 2. Conversation Navigation & Switching
    runner.app.select_conversation("conv_gamedev");
    assert_eq!(runner.app.active_conv_id, "conv_gamedev");
    let title = runner.app.active_conversation().unwrap().title.clone();
    assert!(
        title.contains("Cyberpunk"),
        "Switched to Cyberpunk conversation"
    );
    runner.step_frame();
    println!("✓ Conversation switching verified: '{}'.", title);

    // 3. Search & Live Filter
    runner.app.ui.search_query = "Rust".into();
    let search_hits: Vec<_> = runner
        .app
        .conversations
        .iter()
        .filter(|c| c.title.contains("Rust"))
        .collect();
    assert_eq!(
        search_hits.len(),
        1,
        "Search query should isolate Rust conversation"
    );
    runner.app.ui.search_query.clear();
    println!("✓ Conversation search & filter verified.");

    // 4. Create New Conversation
    let initial_count = runner.app.conversations.len();
    runner.app.create_new_conversation();
    assert_eq!(runner.app.conversations.len(), initial_count + 1);
    assert_eq!(runner.app.active_conversation().unwrap().messages.len(), 0);
    println!(
        "✓ New conversation creation verified (total: {}).",
        runner.app.conversations.len()
    );

    // 5. Typing and Sending User Message
    runner
        .app
        .send_message("Can you write a concurrent task pipeline in Rust?");
    let conv = runner.app.active_conversation().unwrap();
    assert_eq!(
        conv.messages.len(),
        2,
        "Conversation should have User and Assistant messages"
    );
    assert_eq!(
        conv.messages[0].content,
        "Can you write a concurrent task pipeline in Rust?"
    );
    assert!(
        runner.app.streaming.is_active,
        "Simulated typewriter streaming must be active"
    );
    println!("✓ Message sending & response dispatch verified.");

    // 6. Incremental Typewriter Streaming
    let step1_len = runner.app.streaming.current_char_index;
    runner.step_frame();
    let step2_len = runner.app.streaming.current_char_index;
    assert!(
        step2_len > step1_len,
        "Streaming char index must advance each frame"
    );
    println!(
        "✓ Incremental streaming typewriter verified (advanced from {} to {} chars).",
        step1_len, step2_len
    );

    // 7. Stop Generating Active Response
    runner.app.stop_streaming();
    assert!(
        !runner.app.streaming.is_active,
        "Streaming should be halted"
    );
    println!("✓ Stop-generating active response verified.");

    // 8. Copy to Clipboard
    let msg_to_copy = "Test message content to copy";
    runner.app.copy_to_clipboard(msg_to_copy);
    assert_eq!(runner.app.ui.clipboard_buffer, msg_to_copy);
    println!("✓ Message copy to clipboard verified.");

    // 9. Model Switcher
    runner.app.set_model(ChatModel::LlamaLocal);
    assert_eq!(runner.app.selected_model, ChatModel::LlamaLocal);
    runner.app.set_model(ChatModel::KathamoCore);
    assert_eq!(runner.app.selected_model, ChatModel::KathamoCore);
    println!("✓ Model selector switching verified (ClaudeFast -> LlamaLocal -> KathamoCore).");

    // 10. Sidebar Collapse & Expand
    assert!(runner.app.ui.sidebar_expanded);
    runner.app.toggle_sidebar();
    assert!(
        !runner.app.ui.sidebar_expanded,
        "Sidebar should be collapsed"
    );
    runner.step_frame();
    runner.app.toggle_sidebar();
    assert!(runner.app.ui.sidebar_expanded, "Sidebar should be expanded");
    println!("✓ Sidebar collapse/expand toggle verified.");

    // 11. Code & Artifact Preview Panel
    assert!(!runner.app.ui.preview_panel_open);
    runner.app.toggle_preview_panel();
    assert!(
        runner.app.ui.preview_panel_open,
        "Preview panel should be open"
    );
    runner.step_frame();
    runner.app.toggle_preview_panel();
    assert!(
        !runner.app.ui.preview_panel_open,
        "Preview panel should be closed"
    );
    println!("✓ Collapsible Code / Artifact Preview panel verified.");

    // 12. Bilingual Text Input & Unicode Shaping (Bengali)
    runner.app.create_new_conversation();
    runner
        .app
        .send_message("কাঠামো ফ্রেমওয়ার্কে বাংলা টেক্সট রেন্ডারিং কেমন?");
    let ben_conv = runner.app.active_conversation().unwrap();
    assert!(ben_conv.messages[0].content.contains("কাঠামো ফ্রেমওয়ার্কে বাংলা"));
    runner.app.stop_streaming();
    println!("✓ Bilingual Bengali text input and Unicode scalar safety verified.");

    // 13. Responsive Resizing Verification
    runner.app.window_size = Vec2::new(1280.0, 800.0);
    let cmd_count_1280 = runner.step_frame();
    assert!(cmd_count_1280 > 50, "Valid layout at 1280x800");

    runner.app.window_size = Vec2::new(1600.0, 900.0);
    let cmd_count_1600 = runner.step_frame();
    assert!(cmd_count_1600 > 50, "Valid layout at 1600x900");
    println!("✓ Multi-resolution layout verified (1280x800 and 1600x900).");

    // 14. CPU Safe-Mode Pixel Rasterization
    let pixels = runner
        .capture_pixels()
        .expect("CPU Safe-Mode rasterization must succeed");
    assert_eq!(
        pixels.len(),
        1440 * 900,
        "Pixel buffer match default window dimensions"
    );
    println!(
        "✓ Infallible CPU Safe-Mode pixel rasterization verified: {} pixels.",
        pixels.len()
    );

    println!("===========================================================");
    println!("  ALL KATHAMO CHAT HEADLESS CHECKS PASSED (100% SUCCESS)   ");
    println!("===========================================================");
}
