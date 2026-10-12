//! Standalone Example: Ojon (ওজন) AI Model Quantization Workstation
//!
//! Run with: `cargo run -p ui-lab --example ojon_workstation`

use ui_core::{Rect, SizeConstraint, Vec2};
use ui_render::CommandBuffer;
use ui_themes::{BilingualLabel, ThemePalette};
use ui_widgets::{icon_cache::IconAtlas, KathamoUi};

fn main() {
    println!("------------------------------------------------------------");
    println!("🚀 LAUNCHING OJON (ওজন) AI MODEL QUANTIZATION WORKSTATION DEMO");
    println!("   Built with Kathamo (কাঠামো) Unified UI Framework");
    println!("------------------------------------------------------------");

    let mut cmd_buf = CommandBuffer::new();
    let mut icon_atlas = IconAtlas::new();
    let palette = ThemePalette::dark();
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1440.0, 900.0));
    let pointer = ui_platform::PointerState::default();

    let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, viewport, pointer);

    // 1. Top Telemetry Header
    ui.horizontal(72.0, |top| {
        top.vertical(360.0, |brand| {
            brand.bilingual_heading(&BilingualLabel::new("Ojon", "ওজন"));
            brand.horizontal(22.0, |pills| {
                pills.status_pill("CUDA 12.4 Active", true);
                pills.status_pill("Dual VRAM Pools Online", true);
            });
        });

        top.stat_card("PRIMARY VRAM (RTX 4060 Ti)", "11.8 / 16.0 GB", "74% Active (5.2 GB Free)");
        top.stat_card("SECONDARY POOL (ARC B60)", "24.0 GB", "Dedicated VRAM Pool");
        top.stat_card("SYSTEM MEMORY", "21.4 / 32.0 GB", "Direct DMA Buffering");
        top.stat_card("CONVERSION SPEED", "142.8 MB/s", "NVMe Stream Active");
    });

    ui.separator();

    // 2. Main Two-Column Layout
    let mut search_query = String::new();
    let mut selected_model = 0;
    let mut selected_tab = 0;
    let mut quant_recipe = 0;
    let mut preserve_head = true;
    let mut offload_kv = true;
    let mut flash_attn = true;
    let mut imatrix = false;
    let mut calib_ctx = 2048.0;
    let mut test_prompt = String::from("Explain quantum annealing in simple terms.");

    ui.columns(
        780.0,
        &[SizeConstraint::Fixed(320.0), SizeConstraint::Flex(1.0)],
        |col_idx, col_ui| {
            if col_idx == 0 {
                col_ui.heading("MODEL REPOSITORY");
                col_ui.search_bar(&mut search_query, "Filter local checkpoints...");

                let models = [
                    ("Qwen-2.5-Coder-7B-Instruct", "7.6B Params", "GGUF Ready"),
                    ("Meta-Llama-3.1-8B-Instruct", "8.0B Params", "GGUF Ready"),
                    ("Gemma-2-9B-IT", "9.2B Params", "AWQ Compatible"),
                    ("Mistral-7B-Instruct-v0.3", "7.2B Params", "GGUF Ready"),
                    ("Qwen2-VL-7B-Instruct", "7.6B VLM", "Mixed Precision"),
                    ("Flux.1-dev", "12.0B DiT", "FP8 Candidate"),
                ];

                for (i, (name, params, quant)) in models.iter().enumerate() {
                    let is_selected = selected_model == i;
                    if col_ui.model_card(name, params, quant, is_selected).clicked() {
                        selected_model = i;
                    }
                }

                col_ui.button_secondary("+ Import HuggingFace / Local");
            } else {
                col_ui.horizontal(30.0, |hdr| {
                    hdr.heading("Qwen-2.5-Coder-7B-Instruct");
                    hdr.badge("BFloat16 Source", true);
                    hdr.badge("4 Shards (15.2 GB)", true);
                    hdr.badge("RoPE 128k", true);
                });

                col_ui.glass_card(180.0, |card| {
                    card.label("TARGET QUANTIZATION ENGINE & OUTPUT FORMAT");
                    let formats = ["GGUF (llama.cpp)", "AWQ (AutoAWQ W4A16)", "EXL2 (ExLlamaV2)", "FP8 (TensorRT-LLM)"];
                    card.tabs(&formats, &mut selected_tab);

                    let recipes = [
                        "Q5_K_M (Recommended Quality — 5.45 bpw, 5.8 GB output)",
                        "Q4_K_M (Balanced Compact — 4.50 bpw, 4.9 GB output)",
                        "Q8_0 (Near Lossless — 8.50 bpw, 8.2 GB output)",
                        "IQ3_M (High Compression — 3.66 bpw, 3.9 GB output)",
                    ];
                    card.dropdown("Quantization Recipe", &mut quant_recipe, &recipes);

                    card.horizontal(24.0, |row| {
                        row.toggle("Preserve Output Head (FP16)", &mut preserve_head);
                        row.toggle("Offload KV Cache to GPU", &mut offload_kv);
                        row.toggle("FlashAttention-2 Kernel", &mut flash_attn);
                        row.toggle("IMatrix Calibration Matrix", &mut imatrix);
                    });

                    card.scrubber("Calibration Context Window", &mut calib_ctx, 512.0, 8192.0);
                });

                col_ui.glass_card(130.0, |job| {
                    job.horizontal(26.0, |hdr| {
                        hdr.status_pill("Quantizing Shard 3/4", true);
                        hdr.label("Estimated Output: 5.84 GB (-61.4% reduction)");
                        hdr.label("Throughput: 142.8 MB/s (Direct NVMe DMA)");
                        hdr.badge("ETA: 01:24", true);
                    });

                    job.progress(0.68);

                    job.horizontal(32.0, |actions| {
                        actions.button("Quantize Model (Active)");
                        actions.button_secondary("Pause Job");
                        actions.button_secondary("Inspect Layer Tensors");
                        actions.button_secondary("View Conversion Log");
                    });
                });

                col_ui.card(96.0, |test| {
                    test.label("POST-QUANTIZATION INFERENCE VERIFICATION (Ojon Test Harness)");
                    test.text_input(&mut test_prompt, "Enter prompt for perplexity check...");
                    test.horizontal(26.0, |row| {
                        row.button_secondary("Run Perplexity Benchmark (PPL)");
                        row.label("Measured PPL: 5.62 (+0.08 delta vs FP16 baseline) | Memory: 6.2 GB VRAM");
                    });
                });
            }
        },
    );

    println!("✅ Frame evaluated successfully!");
    println!("   Total Emitted Draw Commands: {}", cmd_buf.len());
    println!("   Layout Structure: Master-Detail Split (320px Sidebar + Flex Main)");
    println!("   Visual Controls: Glass Cards, Dual-Stroke Highlight, Status Pills, Toggles, Tabs");
    println!("------------------------------------------------------------");
}
