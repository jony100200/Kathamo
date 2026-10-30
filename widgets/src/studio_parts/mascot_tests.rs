use super::*;
use ui_core::motion;
use ui_platform::PointerState;
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::{ExtendedTokens, ThemeMode, ThemePalette};

const MOODS: [Mood; 5] = [
    Mood::Ready,
    Mood::Working,
    Mood::NeedsYou,
    Mood::Error,
    Mood::Sleeping,
];

fn frame(name: &str, mood: Mood, t: f32, pointer: &PointerState) -> Vec<DrawCommand> {
    let mut buf = CommandBuffer::default();
    let mut s = Studio::new(
        &mut buf,
        ThemePalette::warm_paper(),
        ExtendedTokens::for_mode(ThemeMode::WarmPaper),
        pointer,
    );
    let rect = Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::splat(44.0));
    mascot(&mut s, rect, name, mood, t);
    buf.commands().to_vec()
}

#[test]
fn working_pose_moves_over_time() {
    let ptr = PointerState::default();
    assert_ne!(
        pose(Mood::Working, Some(0.0), 0.3),
        pose(Mood::Working, Some(0.5), 0.3)
    );
    assert_ne!(
        frame("codex", Mood::Working, 0.0, &ptr),
        frame("codex", Mood::Working, 0.5, &ptr)
    );
    // Same time, same frame: drawing is a pure function of time.
    assert_eq!(
        frame("codex", Mood::Working, 0.5, &ptr),
        frame("codex", Mood::Working, 0.5, &ptr)
    );
}

#[test]
fn reduced_motion_draws_a_still_pose() {
    let ptr = PointerState::default();
    motion::set_reduced_motion(true);
    for mood in MOODS {
        assert_eq!(
            frame("hermes", mood, 0.0, &ptr),
            frame("hermes", mood, 0.5, &ptr),
            "{mood:?} must not move under reduced motion"
        );
        assert_eq!(
            frame("hermes", mood, 0.0, &ptr),
            frame("hermes", mood, 7.3, &ptr)
        );
    }
    motion::set_reduced_motion(false);
}

#[test]
fn identity_is_stable_and_names_differ() {
    assert_eq!(identity("codex"), identity("codex"));
    assert_eq!(identity("Codex"), identity("codex"));
    assert_ne!(identity("codex").body, identity("hermes").body);
    let names = [
        "codex", "hermes", "kilo", "opencode", "qwen", "cline", "claude", "grok", "agy", "dsh",
    ];
    let distinct = names.iter().map(|n| identity(n).body).collect::<Vec<_>>();
    let unique = distinct
        .iter()
        .enumerate()
        .filter(|(i, c)| !distinct[..*i].contains(c))
        .count();
    assert_eq!(unique, names.len(), "two agents share a body colour");
    let looks = names
        .iter()
        .map(|n| {
            let id = identity(n);
            (id.body, id.feature)
        })
        .collect::<Vec<_>>();
    let unique_looks = looks
        .iter()
        .enumerate()
        .filter(|(i, l)| !looks[..*i].contains(l))
        .count();
    assert_eq!(unique_looks, names.len(), "two agents look identical");
}

#[test]
fn each_mood_draws_distinctly_and_reports_hover() {
    let ptr = PointerState {
        position: Vec2::new(32.0, 32.0),
        ..PointerState::default()
    };
    let frames: Vec<_> = MOODS.iter().map(|m| frame("kilo", *m, 1.0, &ptr)).collect();
    for (i, f) in frames.iter().enumerate() {
        assert!(f.len() > 8, "{:?} drew too little", MOODS[i]);
        assert!(
            !frames[..i].contains(f),
            "{:?} looks like another mood",
            MOODS[i]
        );
    }
    let mut buf = CommandBuffer::default();
    let mut s = Studio::new(
        &mut buf,
        ThemePalette::midnight_graphite(),
        ExtendedTokens::for_mode(ThemeMode::MidnightGraphite),
        &ptr,
    );
    let rect = Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::splat(44.0));
    assert!(mascot(&mut s, rect, "kilo", Mood::Ready, 0.0).hovered);
}

#[test]
fn tones_map_to_moods() {
    assert_eq!(Mood::from_tone(Tone::Good), Mood::Ready);
    assert_eq!(Mood::from_tone(Tone::Working), Mood::Working);
    assert_eq!(Mood::from_tone(Tone::Warning), Mood::NeedsYou);
    assert_eq!(Mood::from_tone(Tone::Bad), Mood::Error);
    assert_eq!(Mood::from_tone(Tone::Neutral), Mood::Sleeping);
}
