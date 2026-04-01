// onboard/wizard_ui.rs — UI helpers (banner, steps, bullets, summary)

use console::style;

pub const BANNER: &str = r"
    ⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡

    ███████╗███████╗██████╗  ██████╗  ██████╗██╗      █████╗ ██╗    ██╗
    ╚══███╔╝██╔════╝██╔══██╗██╔═══██╗██╔════╝██║     ██╔══██╗██║    ██║
      ███╔╝ █████╗  ██████╔╝██║   ██║██║     ██║     ███████║██║ █╗ ██║
     ███╔╝  ██╔══╝  ██╔══██╗██║   ██║██║     ██║     ██╔══██║██║███╗██║
    ███████╗███████╗██║  ██║╚██████╔╝╚██████╗███████╗██║  ██║╚███╔███╔╝
    ╚══════╝╚══════╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝

    Zero overhead. Zero compromise. 100% Rust. 100% Agnostic.

    ⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡⚡
";

pub fn print_step(current: u8, total: u8, title: &str) {
    println!();
    println!(
        "  {} {}",
        style(format!("[{current}/{total}]")).cyan().bold(),
        style(title).white().bold()
    );
    println!("  {}", style("─".repeat(50)).dim());
}

pub fn print_bullet(text: &str) {
    println!("  {} {}", style("›").cyan(), text);
}

pub fn print_model_preview(models: &[String]) {
    const MODEL_PREVIEW_LIMIT: usize = 20;
    for model in models.iter().take(MODEL_PREVIEW_LIMIT) {
        println!("  {} {model}", style("-"));
    }
    if models.len() > MODEL_PREVIEW_LIMIT {
        println!(
            "  {} ... and {} more",
            style("-"),
            models.len() - MODEL_PREVIEW_LIMIT
        );
    }
}

pub fn print_summary(config: &crate::config::Config) {
    println!();
    println!("  {} Configuration Summary", style("📋").cyan());
    println!("  {}", style("─".repeat(50)).dim());
    println!(
        "  {} Provider: {}",
        style("✓").green().bold(),
        style(config.default_provider.as_deref().unwrap_or("not set")).green()
    );
    println!(
        "  {} Model: {}",
        style("✓").green().bold(),
        style(config.default_model.as_deref().unwrap_or("not set")).green()
    );
    println!(
        "  {} Workspace: {}",
        style("✓").green().bold(),
        style(config.workspace_dir.display()).green()
    );
    println!();
}
