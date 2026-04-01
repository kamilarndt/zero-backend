// onboard/wizard_validation.rs — Validation helpers

use anyhow::{bail, Result};
use console::style;
use dialoguer::Confirm;
use std::io::IsTerminal;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveOnboardingMode {
    FullOnboarding,
    UpdateProviderOnly,
}

pub fn resolve_interactive_onboarding_mode(config_path: &Path, force: bool) -> Result<InteractiveOnboardingMode> {
    if !config_path.exists() {
        return Ok(InteractiveOnboardingMode::FullOnboarding);
    }
    if force {
        println!(
            "  {} Existing config detected at {}. Proceeding with full onboarding because --force was provided.",
            style("!").yellow().bold(),
            style(config_path.display()).yellow()
        );
        return Ok(InteractiveOnboardingMode::FullOnboarding);
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "Refusing to overwrite existing config at {} in non-interactive mode. Re-run with --force if overwrite is intentional.",
            config_path.display()
        );
    }
    let options = [
        "Full onboarding (overwrite config.toml)",
        "Update AI provider/model/API key only (preserve existing configuration)",
        "Cancel",
    ];
    let mode = dialoguer::Select::new()
        .with_prompt(format!("  Existing config found at {}. Select setup mode", config_path.display()))
        .items(&options)
        .default(1)
        .interact()?;
    match mode {
        0 => Ok(InteractiveOnboardingMode::FullOnboarding),
        1 => Ok(InteractiveOnboardingMode::UpdateProviderOnly),
        _ => bail!("Onboarding canceled: existing configuration was left unchanged."),
    }
}

pub fn ensure_onboard_overwrite_allowed(config_path: &Path, force: bool) -> Result<()> {
    if !config_path.exists() { return Ok(()); }
    if force {
        println!(
            "  {} Existing config detected at {}. Proceeding because --force was provided.",
            style("!").yellow().bold(),
            style(config_path.display()).yellow()
        );
        return Ok(());
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!(
            "Refusing to overwrite existing config at {} in non-interactive mode. Re-run with --force if overwrite is intentional.",
            config_path.display()
        );
    }
    let confirmed = Confirm::new()
        .with_prompt(format!(
            "  Existing config found at {}. Re-running onboarding will overwrite config.toml and may create missing workspace files (including BOOTSTRAP.md). Continue?",
            config_path.display()
        ))
        .default(false)
        .interact()?;
    if !confirmed {
        bail!("Onboarding canceled: existing configuration was left unchanged.");
    }
    Ok(())
}

pub fn has_launchable_channels(channels: &crate::config::ChannelsConfig) -> bool {
    channels.channels_except_webhook().iter().any(|(_, ok)| *ok)
}

pub fn apply_provider_update(
    config: &mut crate::config::Config,
    provider: String,
    api_key: String,
    model: String,
    provider_api_url: Option<String>,
) {
    config.default_provider = Some(provider);
    config.default_model = Some(model);
    config.api_url = provider_api_url;
    config.api_key = if api_key.trim().is_empty() { None } else { Some(api_key) };
}

pub async fn persist_workspace_selection(config_path: &Path) -> Result<()> {
    let config_dir = config_path.parent().context("Config path must have a parent directory")?;
    crate::config::schema::persist_active_workspace_config_dir(config_dir)
        .await
        .with_context(|| format!("Failed to persist active workspace selection for {}", config_dir.display()))
}
