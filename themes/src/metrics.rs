//! Standard studio dimensions, spacing scales, and layout metrics.

pub const HEIGHT_MENU_BAR: f32 = 26.0;
pub const HEIGHT_CONTEXT_TOOLBAR: f32 = 28.0;
pub const HEIGHT_DOCKER_HEADER: f32 = 26.0;
pub const HEIGHT_STATUS_BAR: f32 = 22.0;
pub const HEIGHT_STANDARD_INPUT: f32 = 24.0;
pub const WIDTH_TOOL_PALETTE: f32 = 42.0;
pub const WIDTH_RIGHT_DOCK: f32 = 310.0;
pub const SIZE_TOOL_BUTTON: f32 = 32.0;
pub const SIZE_TOOL_ICON: f32 = 19.0;
pub const CORNER_RADIUS_STANDARD: f32 = 2.0;
pub const CORNER_RADIUS_SURFACE: f32 = 4.0;
pub const ANIMATION_TIME_SECONDS: f32 = 0.08;

/// Spacing scale on a 4 px grid.
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_6: f32 = 24.0;
pub const SPACE_8: f32 = 32.0;

/// Radii for controls (buttons, inputs, pills) and cards.
pub const RADIUS_CONTROL: f32 = 8.0;
pub const RADIUS_CARD: f32 = 12.0;

/// Workspace frame proportions (logical px).
pub const WORKSPACE_HEADER_HEIGHT: f32 = 52.0;
pub const WORKSPACE_RAIL_WIDTH: f32 = 64.0;
pub const WORKSPACE_CONTEXT_WIDTH: f32 = 236.0;
pub const WORKSPACE_INSPECTOR_WIDTH: f32 = 320.0;
pub const WORKSPACE_ROW_HEIGHT: f32 = 44.0;
pub const WORKSPACE_READABLE_WIDTH: f32 = 760.0;

/// Standard transition time; zero when the user asked for reduced motion.
pub const fn motion_seconds(reduced_motion: bool) -> f32 {
    if reduced_motion {
        0.0
    } else {
        0.14
    }
}
