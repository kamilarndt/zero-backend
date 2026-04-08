//! Server-Sent Events (SSE) parser for streaming LLM responses.
//!
//! This module provides a parser for SSE format, commonly used by LLM providers
//! for streaming responses (e.g., OpenAI, Anthropic, Gemini).

use std::fmt;

/// Types of SSE events.
#[derive(Debug, Clone, PartialEq)]
pub enum SseEventType {
    /// Regular data event
    Data,
    /// Event name/type
    Event,
    /// Event ID
    Id,
    /// Retry interval
    Retry,
    /// Comment line (starts with ':')
    Comment,
    /// Unknown or malformed line
    Unknown,
}

/// A parsed Server-Sent Event.
///
/// SSE format consists of lines starting with field names followed by a colon,
/// then the field value. Events are separated by blank lines.
///
/// Example SSE format:
/// ```text
/// data: {"content": "Hello"}
///
/// data: {"content": " World"}
///
/// ```
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// The type of this event
    pub event_type: SseEventType,
    /// The event value (after the colon, trimmed)
    pub value: String,
}

impl SseEvent {
    /// Create a new SSE event.
    pub fn new(event_type: SseEventType, value: String) -> Self {
        Self { event_type, value }
    }

    /// Create a data event (the most common type for LLM streaming).
    pub fn data(value: String) -> Self {
        Self {
            event_type: SseEventType::Data,
            value,
        }
    }

    /// Create an event type event.
    pub fn event(value: String) -> Self {
        Self {
            event_type: SseEventType::Event,
            value,
        }
    }

    /// Create an ID event.
    pub fn id(value: String) -> Self {
        Self {
            event_type: SseEventType::Id,
            value,
        }
    }

    /// Create a retry event.
    pub fn retry(value: String) -> Self {
        Self {
            event_type: SseEventType::Retry,
            value,
        }
    }

    /// Check if this is a data event.
    pub fn is_data(&self) -> bool {
        self.event_type == SseEventType::Data
    }

    /// Check if this is a comment (should be ignored).
    pub fn is_comment(&self) -> bool {
        self.event_type == SseEventType::Comment
    }
}

impl fmt::Display for SseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.event_type {
            SseEventType::Data => write!(f, "data: {}", self.value),
            SseEventType::Event => write!(f, "event: {}", self.value),
            SseEventType::Id => write!(f, "id: {}", self.value),
            SseEventType::Retry => write!(f, "retry: {}", self.value),
            SseEventType::Comment => write!(f, ": {}", self.value),
            SseEventType::Unknown => write!(f, "{}", self.value),
        }
    }
}

/// Error type for SSE parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum SseParseError {
    /// Invalid SSE format
    InvalidFormat(String),
    /// Incomplete event data
    IncompleteEvent,
}

impl fmt::Display for SseParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "Invalid SSE format: {}", msg),
            Self::IncompleteEvent => write!(f, "Incomplete SSE event"),
        }
    }
}

impl std::error::Error for SseParseError {}

/// Parser for Server-Sent Events.
///
/// This parser handles the SSE format used by many LLM providers for streaming
/// responses. It accumulates data across multiple SSE chunks and returns complete
/// events when separated by blank lines.
///
/// # Example
/// ```rust
/// use crate::providers::common::sse_parser::SseParser;
///
/// let mut parser = SseParser::new();
///
/// // Feed SSE chunks
/// let chunk1 = "data: {\"content\": \"Hello\"}\n\n";
/// for event in parser.parse(chunk1) {
///     if event.is_data() {
///         println!("Data: {}", event.value);
///     }
/// }
/// ```
pub struct SseParser {
    /// Buffer for incomplete event data
    buffer: String,
    /// Whether we're currently inside an event
    in_event: bool,
}

impl SseParser {
    /// Create a new SSE parser.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            in_event: false,
        }
    }

    /// Parse a chunk of SSE data and return complete events.
    ///
    /// This method accumulates data across calls and returns events only when
    /// they're complete (terminated by a blank line).
    ///
    /// # Arguments
    /// * `chunk` - A chunk of SSE data (may be partial)
    ///
    /// # Returns
    /// A vector of complete events parsed from the accumulated data
    pub fn parse(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();
        let mut lines = self.buffer.lines().peekable();

        while let Some(line) = lines.next() {
            // Empty line marks the end of an event
            if line.is_empty() {
                if self.in_event {
                    self.in_event = false;
                }
                continue;
            }

            // Skip comments
            if line.starts_with(':') {
                events.push(SseEvent::new(
                    SseEventType::Comment,
                    line[1..].trim().to_string(),
                ));
                continue;
            }

            // Parse field: value pairs
            if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = line[colon_pos + 1..].trim().to_string();

                let event_type = match field {
                    "data" => SseEventType::Data,
                    "event" => SseEventType::Event,
                    "id" => SseEventType::Id,
                    "retry" => SseEventType::Retry,
                    _ => SseEventType::Unknown,
                };

                events.push(SseEvent::new(event_type, value));
                self.in_event = true;
            } else {
                // Line without colon - treat as data with empty value
                events.push(SseEvent::data(line.to_string()));
                self.in_event = true;
            }
        }

        // Clear the buffer if we've processed all lines
        if self.buffer.lines().count() == events.len() + self.buffer.matches('\n').count() {
            self.buffer.clear();
        }

        events
    }

    /// Parse SSE chunks from a bytes buffer (common for streaming responses).
    ///
    /// # Arguments
    /// * `bytes` - Raw bytes from a streaming HTTP response
    ///
    /// # Returns
    /// A vector of complete events parsed from the chunk
    ///
    /// # Errors
    /// Returns an error if the bytes cannot be interpreted as UTF-8
    pub fn parse_bytes(&mut self, bytes: &[u8]) -> Result<Vec<SseEvent>, SseParseError> {
        let chunk = std::str::from_utf8(bytes)
            .map_err(|e| SseParseError::InvalidFormat(format!("Invalid UTF-8: {}", e)))?;
        Ok(self.parse(chunk))
    }

    /// Reset the parser state, clearing any buffered data.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.in_event = false;
    }

    /// Check if the parser is currently buffering incomplete event data.
    pub fn is_buffering(&self) -> bool {
        !self.buffer.is_empty()
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to parse a single SSE chunk.
///
/// # Example
/// ```rust
/// let events = parse_sse_chunk("data: hello\n\ndata: world\n\n")?;
/// assert_eq!(events.len(), 2);
/// ```
pub fn parse_sse_chunk(chunk: &str) -> Result<Vec<SseEvent>, SseParseError> {
    let mut parser = SseParser::new();
    Ok(parser.parse(chunk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_data() {
        let event = SseEvent::data("hello".to_string());
        assert!(event.is_data());
        assert!(!event.is_comment());
    }

    #[test]
    fn test_sse_event_comment() {
        let event = SseEvent::new(SseEventType::Comment, "comment".to_string());
        assert!(event.is_comment());
        assert!(!event.is_data());
    }

    #[test]
    fn test_sse_parser_single_event() {
        let mut parser = SseParser::new();
        let events = parser.parse("data: hello\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, SseEventType::Data);
        assert_eq!(events[0].value, "hello");
    }

    #[test]
    fn test_sse_parser_multiple_events() {
        let mut parser = SseParser::new();
        let events = parser.parse("data: hello\n\ndata: world\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].value, "hello");
        assert_eq!(events[1].value, "world");
    }

    #[test]
    fn test_sse_parser_json_data() {
        let mut parser = SseParser::new();
        let events = parser.parse("data: {\"text\": \"Hello\"}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, "{\"text\": \"Hello\"}");
    }

    #[test]
    fn test_sse_parser_comment() {
        let mut parser = SseParser::new();
        let events = parser.parse(": this is a comment\n\ndata: real data\n\n");
        assert_eq!(events.len(), 2);
        assert!(events[0].is_comment());
        assert_eq!(events[1].value, "real data");
    }

    #[test]
    fn test_sse_parser_event_type() {
        let mut parser = SseParser::new();
        let events = parser.parse("event: message\ndata: hello\n\n");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, SseEventType::Event);
        assert_eq!(events[0].value, "message");
        assert_eq!(events[1].event_type, SseEventType::Data);
    }

    #[test]
    fn test_sse_parser_retry() {
        let mut parser = SseParser::new();
        let events = parser.parse("retry: 1000\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, SseEventType::Retry);
        assert_eq!(events[0].value, "1000");
    }

    #[test]
    fn test_sse_parser_chunked() {
        let mut parser = SseParser::new();

        // First chunk - incomplete event
        let events1 = parser.parse("data: hel");
        assert_eq!(events1.len(), 0);
        assert!(parser.is_buffering());

        // Second chunk - complete the event
        let events2 = parser.parse("lo\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].value, "hello");
    }

    #[test]
    fn test_sse_parser_reset() {
        let mut parser = SseParser::new();
        parser.parse("data: incompl");
        assert!(parser.is_buffering());

        parser.reset();
        assert!(!parser.is_buffering());
    }

    #[test]
    fn test_sse_parser_parse_bytes() {
        let mut parser = SseParser::new();
        let bytes = b"data: test\n\n";
        let events = parser.parse_bytes(bytes).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, "test");
    }

    #[test]
    fn test_sse_parser_invalid_utf8() {
        let mut parser = SseParser::new();
        let bytes: Vec<u8> = vec![0xFF, 0xFE]; // Invalid UTF-8
        let result = parser.parse_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_sse_chunk_convenience() {
        let events = parse_sse_chunk("data: hello\n\n").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, "hello");
    }

    #[test]
    fn test_sse_parser_empty_lines() {
        let mut parser = SseParser::new();
        let events = parser.parse("\n\ndata: hello\n\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, "hello");
    }

    #[test]
    fn test_sse_event_display() {
        let event = SseEvent::data("hello".to_string());
        assert_eq!(format!("{}", event), "data: hello");

        let event2 = SseEvent::event("message".to_string());
        assert_eq!(format!("{}", event2), "event: message");
    }
}
