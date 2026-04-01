//! Channel registry for collecting and starting configured channels.
//!
//! This module extracts channel startup logic from the main channels module,
//! providing a clean separation of concerns for channel initialization.

use crate::channels::TelegramChannel;
use crate::channels::discord::DiscordChannel;
use crate::channels::slack::SlackChannel;
use crate::channels::mattermost::MattermostChannel;
use crate::channels::imessage::IMessageChannel;
#[cfg(feature = "channel-matrix")]
use crate::channels::matrix::MatrixChannel;
use crate::channels::signal::SignalChannel;
use crate::channels::whatsapp::WhatsAppChannel;
#[cfg(feature = "whatsapp-web")]
use crate::channels::whatsapp_web::WhatsAppWebChannel;
use crate::channels::linq::LinqChannel;
use crate::channels::wati::WatiChannel;
use crate::channels::nextcloud_talk::NextcloudTalkChannel;
use crate::channels::email_channel::EmailChannel;
use crate::channels::irc::IrcChannel;
#[cfg(feature = "channel-lark")]
use crate::channels::lark::LarkChannel;
use crate::channels::dingtalk::DingTalkChannel;
use crate::channels::qq::QQChannel;
use crate::channels::clawdtalk::ClawdTalkChannel;
use crate::channels::nostr::NostrChannel;
use crate::config::Config;
use crate::channels::traits::Channel;
use anyhow::Result;
use std::sync::Arc;

/// A configured channel with its display name.
#[derive(Clone)]
pub struct ConfiguredChannel {
    /// Human-readable display name for the channel.
    pub display_name: &'static str,
    /// The channel instance.
    pub channel: Arc<dyn Channel>,
}

/// Channel registry for collecting and managing configured channels.
pub struct ChannelRegistry;

impl ChannelRegistry {
    /// Create a new channel registry (no state needed currently).
    pub fn new() -> Self {
        Self
    }

    /// Collect all configured channels from the configuration.
    ///
    /// This iterates through all possible channel configurations and creates
    /// channel instances for those that are properly configured.
    pub fn collect_configured_channels(
        config: &Config,
        matrix_skip_context: &str,
    ) -> Vec<ConfiguredChannel> {
        let mut channels = Vec::new();

        // Telegram
        if let Some(ref tg) = config.channels_config.telegram {
            let mut telegram_channel = TelegramChannel::new(
                tg.bot_token.clone(),
                tg.allowed_users.clone(),
                tg.mention_only,
            )
            .with_streaming(tg.stream_mode, tg.draft_update_interval_ms)
            .with_transcription(config.transcription.clone())
            .with_workspace_dir(config.workspace_dir.clone());

            if let Some(ref webhook_url) = tg.webhook_url {
                telegram_channel = telegram_channel.with_webhook_url(webhook_url.clone());
            }

            channels.push(ConfiguredChannel {
                display_name: "Telegram",
                channel: Arc::new(telegram_channel),
            });
        }

        // Discord
        if let Some(ref dc) = config.channels_config.discord {
            channels.push(ConfiguredChannel {
                display_name: "Discord",
                channel: Arc::new(DiscordChannel::new(
                    dc.bot_token.clone(),
                    dc.guild_id.clone(),
                    dc.allowed_users.clone(),
                    dc.listen_to_bots,
                    dc.mention_only,
                )),
            });
        }

        // Slack
        if let Some(ref sl) = config.channels_config.slack {
            channels.push(ConfiguredChannel {
                display_name: "Slack",
                channel: Arc::new(SlackChannel::new(
                    sl.bot_token.clone(),
                    sl.channel_id.clone(),
                    sl.allowed_users.clone(),
                )),
            });
        }

        // Mattermost
        if let Some(ref mm) = config.channels_config.mattermost {
            channels.push(ConfiguredChannel {
                display_name: "Mattermost",
                channel: Arc::new(MattermostChannel::new(
                    mm.url.clone(),
                    mm.bot_token.clone(),
                    mm.channel_id.clone(),
                    mm.allowed_users.clone(),
                    mm.thread_replies.unwrap_or(true),
                    mm.mention_only.unwrap_or(false),
                )),
            });
        }

        // iMessage
        if let Some(ref im) = config.channels_config.imessage {
            channels.push(ConfiguredChannel {
                display_name: "iMessage",
                channel: Arc::new(IMessageChannel::new(im.allowed_contacts.clone())),
            });
        }

        // Matrix (feature-gated)
        #[cfg(feature = "channel-matrix")]
        if let Some(ref mx) = config.channels_config.matrix {
            channels.push(ConfiguredChannel {
                display_name: "Matrix",
                channel: Arc::new(MatrixChannel::new_with_session_hint_and_zeroclaw_dir(
                    mx.homeserver.clone(),
                    mx.access_token.clone(),
                    mx.room_id.clone(),
                    mx.allowed_users.clone(),
                    mx.user_id.clone(),
                    mx.device_id.clone(),
                    config.config_path.parent().map(|path| path.to_path_buf()),
                )),
            });
        }

        #[cfg(not(feature = "channel-matrix"))]
        if config.channels_config.matrix.is_some() {
            tracing::warn!(
                "Matrix channel is configured but this build was compiled without `channel-matrix`; skipping Matrix {}.",
                matrix_skip_context
            );
        }

        // Signal
        if let Some(ref sig) = config.channels_config.signal {
            channels.push(ConfiguredChannel {
                display_name: "Signal",
                channel: Arc::new(SignalChannel::new(
                    sig.http_url.clone(),
                    sig.account.clone(),
                    sig.group_id.clone(),
                    sig.allowed_from.clone(),
                    sig.ignore_attachments,
                    sig.ignore_stories,
                )),
            });
        }

        // WhatsApp (with runtime negotiation)
        if let Some(ref wa) = config.channels_config.whatsapp {
            if wa.is_ambiguous_config() {
                tracing::warn!(
                    "WhatsApp config has both phone_number_id and session_path set; preferring Cloud API mode. Remove one selector to avoid ambiguity."
                );
            }
            match wa.backend_type() {
                "cloud" => {
                    if wa.is_cloud_config() {
                        channels.push(ConfiguredChannel {
                            display_name: "WhatsApp",
                            channel: Arc::new(WhatsAppChannel::new(
                                wa.access_token.clone().unwrap_or_default(),
                                wa.phone_number_id.clone().unwrap_or_default(),
                                wa.verify_token.clone().unwrap_or_default(),
                                wa.allowed_numbers.clone(),
                            )),
                        });
                    } else {
                        tracing::warn!("WhatsApp Cloud API configured but missing required fields (phone_number_id, access_token, verify_token)");
                    }
                }
                "web" => {
                    #[cfg(feature = "whatsapp-web")]
                    if wa.is_web_config() {
                        channels.push(ConfiguredChannel {
                            display_name: "WhatsApp",
                            channel: Arc::new(WhatsAppWebChannel::new(
                                wa.session_path.clone().unwrap_or_default(),
                                wa.pair_phone.clone(),
                                wa.pair_code.clone(),
                                wa.allowed_numbers.clone(),
                            )),
                        });
                    } else {
                        tracing::warn!("WhatsApp Web configured but session_path not set");
                    }
                    #[cfg(not(feature = "whatsapp-web"))]
                    {
                        tracing::warn!("WhatsApp Web backend requires 'whatsapp-web' feature. Enable with: cargo build --features whatsapp-web");
                    }
                }
                _ => {
                    tracing::warn!("WhatsApp config invalid: neither phone_number_id (Cloud API) nor session_path (Web) is set");
                }
            }
        }

        // Linq
        if let Some(ref lq) = config.channels_config.linq {
            channels.push(ConfiguredChannel {
                display_name: "Linq",
                channel: Arc::new(LinqChannel::new(
                    lq.api_token.clone(),
                    lq.from_phone.clone(),
                    lq.allowed_senders.clone(),
                )),
            });
        }

        // WATI
        if let Some(ref wati_cfg) = config.channels_config.wati {
            channels.push(ConfiguredChannel {
                display_name: "WATI",
                channel: Arc::new(WatiChannel::new(
                    wati_cfg.api_token.clone(),
                    wati_cfg.api_url.clone(),
                    wati_cfg.tenant_id.clone(),
                    wati_cfg.allowed_numbers.clone(),
                )),
            });
        }

        // Nextcloud Talk
        if let Some(ref nc) = config.channels_config.nextcloud_talk {
            channels.push(ConfiguredChannel {
                display_name: "Nextcloud Talk",
                channel: Arc::new(NextcloudTalkChannel::new(
                    nc.base_url.clone(),
                    nc.app_token.clone(),
                    nc.allowed_users.clone(),
                )),
            });
        }

        // Email
        if let Some(ref email_cfg) = config.channels_config.email {
            channels.push(ConfiguredChannel {
                display_name: "Email",
                channel: Arc::new(EmailChannel::new(email_cfg.clone())),
            });
        }

        // IRC
        if let Some(ref irc) = config.channels_config.irc {
            channels.push(ConfiguredChannel {
                display_name: "IRC",
                channel: Arc::new(IrcChannel::new(crate::channels::irc::IrcChannelConfig {
                    server: irc.server.clone(),
                    port: irc.port,
                    nickname: irc.nickname.clone(),
                    username: irc.username.clone(),
                    channels: irc.channels.clone(),
                    allowed_users: irc.allowed_users.clone(),
                    server_password: irc.server_password.clone(),
                    nickserv_password: irc.nickserv_password.clone(),
                    sasl_password: irc.sasl_password.clone(),
                    verify_tls: irc.verify_tls.unwrap_or(true),
                })),
            });
        }

        // Lark/Feishu (feature-gated)
        #[cfg(feature = "channel-lark")]
        if let Some(ref lk) = config.channels_config.lark {
            if lk.use_feishu {
                if config.channels_config.feishu.is_some() {
                    tracing::warn!(
                        "Both [channels_config.feishu] and legacy [channels_config.lark].use_feishu=true are configured; ignoring legacy Feishu fallback in lark."
                    );
                } else {
                    tracing::warn!(
                        "Using legacy [channels_config.lark].use_feishu=true compatibility path; prefer [channels_config.feishu]."
                    );
                    channels.push(ConfiguredChannel {
                        display_name: "Feishu",
                        channel: Arc::new(LarkChannel::from_config(lk)),
                    });
                }
            } else {
                channels.push(ConfiguredChannel {
                    display_name: "Lark",
                    channel: Arc::new(LarkChannel::from_lark_config(lk)),
                });
            }
        }

        #[cfg(feature = "channel-lark")]
        if let Some(ref fs) = config.channels_config.feishu {
            channels.push(ConfiguredChannel {
                display_name: "Feishu",
                channel: Arc::new(LarkChannel::from_feishu_config(fs)),
            });
        }

        #[cfg(not(feature = "channel-lark"))]
        if config.channels_config.lark.is_some() || config.channels_config.feishu.is_some() {
            tracing::warn!(
                "Lark/Feishu channel is configured but this build was compiled without `channel-lark`; skipping Lark/Feishu health check."
            );
        }

        // DingTalk
        if let Some(ref dt) = config.channels_config.dingtalk {
            channels.push(ConfiguredChannel {
                display_name: "DingTalk",
                channel: Arc::new(DingTalkChannel::new(
                    dt.client_id.clone(),
                    dt.client_secret.clone(),
                    dt.allowed_users.clone(),
                )),
            });
        }

        // QQ
        if let Some(ref qq) = config.channels_config.qq {
            channels.push(ConfiguredChannel {
                display_name: "QQ",
                channel: Arc::new(QQChannel::new(
                    qq.app_id.clone(),
                    qq.app_secret.clone(),
                    qq.allowed_users.clone(),
                )),
            });
        }

        // ClawdTalk
        if let Some(ref ct) = config.channels_config.clawdtalk {
            channels.push(ConfiguredChannel {
                display_name: "ClawdTalk",
                channel: Arc::new(ClawdTalkChannel::new(ct.clone())),
            });
        }

        channels
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect configured channels using the default registry.
pub fn collect_configured_channels(
    config: &Config,
    matrix_skip_context: &str,
) -> Vec<ConfiguredChannel> {
    ChannelRegistry::collect_configured_channels(config, matrix_skip_context)
}
