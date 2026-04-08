pub mod a2a;
#[allow(clippy::module_inception)]
pub mod agent;
pub mod classifier;
pub mod dispatcher;
pub mod hands;
pub mod interruption;
pub mod loop_;
pub mod memory_loader;
pub mod prompt;
pub mod streaming;
pub mod tasks_section;
pub mod workspace;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use agent::{Agent, AgentBuilder};
#[allow(unused_imports)]
pub use loop_::{
    build_tool_instructions, is_tool_loop_cancelled, process_message, run, run_tool_call_loop,
    scrub_credentials, DRAFT_CLEAR_SENTINEL,
};
pub use streaming::{AgentStreamChunk, AgentTurnRequest};
