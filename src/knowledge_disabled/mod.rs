//! Knowledge Base module — documents, blocks, SQLite persistence.
//!
//! Minimal MVP: documents + paragraph/heading blocks, REST CRUD, SSE events.

pub mod schema;
pub mod database;
pub mod routes;
pub mod sse;

pub use database::KnowledgeStore;
