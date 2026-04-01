//! End-to-end tests for channel message processing.
//!
//! These tests verify the complete message flow from channel
//! through agent loop to response delivery.

use super::*;
use crate::agent::loop_::*;
use crate::config::Config;
use crate::memory::{Memory, MemoryCategory};
use crate::observability::NoopObserver;
use crate::providers::{ChatMessage, Provider};
use crate::tools::Tool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

// Test doubles

struct DummyProvider;

#[async_trait::async_trait]
impl Provider for DummyProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

#[derive(Default)]
struct RecordingChannel {
    sent_messages: tokio::sync::Mutex<Vec<String>>,
    start_typing_calls: AtomicUsize,
    stop_typing_calls: AtomicUsize,
    reactions_added: tokio::sync::Mutex<Vec<(String, String, String)>>,
    reactions_removed: tokio::sync::Mutex<Vec<(String, String, String)>>,
}

#[async_trait::async_trait]
impl crate::channels::traits::Channel for RecordingChannel {
    fn name(&self) -> &str {
        "test-channel"
    }

    async fn send(&self, message: &crate::channels::SendMessage) -> anyhow::Result<()> {
        self.sent_messages
            .lock()
            .await
            .push(format!("{}:{}", message.recipient, message.content));
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<crate::channels::traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.start_typing_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.stop_typing_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn add_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        self.reactions_added.lock().await.push((
            channel_id.to_string(),
            message_id.to_string(),
            emoji.to_string(),
        ));
        Ok(())
    }

    async fn remove_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> anyhow::Result<()> {
        self.reactions_removed.lock().await.push((
            channel_id.to_string(),
            message_id.to_string(),
            emoji.to_string(),
        ));
        Ok(())
    }
}

struct NoopMemory;

#[async_trait::async_trait]
impl Memory for NoopMemory {
    fn name(&self) -> &str {
        "noop"
    }

    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: crate::memory::MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn get(&self, _key: &str) -> anyhow::Result<Option<crate::memory::MemoryEntry>> {
        Ok(None)
    }

    async fn list(
        &self,
        _category: Option<&crate::memory::MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<crate::memory::MemoryEntry>> {
        Ok(Vec::new())
    }

    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ── E2E: photo [IMAGE:] marker rejected by non-vision provider ───

/// End-to-end test: a photo attachment message (containing `[IMAGE:]`
/// marker) sent through `process_channel_message` with a non-vision
/// provider must produce a `"⚠️ Error: …does not support vision"` reply
/// on the recording channel — no real Telegram or LLM API required.
#[tokio::test]
async fn e2e_photo_attachment_rejected_by_non_vision_provider() {
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn crate::channels::traits::Channel> = channel_impl.clone();

    let mut channels_by_name = HashMap::new();
    channels_by_name.insert(channel.name().to_string(), channel);

    // DummyProvider has default capabilities (vision: false).
    let runtime_ctx = Arc::new(crate::channels::ChannelRuntimeContext {
        channels_by_name: Arc::new(channels_by_name),
        provider: Arc::new(DummyProvider),
        default_provider: Arc::new("dummy".to_string()),
        memory: Arc::new(NoopMemory),
        tools_registry: Arc::new(vec![]),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("You are a helpful assistant.".to_string()),
        model: Arc::new("test-model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_tool_iterations: 5,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(crate::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(std::env::temp_dir()),
        message_timeout_secs: crate::channels::CHANNEL_MESSAGE_TIMEOUT_SECS,
        interrupt_on_new_message: false,
        multimodal: crate::config::MultimodalConfig::default(),
        hooks: None,
        all_skills: Arc::new(vec![]),
        non_cli_excluded_tools: Arc::new(Vec::new()),
    });

    // Simulate a photo attachment message with [IMAGE:] marker.
    crate::channels::process_channel_message(
        runtime_ctx,
        crate::channels::traits::ChannelMessage {
            id: "msg-photo-1".to_string(),
            sender: "zeroclaw_user".to_string(),
            reply_target: "chat-photo".to_string(),
            content: "[IMAGE:/tmp/workspace/photo_99_1.jpg]\n\nWhat is this?".to_string(),
            channel: "test-channel".to_string(),
            timestamp: 1,
            thread_ts: None,
            active_skills: vec![],
        },
        CancellationToken::new(),
    )
    .await;

    let sent = channel_impl.sent_messages.lock().await;
    assert_eq!(sent.len(), 1, "expected exactly one reply message");
    assert!(
        sent[0].contains("does not support vision"),
        "reply must mention vision capability error, got: {}",
        sent[0]
    );
    assert!(
        sent[0].contains("⚠️ Error"),
        "reply must start with error prefix, got: {}",
        sent[0]
    );
}

#[tokio::test]
async fn e2e_failed_vision_turn_does_not_poison_follow_up_text_turn() {
    let channel_impl = Arc::new(RecordingChannel::default());
    let channel: Arc<dyn crate::channels::traits::Channel> = channel_impl.clone();

    let mut channels_by_name = HashMap::new();
    channels_by_name.insert(channel.name().to_string(), channel);

    let runtime_ctx = Arc::new(crate::channels::ChannelRuntimeContext {
        channels_by_name: Arc::new(channels_by_name),
        provider: Arc::new(DummyProvider),
        default_provider: Arc::new("dummy".to_string()),
        memory: Arc::new(NoopMemory),
        tools_registry: Arc::new(vec![]),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("You are a helpful assistant.".to_string()),
        model: Arc::new("test-model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_tool_iterations: 5,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(crate::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(std::env::temp_dir()),
        message_timeout_secs: crate::channels::CHANNEL_MESSAGE_TIMEOUT_SECS,
        interrupt_on_new_message: false,
        multimodal: crate::config::MultimodalConfig::default(),
        hooks: None,
        all_skills: Arc::new(vec![]),
        non_cli_excluded_tools: Arc::new(Vec::new()),
    });

    crate::channels::process_channel_message(
        Arc::clone(&runtime_ctx),
        crate::channels::traits::ChannelMessage {
            id: "msg-photo-1".to_string(),
            sender: "zeroclaw_user".to_string(),
            reply_target: "chat-photo".to_string(),
            content: "[IMAGE:/tmp/workspace/photo_99_1.jpg]\n\nWhat is this?".to_string(),
            channel: "test-channel".to_string(),
            timestamp: 1,
            thread_ts: None,
            active_skills: vec![],
        },
        CancellationToken::new(),
    )
    .await;

    crate::channels::process_channel_message(
        Arc::clone(&runtime_ctx),
        crate::channels::traits::ChannelMessage {
            id: "msg-text-2".to_string(),
            sender: "zeroclaw_user".to_string(),
            reply_target: "chat-photo".to_string(),
            content: "What is WAL?".to_string(),
            channel: "test-channel".to_string(),
            timestamp: 2,
            thread_ts: None,
            active_skills: vec![],
        },
        CancellationToken::new(),
    )
    .await;

    let sent = channel_impl.sent_messages.lock().await;
    assert_eq!(sent.len(), 2, "expected one error and one successful reply");
    assert!(
        sent[0].contains("does not support vision"),
        "first reply must mention vision capability error, got: {}",
        sent[0]
    );
    assert!(
        sent[1].ends_with(":ok"),
        "second reply should succeed for text-only turn, got: {}",
        sent[1]
    );
    drop(sent);

    let histories = runtime_ctx
        .conversation_histories
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let turns = histories
        .get("test-channel_zeroclaw_user")
        .expect("history should exist for sender");
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].role, "user");
    assert_eq!(turns[0].content, "What is WAL?");
    assert_eq!(turns[1].role, "assistant");
    assert_eq!(turns[1].content, "ok");
    assert!(
        turns.iter().all(|turn| !turn.content.contains("[IMAGE:")),
        "failed vision turn must not persist image marker content"
    );
}
