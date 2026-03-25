//! Common HTTP client and utilities for LLM providers.
//!
//! This module contains reusable components for provider implementations:
//! - HTTP client factory with retry logic and timeout configuration
//! - Common error handling and response parsing
//! - Server-Sent Events (SSE) parser for streaming responses

pub mod http_client;
pub mod sse_parser;
