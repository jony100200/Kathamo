//! Presentation-only studio workspace components.
//!
//! Widgets draw from theme tokens and Studio primitives, report pointer
//! interaction, and never own application state or perform I/O.

pub mod cards;
pub mod chat;
pub mod controls;
pub mod mascot;
mod mascot_draw;

pub use cards::{agent_card, agent_card_with_mood, model_card, task_card};
pub use chat::{code_block, composer, message_block, ComposerOut, MessageRole};
pub use controls::{channel_row, chip, search_field, segmented};
pub use mascot::{identity as mascot_identity, mascot, Mood};

#[cfg(test)]
#[path = "studio_parts_tests.rs"]
mod tests;
