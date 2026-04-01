pub mod schema;
pub mod traits;
pub mod routing;

// Domain-specific schema modules
pub mod schemas {
    pub mod memory_schema;
    pub mod security_schema;
    pub mod llm_schema;

    // Re-export memory types
    pub use memory_schema::{
        StorageConfig, StorageProviderSection, StorageProviderConfig,
        MemoryConfig, QdrantConfig,
    };

    // Re-export security types
    pub use security_schema::{
        SecurityConfig, OtpConfig, OtpMethod, EstopConfig,
        SandboxConfig, SandboxBackend, ResourceLimitsConfig, AuditConfig,
    };

    // Re-export LLM types
    pub use llm_schema::{
        ModelRouteConfig, EmbeddingRouteConfig,
        QueryClassificationConfig, ClassificationRule, DelegateAgentConfig,
    };
}

// Re-export types from schema.rs (main config types)
#[allow(unused_imports)]
pub use schema::{
    apply_runtime_proxy_to_builder, build_runtime_proxy_client,
    build_runtime_proxy_client_with_timeouts, runtime_proxy_config, set_runtime_proxy_config,
    default_nostr_relays,
    // Types defined in schema.rs
    AgentConfig, AutonomyConfig, BrowserComputerUseConfig, BrowserConfig,
    BuiltinHooksConfig, ChannelsConfig, ComposioConfig, Config, CostConfig,
    CronConfig, DiscordConfig, DockerRuntimeConfig,
    DingTalkConfig, FeishuConfig, GatewayConfig, HardwareConfig, HardwareTransport,
    HeartbeatConfig, HooksConfig, HttpRequestConfig, IMessageConfig, IdentityConfig, IrcConfig,
    LarkConfig, LarkReceiveMode, LinqConfig, MatrixConfig,
    MultimodalConfig, NextcloudTalkConfig, NostrConfig, ObservabilityConfig,
    PeripheralBoardConfig, PeripheralsConfig, ProxyConfig, ProxyScope,
    QQConfig, QuotaRoutingSchema, ReliabilityConfig,
    RuntimeConfig, SchedulerConfig, SecretsConfig, SignalConfig,
    SkillsConfig, SkillsPromptInjectionMode, SlackConfig,
    StreamMode, TelegramConfig, TranscriptionConfig, TunnelConfig, TailscaleTunnelConfig,
    WebFetchConfig, WebSearchConfig, WebhookConfig, WhatsAppConfig,
};

// Re-export types from schemas submodules
#[allow(unused_imports)]
pub use schemas::{
    // From llm_schema
    ClassificationRule, DelegateAgentConfig, EmbeddingRouteConfig, ModelRouteConfig, QueryClassificationConfig,
    // From memory_schema
    MemoryConfig, QdrantConfig, StorageConfig, StorageProviderConfig, StorageProviderSection,
    // From security_schema
    AuditConfig, EstopConfig, OtpConfig, OtpMethod, ResourceLimitsConfig, SandboxBackend, SandboxConfig, SecurityConfig,
};

pub fn name_and_presence<T: traits::ChannelConfig>(channel: &Option<T>) -> (&'static str, bool) {
    (T::name(), channel.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_config_default_is_constructible() {
        let config = Config::default();

        assert!(config.default_provider.is_some());
        assert!(config.default_model.is_some());
        assert!(config.default_temperature > 0.0);
    }

    #[test]
    fn reexported_channel_configs_are_constructible() {
        let telegram = TelegramConfig {
            bot_token: "token".into(),
            allowed_users: vec!["alice".into()],
            stream_mode: StreamMode::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            webhook_url: None,
        };

        let discord = DiscordConfig {
            bot_token: "token".into(),
            guild_id: Some("123".into()),
            allowed_users: vec![],
            listen_to_bots: false,
            mention_only: false,
        };

        let lark = LarkConfig {
            app_id: "app-id".into(),
            app_secret: "app-secret".into(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: vec![],
            mention_only: false,
            use_feishu: false,
            receive_mode: crate::config::schema::LarkReceiveMode::Websocket,
            port: None,
        };
        let feishu = FeishuConfig {
            app_id: "app-id".into(),
            app_secret: "app-secret".into(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: vec![],
            receive_mode: crate::config::schema::LarkReceiveMode::Websocket,
            port: None,
        };

        let nextcloud_talk = NextcloudTalkConfig {
            base_url: "https://cloud.example.com".into(),
            app_token: "app-token".into(),
            webhook_secret: None,
            allowed_users: vec!["*".into()],
        };

        assert_eq!(telegram.allowed_users.len(), 1);
        assert_eq!(discord.guild_id.as_deref(), Some("123"));
        assert_eq!(lark.app_id, "app-id");
        assert_eq!(feishu.app_id, "app-id");
        assert_eq!(nextcloud_talk.base_url, "https://cloud.example.com");
    }
}
