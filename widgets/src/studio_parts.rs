//! Presentation-only studio workspace components.
//!
//! Widgets draw from theme tokens and Studio primitives, report pointer
//! interaction, and never own application state or perform I/O.

pub mod cards;
pub mod chat;
pub mod controls;

pub use cards::{agent_card, model_card, task_card};
pub use chat::{code_block, composer, message_block, ComposerOut, MessageRole};
pub use controls::{channel_row, chip, search_field, segmented};

#[cfg(test)]
#[path = "studio_parts_tests.rs"]
mod tests;
