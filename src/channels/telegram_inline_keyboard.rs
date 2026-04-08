// Zero-Bloat Inline Keyboards for Telegram
// No external dependencies beyond serde

use serde::{Deserialize, Serialize};

/// Inline keyboard with buttons arranged in rows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineKeyboard {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

/// Single inline keyboard button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_app: Option<WebAppInfo>,
}

/// Web App info for Mini Apps button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppInfo {
    pub url: String,
}

/// Callback query from inline button press
#[derive(Debug, Clone, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: CallbackQueryMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
    pub data: Option<String>,
}

/// Callback query message (user who pressed button)
#[derive(Debug, Clone, Deserialize)]
pub struct CallbackQueryMessage {
    pub id: i64,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// Answer to a callback query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackAnswer {
    pub callback_query_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_alert: Option<bool>,
}

impl InlineKeyboard {
    /// Create a new empty inline keyboard
    pub fn new() -> Self {
        Self {
            inline_keyboard: Vec::new(),
        }
    }

    /// Add a row of buttons
    pub fn add_row(mut self, buttons: Vec<InlineKeyboardButton>) -> Self {
        self.inline_keyboard.push(buttons);
        self
    }

    /// Helper: Create Yes/No keyboard
    pub fn yes_no() -> Self {
        Self::new().add_row(vec![
            InlineKeyboardButton {
                text: "✅ Yes".to_string(),
                callback_data: Some("action:yes".to_string()),
                url: None,
                web_app: None,
            },
            InlineKeyboardButton {
                text: "❌ No".to_string(),
                callback_data: Some("action:no".to_string()),
                url: None,
                web_app: None,
            },
        ])
    }

    /// Helper: Create Confirm/Cancel keyboard
    pub fn confirm_cancel() -> Self {
        Self::new().add_row(vec![
            InlineKeyboardButton {
                text: "✅ Confirm".to_string(),
                callback_data: Some("action:confirm".to_string()),
                url: None,
                web_app: None,
            },
            InlineKeyboardButton {
                text: "❌ Cancel".to_string(),
                callback_data: Some("action:cancel".to_string()),
                url: None,
                web_app: None,
            },
        ])
    }

    /// Helper: Create a single button with URL
    pub fn url_button(text: &str, url: &str) -> Self {
        Self::new().add_row(vec![InlineKeyboardButton {
            text: text.to_string(),
            callback_data: None,
            url: Some(url.to_string()),
            web_app: None,
        }])
    }
}

impl CallbackAnswer {
    /// Create a new callback answer
    pub fn new(query_id: String) -> Self {
        Self {
            callback_query_id: query_id,
            text: None,
            show_alert: None,
        }
    }

    /// Set the text (optional)
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// Set whether to show alert (optional)
    pub fn show_alert(mut self, show: bool) -> Self {
        self.show_alert = Some(show);
        self
    }
}
