// Zero-Bloat Telegram Menu Button
// Add menu button to chat (requires @BotFather configuration)

use serde::{Deserialize, Serialize};

/// Menu button configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuButtonConfig {
    pub text: String,
    pub url: String,
}

/// Request to set menu button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMenuButtonRequest {
    pub text: String,
    pub url: String,
}

impl MenuButtonConfig {
    /// Create new menu button config
    pub fn new(text: &str, url: &str) -> Self {
        Self {
            text: text.to_string(),
            url: url.to_string(),
        }
    }

    /// Helper: Create TMA Hub menu button
    pub fn tma_hub() -> Self {
        Self {
            text: "⌘ System Hub".to_string(),
            url: "https://dash.karndt.pl/tma/hub".to_string(),
        }
    }

    /// Helper: Create custom menu button
    pub fn custom(text: &str, url: &str) -> Self {
        Self::new(text, url)
    }

    /// Convert to set request
    pub fn to_set_request(&self) -> SetMenuButtonRequest {
        SetMenuButtonRequest {
            text: self.text.clone(),
            url: self.url.clone(),
        }
    }
}

impl SetMenuButtonRequest {
    /// Create new set menu button request
    pub fn new(text: &str, url: &str) -> Self {
        Self {
            text: text.to_string(),
            url: url.to_string(),
        }
    }
}

/// Instructions for @BotFather configuration
pub const BOTFATHER_INSTRUCTIONS: &str = r#"
To configure the Telegram Menu Button for your bot:

1. Open a chat with @BotFather
2. Send the command: /setmenubutton
3. Select your bot from the list
4. Enter the URL for the menu button:
   https://dash.karndt.pl/tma/hub
5. Enter the text for the menu button:
   ⌘ System Hub

After configuration:
- The menu button will appear in the bottom-left corner of chat
- When tapped, it will open the specified URL in Telegram Mini App
- The Mini App will have access to initData for authentication

Example:
  Bot: @YourBotName
  URL: https://your-domain.com/tma/hub
  Text: ⌘ Hub
"#;
