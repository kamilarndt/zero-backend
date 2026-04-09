//! Buddy — AI Co-founder dla ADHD
//!
//! Core module implementing the personality, emotional state, and
//! interdependence mechanics that make Buddy a partner, not just an assistant.
//!
//! ## Architecture
//!
//! - [`Personality`] — Defines who Buddy is (name, traits, language)
//! - [`EmotionalState`] — How Buddy feels (Neutral → Frustrated → Proud)
//! - [`Blocker`] — Interdependence mechanic (agent blocked by user decisions)
//! - [`Situation`] — Events that trigger emotional transitions
//! - [`BuddyState`] — Combined state machine tying everything together

pub mod blocker;
pub mod cli;
pub mod conversation;
pub mod emotional_state;
pub mod evolution;
pub mod logger;
pub mod patterns;
pub mod personality;
pub mod situation;
pub mod state;

// Re-exports for ergonomic use
pub use blocker::{Blocker, BlockerSeverity};
pub use conversation::ConversationProcessor;
pub use emotional_state::EmotionalState;
pub use personality::Personality;
pub use situation::Situation;
pub use state::BuddyState;
