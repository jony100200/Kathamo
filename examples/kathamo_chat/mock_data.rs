//! Kathamo Chat — Prepopulated Demonstrations & Scripted Responses
//!
//! Provides realistic, substantive mock conversations and interactive scripted
//! responses without any external AI model or network dependencies.

use super::models::{ChatConversation, ChatMessageItem, ChatModel, CodeArtifact};
use ui_widgets::rich_text::ChatRole;

pub fn create_prepopulated_conversations() -> Vec<ChatConversation> {
    vec![
        create_rust_conversation(),
        create_game_dev_conversation(),
        create_worldbuilding_conversation(),
        create_general_bengali_conversation(),
    ]
}

/// Conversation 1: Rust Programming Assistance
fn create_rust_conversation() -> ChatConversation {
    let mut conv = ChatConversation::new(
        "conv_rust",
        "Rust Concurrent Raycaster",
        "Rayon BVH acceleration & zero-allocation",
        ChatModel::ClaudeFast,
        "10 mins ago",
    );
    conv.is_pinned = true;

    conv.add_message(ChatMessageItem::new(
        "rust_msg_1",
        ChatRole::User,
        "How do I structure a concurrent ray-box intersection in Rust using Rayon without heap allocations during tracing?",
        "10:14 AM",
    ));

    let rust_code = r#"use rayon::prelude::*;

#[derive(Clone, Copy)]
pub struct Ray {
    pub origin: [f32; 3],
    pub dir_inv: [f32; 3], // Precomputed 1.0 / dir for fast slab test
}

#[derive(Clone, Copy)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    #[inline(always)]
    pub fn intersect(&self, ray: &Ray, mut t_min: f32, mut t_max: f32) -> bool {
        for i in 0..3 {
            let t0 = (self.min[i] - ray.origin[i]) * ray.dir_inv[i];
            let t1 = (self.max[i] - ray.origin[i]) * ray.dir_inv[i];
            let (near, far) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
            t_min = t_min.max(near);
            t_max = t_max.min(far);
            if t_max <= t_min { return false; }
        }
        true
    }
}

pub fn trace_rays_parallel(boxes: &[Aabb], rays: &[Ray]) -> Vec<bool> {
    rays.par_iter()
        .map(|ray| boxes.iter().any(|b| b.intersect(ray, 0.001, f32::MAX)))
        .collect()
}"#;

    let artifact = CodeArtifact {
        id: "art_rust_raycaster".into(),
        title: "raycaster.rs".into(),
        language: "rust".into(),
        code: rust_code.into(),
        description: "Zero-allocation Rayon Ray-Box intersection kernel".into(),
    };

    let reply = format!(
        "### High-Performance Ray-Box Slab Intersect\n\n\
        To eliminate allocations inside your hot path:\n\n\
        1. **Precompute reciprocal directions**: Store `dir_inv = 1.0 / dir` once per ray to replace divisions with multiplications.\n\
        2. **Use flat contiguous arrays**: Keep `&[Aabb]` in cache-aligned structs without pointers.\n\
        3. **Leverage Rayon iterators**: `par_iter()` balances tiles over work-stealing worker threads with zero per-ray synchronization.\n\n\
        ```rust\n{}\n```\n\n\
        This achieves sub-microsecond throughput per ray tile on multi-core Ryzen CPUs.",
        rust_code
    );

    conv.add_message(
        ChatMessageItem::new("rust_msg_2", ChatRole::Assistant, reply, "10:15 AM")
            .with_artifact(artifact),
    );

    conv
}

/// Conversation 2: Game Development Ideas
fn create_game_dev_conversation() -> ChatConversation {
    let mut conv = ChatConversation::new(
        "conv_gamedev",
        "Cyberpunk 2.5D City Architecture",
        "Layered parallax, volumetric neon & rain FX",
        ChatModel::LlamaLocal,
        "1 hour ago",
    );

    conv.add_message(ChatMessageItem::new(
        "game_msg_1",
        ChatRole::User,
        "Can you break down the visual layer stack for a moody 2.5D cyberpunk alleyway in our indie engine?",
        "09:12 AM",
    ));

    let gamedev_spec = r#"# Cyberpunk 2.5D Alleyway Pipeline Spec

## 1. Depth Layering (Back to Front)
- **Far Background (Parallax 0.1x)**: Sky gradient with distant megastructure silhouettes and holographic billboard glow.
- **Mid-Far Background (Parallax 0.35x)**: Elevated rail transit, steam vents, tangled power cables.
- **Midground Play-plane (Parallax 1.0x)**: Walkable pavement, puddles with screen-space planar reflection, storefront neon.
- **Foreground Silhouettes (Parallax 1.4x)**: Overhanging AC units, dripping fire escapes, security cameras.

## 2. Atmospheric Shader Passes
- Volumetric neon fog with exponential height falloff.
- Screen-space rain streak particles with directional wind distortion.
- Anamorphic lens flare on bright specular highlights."#;

    let artifact = CodeArtifact {
        id: "art_cyberpunk_spec".into(),
        title: "alleyway_pipeline.md".into(),
        language: "markdown".into(),
        code: gamedev_spec.into(),
        description: "2.5D parallax layer and shader pipeline design".into(),
    };

    let reply = format!(
        "### Cyberpunk 2.5D Atmospheric Architecture\n\n\
        Here is the production-tested 4-tier layer configuration:\n\n\
        - **Sky & Megastructure Silhouettes**: Parallax scale `0.10x` with atmospheric haze depth fog.\n\
        - **Industrial Rooftops & Monorail**: Parallax `0.35x` with intermittent neon transit blurs.\n\
        - **Main Gameplay Surface**: Parallax `1.00x` containing physics colliders, interactive doors, and character sprites.\n\
        - **Cinematic Framing Foreground**: Parallax `1.45x` out-of-focus cables and rain-drenched neon signage.\n\n\
        ```markdown\n{}\n```\n\n\
        Tip: Render wet pavement using a single specular blur pass multiplied by the ground puddle normal map.",
        gamedev_spec
    );

    conv.add_message(
        ChatMessageItem::new("game_msg_2", ChatRole::Assistant, reply, "09:14 AM")
            .with_artifact(artifact),
    );

    conv
}

/// Conversation 3: Creative Project Planning
fn create_worldbuilding_conversation() -> ChatConversation {
    let mut conv = ChatConversation::new(
        "conv_lore",
        "Solar Flare Worldbuilding & Lore",
        "Faction conflicts, geothermal cities & lore",
        ChatModel::KathamoCore,
        "Yesterday",
    );

    conv.add_message(ChatMessageItem::new(
        "lore_msg_1",
        ChatRole::User,
        "Outline the key factions for a sci-fi universe where humanity lives entirely underground after the Great Aurora.",
        "Yesterday",
    ));

    let lore_doc = r#"# The Sub-Terra Factions of Sol-3

## 1. The Core Geothermal Consortium (The Molten Guild)
- **Philosophy**: Technocratic survival through thermodynamic mastery.
- **Resource Control**: Geothermal tapheads, heat-sink reservoirs, heavy power turbines.
- **Aesthetic**: Brass heavy plating, pressurized steam conduits, glowing heat vents.

## 2. The Spore-Wardens (Bioluminescent Collective)
- **Philosophy**: Genetic symbiosis with subterranean fungi and algae strains.
- **Resource Control**: Hydroponic caverns, synthetic protein, clean atmospheric scrubbers.
- **Aesthetic**: Woven mycelium fibers, ambient teal bioluminescence, breathing filters.

## 3. The Surface Seekers (The Aurora Heresy)
- **Philosophy**: Reclaiming the scorched planetary crust despite lethal solar radiation.
- **Resource Control**: Scavenged titanium crawlers, experimental lead-lined hazard suits.
- **Aesthetic**: Weathered military rebreathers, polarized gold-tinted blast visors."#;

    let artifact = CodeArtifact {
        id: "art_factions_lore".into(),
        title: "factions_manifesto.md".into(),
        language: "markdown".into(),
        code: lore_doc.into(),
        description: "Subterranean humanity faction manifesto and resource economies".into(),
    };

    let reply = format!(
        "### Subterranean Sol-3 Factions Overview\n\n\
        In this setting, society organizes around thermodynamic energy and air filtration:\n\n\
        - **The Molten Guild**: Controls the deep geothermal tapheads and power generation.\n\
        - **The Spore-Wardens**: Bio-engineers cultivating light-emitting fungal flora for food and breathable air.\n\
        - **The Aurora Heresy**: Outcasts who venture onto the radioactive crust in leaded hazard suits.\n\n\
        ```markdown\n{}\n```\n\n\
        Each faction controls a vital survival choke-point, guaranteeing tension in trade negotiations.",
        lore_doc
    );

    conv.add_message(
        ChatMessageItem::new("lore_msg_2", ChatRole::Assistant, reply, "Yesterday")
            .with_artifact(artifact),
    );

    conv
}

/// Conversation 4: General Conversation & Bengali Support
fn create_general_bengali_conversation() -> ChatConversation {
    let mut conv = ChatConversation::new(
        "conv_bengali",
        "Quick Setup & Bengali Support",
        "Bilingual text shaping & HarfBuzz verification",
        ChatModel::ClaudeFast,
        "2 days ago",
    );

    conv.add_message(ChatMessageItem::new(
        "ben_msg_1",
        ChatRole::User,
        "কাঠামো ফ্রেমওয়ার্কে বাংলা টেক্সট রেন্ডারিং কেমন কাজ করে? (How does Bengali text rendering work in Kathamo?)",
        "Oct 23",
    ));

    conv.add_message(ChatMessageItem::new(
        "ben_msg_2",
        ChatRole::Assistant,
        "### কাঠামো ফ্রেমওয়ার্ক — বহুভাষিক টাইপোগ্রাফি আর্কিটেকচার\n\n\
        কাঠামো ফ্রেমওয়ার্ক সম্পূর্ণ দেশীয়ভাবে **HarfBuzz / rustybuzz** ভিত্তিক টেক্সট শেপিং ইঞ্জিন ব্যবহার করে:\n\n\
        - **যুক্তাক্ষর সঠিক রূপায়ন**: বাংলা যুক্তাক্ষর (যেমন: ক্ষ, জ্ঞ, ঙ্গ, ত্র) নিখুঁতভাবে গঠিত হয়।\n\
        - **Unicode স্কেলার সেফটি**: ক্যারেক্টার বাফার কখনো বাইটের মাঝে বিভক্ত হয় না; ফলে ইমোজি বা যুক্তাক্ষর ভেঙে যাওয়ার ঝুঁকি নেই।\n\
        - **জিরো গ্লিচ বাফার**: GPU এবং CPU সেফ-মোড উভয় পাইপলাইনে অভিন্ন টেক্সট মেট্রিক্স বজায় থাকে।\n\n\
        আপনি যেকোনো প্রশ্ন ইংরেজি কিংবা বাংলায় স্বাচ্ছন্দ্যে করতে পারেন!",
        "Oct 23",
    ));

    conv
}

/// Generates a contextual mock reply for any newly typed user message.
pub fn generate_scripted_response(
    user_input: &str,
    model: ChatModel,
) -> (String, Option<CodeArtifact>) {
    let lower = user_input.to_lowercase();

    if lower.contains("rust") || lower.contains("code") || lower.contains("function") {
        let code = r#"// Fast deterministic parallel pipeline in Rust
pub struct TaskPipeline<T> {
    workers: usize,
    buffer: Vec<T>,
}

impl<T: Send + Sync + 'static> TaskPipeline<T> {
    pub fn new(workers: usize) -> Self {
        Self { workers, buffer: Vec::new() }
    }

    pub fn process<F>(&mut self, f: F)
    where
        F: Fn(&T) + Sync + Send,
    {
        use rayon::prelude::*;
        self.buffer.par_iter().for_each(&f);
    }
}"#;
        let artifact = CodeArtifact {
            id: "art_dyn_rust".into(),
            title: "pipeline.rs".into(),
            language: "rust".into(),
            code: code.into(),
            description: "High-throughput parallel pipeline implementation".into(),
        };
        let text = format!(
            "### Rust Implementation Generated via {}\n\n\
            Here is a clean, production-ready implementation tailored to your request:\n\n\
            - **Thread Safety**: Uses `Send + Sync` constraints for safe multi-core distribution.\n\
            - **Work Stealing**: Backed by Rayon's lock-free deque architecture.\n\
            - **Zero Heap Churn**: Reuses preallocated buffers across execution ticks.\n\n\
            ```rust\n{}\n```\n\n\
            Click 'View Code in Panel' to inspect this snippet in the side drawer.",
            model.display_name(),
            code
        );
        (text, Some(artifact))
    } else if lower.contains("bengali") || lower.contains("বাংলা") || lower.contains("কেমন")
    {
        let text = format!(
            "### কাঠামো ফ্রেমওয়ার্কে আপনাকে স্বাগতম! ({})\n\n\
            আপনার বার্তাটি সফলভাবে গৃহীত হয়েছে।\n\n\
            - **তাত্ক্ষণিক প্রতিক্রিয়া**: স্থানীয় সিমুলেশন ইঞ্জিন দ্রুত কাজ করছে।\n\
            - **উন্নত লেআউট**: ফন্ট শেপিং এবং ভিজ্যুয়াল উপাদান নিখুঁতভাবে সমন্বয় করা হয়েছে।\n\n\
            আমি আপনাকে পরবর্তী যেকোনো কাজে সাহায্য করতে প্রস্তুত!",
            model.display_name()
        );
        (text, None)
    } else {
        let text = format!(
            "### Response from {}\n\n\
            Thank you for your prompt: **\"{}\"**\n\n\
            Here is a structured assessment based on your inquiry:\n\n\
            - **Deterministic Pipeline**: Executed smoothly across all Kathamo layout boundaries.\n\
            - **Responsive Typography**: Rendered with dynamic font metrics and automatic line wrapping.\n\
            - **Native Event Loop**: Processed within the 60 FPS update window with zero dropped frames.\n\n\
            Feel free to ask for specific Rust code, architectural layouts, or game design specs!",
            model.display_name(),
            user_input.trim()
        );
        (text, None)
    }
}
