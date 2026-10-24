//! Ojon AI Model Studio Showcase Tab (Quantization, GGUF/AWQ/EXL2, Perplexity checks).

use crate::app::UiLabApp;
use ui_core::*;
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::{BilingualLabel, ThemePalette};
use ui_widgets::*;

pub fn render_ojon_workstation_tab(
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

    let pointer = app.pointer.clone();
    let mut ui = KathamoUi::new(cmd_buf, &mut app.icon_atlas, p, content_rect, pointer);

    // 1. Top Telemetry & Workstation Header
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

    // 2. Main Two-Column Layout (Sidebar: 320px, Content: Flex 1.0)
    let main_height = (content_rect.height() - 110.0).max(300.0);
    ui.columns(
        main_height,
        &[SizeConstraint::Fixed(320.0), SizeConstraint::Flex(1.0)],
        |col_idx, col_ui| {
            if col_idx == 0 {
                col_ui.heading("MODEL REPOSITORY");
                col_ui.search_bar(&mut app.ojon_search_query, "Filter local checkpoints...");

                let models = [
                    ("Qwen-2.5-Coder-7B-Instruct", "7.6B Params", "GGUF Ready"),
                    ("Meta-Llama-3.1-8B-Instruct", "8.0B Params", "GGUF Ready"),
                    ("Gemma-2-9B-IT", "9.2B Params", "AWQ Compatible"),
                    ("Mistral-7B-Instruct-v0.3", "7.2B Params", "GGUF Ready"),
                    ("Qwen2-VL-7B-Instruct", "7.6B VLM", "Mixed Precision"),
                    ("Flux.1-dev", "12.0B DiT", "FP8 Candidate"),
                ];

                for (i, (name, params, quant)) in models.iter().enumerate() {
                    let is_selected = app.ojon_selected_model == i;
                    if col_ui.model_card(name, params, quant, is_selected).clicked() {
                        app.ojon_selected_model = i;
                    }
                }

                col_ui.button_secondary("+ Import HuggingFace / Local");
            } else {
                let model_names = [
                    "Qwen-2.5-Coder-7B-Instruct",
                    "Meta-Llama-3.1-8B-Instruct",
                    "Gemma-2-9B-IT",
                    "Mistral-7B-Instruct-v0.3",
                    "Qwen2-VL-7B-Instruct",
                    "Flux.1-dev",
                ];
                let current_name = model_names[app.ojon_selected_model.min(model_names.len() - 1)];

                col_ui.horizontal(30.0, |hdr| {
                    hdr.heading(current_name);
                    hdr.badge("BFloat16 Source", true);
                    hdr.badge("4 Shards (15.2 GB)", true);
                    hdr.badge("RoPE 128k", true);
                });

                col_ui.glass_card(180.0, |card| {
                    card.label("TARGET QUANTIZATION ENGINE & OUTPUT FORMAT");
                    let formats = [
                        "GGUF (llama.cpp)",
                        "AWQ (AutoAWQ W4A16)",
                        "EXL2 (ExLlamaV2)",
                        "FP8 (TensorRT-LLM)",
                    ];
                    card.tabs(&formats, &mut app.ojon_selected_tab);

                    let recipes = [
                        "Q5_K_M (Recommended Quality — 5.45 bpw, 5.8 GB output)",
                        "Q4_K_M (Balanced Compact — 4.50 bpw, 4.9 GB output)",
                        "Q8_0 (Near Lossless — 8.50 bpw, 8.2 GB output)",
                        "IQ3_M (High Compression — 3.66 bpw, 3.9 GB output)",
                    ];
                    card.dropdown("Quantization Recipe", &mut app.ojon_quant_recipe, &recipes);

                    card.horizontal(24.0, |row| {
                        row.toggle("Preserve Output Head (FP16)", &mut app.ojon_preserve_head);
                        row.toggle("Offload KV Cache to GPU", &mut app.ojon_offload_kv);
                        row.toggle("FlashAttention-2 Kernel", &mut app.ojon_flash_attn);
                        row.toggle("IMatrix Calibration Matrix", &mut app.ojon_imatrix);
                    });

                    card.scrubber(
                        "Calibration Context Window",
                        &mut app.ojon_calib_ctx,
                        512.0,
                        8192.0,
                    );
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
                    test.text_input(
                        &mut app.ojon_test_prompt,
                        "Enter prompt for perplexity check...",
                    );
                    test.horizontal(26.0, |row| {
                        row.button_secondary("Run Perplexity Benchmark (PPL)");
                        row.label(
                            "Measured PPL: 5.62 (+0.08 delta vs FP16 baseline) | Memory: 6.2 GB VRAM",
                        );
                    });
                });
            }
        },
    );
}
