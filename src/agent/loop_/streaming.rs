use crate::util::truncate_with_ellipsis;

/// Minimum characters per chunk when relaying LLM text to a streaming draft.
pub const STREAM_CHUNK_MIN_CHARS: usize = 80;

/// Minimum interval between progress sends to avoid flooding the draft channel.
pub const PROGRESS_MIN_INTERVAL_MS: u64 = 500;

/// Sentinel value sent through on_delta to signal the draft updater to clear accumulated text.
/// Used before streaming the final answer so progress lines are replaced by the clean response.
pub const DRAFT_CLEAR_SENTINEL: &str = "\x00CLEAR\x00";

/// Extract a short hint from tool call arguments for progress display.
pub fn truncate_tool_args_for_progress(name: &str, args: &serde_json::Value, max_len: usize) -> String {
    let hint = match name {
        "shell" => args.get("command").and_then(|v| v.as_str()),
        "file_read" | "file_write" => args.get("path").and_then(|v| v.as_str()),
        _ => args
            .get("action")
            .and_then(|v| v.as_str())
            .or_else(|| args.get("query").and_then(|v| v.as_str())),
    };
    match hint {
        Some(s) => truncate_with_ellipsis(s, max_len),
        None => String::new(),
    }
}
