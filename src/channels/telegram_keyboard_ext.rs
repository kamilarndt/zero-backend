// Telegram Inline Keyboard Extension for TelegramChannel
// Zero-Bloat Architecture - Extends TelegramChannel with keyboard support

use crate::channels::telegram_inline_keyboard::{
    InlineKeyboard, InlineKeyboardButton, CallbackQuery, CallbackAnswer,
};
use anyhow::Result;

/// Extension trait for TelegramChannel with keyboard support
pub trait TelegramKeyboardExt {
    /// Send a message with inline keyboard
    async fn send_with_keyboard(
        &self,
        chat_id: i64,
        text: &str,
        keyboard: &InlineKeyboard,
    ) -> Result<i64>;

    /// Answer a callback query
    async fn answer_callback_query(
        &self,
        callback_query_id: &str,
        text: Option<&str>,
        show_alert: bool,
    ) -> Result<()>;

    /// Send a simple yes/no keyboard
    async fn send_yes_no(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<i64> {
        self.send_with_keyboard(chat_id, text, &InlineKeyboard::yes_no()).await
    }

    /// Send a confirm/cancel keyboard
    async fn send_confirm_cancel(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<i64> {
        self.send_with_keyboard(chat_id, text, &InlineKeyboard::confirm_cancel()).await
    }
}

// Note: This is a placeholder extension trait
// In production, you would implement this on TelegramChannel directly
// by adding methods to the struct in telegram.rs
//
// Example implementation that would be added to telegram.rs:
//
// impl TelegramChannel {
//     pub async fn send_with_keyboard(
//         &self,
//         chat_id: i64,
//         text: &str,
//         keyboard: &InlineKeyboard,
//     ) -> anyhow::Result<i64> {
//         let body = serde_json::json!({
//             "chat_id": chat_id,
//             "text": text,
//             "reply_markup": keyboard,
//             "parse_mode": "HTML",
//         });
//
//         let resp = self.http_client()
//             .post(self.api_url("sendMessage"))
//             .json(&body)
//             .send()
//             .await?;
//
//         if !resp.status().is_success() {
//             let error_text = resp.text().await?;
//             anyhow::bail!("Failed to send message with keyboard: {}", error_text);
//         }
//
//         let json: serde_json::Value = resp.json().await?;
//         let message_id = json["result"]["message_id"]
//             .as_i64()
//             .ok_or_else(|| anyhow::anyhow!("No message_id in response"))?;
//
//         Ok(message_id)
//     }
//
//     pub async fn answer_callback_query(
//         &self,
//         callback_query_id: &str,
//         text: Option<&str>,
//         show_alert: bool,
//     ) -> anyhow::Result<()> {
//         let mut body = serde_json::json!({
//             "callback_query_id": callback_query_id,
//         });
//
//         if let Some(t) = text {
//             body["text"] = serde_json::Value::String(t.to_string());
//             body["show_alert"] = serde_json::Value::Bool(show_alert);
//         }
//
//         let resp = self.http_client()
//             .post(self.api_url("answerCallbackQuery"))
//             .json(&body)
//             .send()
//             .await?;
//
//         if !resp.status().is_success() {
//             let error_text = resp.text().await?;
// anyhow::bail!("Failed to answer callback query: {}", error_text);
//         }
//
//         Ok(())
//     }
// }
//
// // Listen loop with callback query support:
// pub async fn listen_with_callbacks(&self) -> anyhow::Result<()> {
//     let mut offset: i64 = 0;
//
//     loop {
//         let resp = self.http_client()
//             .get(&format!(
//                 "{}?offset={}&timeout=30",
//                 self.api_url("getUpdates"),
//                 offset
//             ))
//             .send()
//             .await?;
//
//         if !resp.status().is_success() {
//             tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
//             continue;
//         }
//
//         let json: serde_json::Value = resp.json().await?;
//         let updates = json["result"].as_array().unwrap_or(&vec![]);
//
//         for update in updates {
//             // Update offset
//             if let Some(update_id) = update["update_id"].as_i64() {
//                 offset = update_id + 1;
//             }
//
//             // Handle callback query
//             if let Some(callback) = update.get("callback_query") {
//                 if let Ok(query) = serde_json::from_value::<CallbackQuery>(callback.clone()) {
//                     if let Some(data) = &query.data {
//                         tracing::info!(
//                             "Callback query from {}: {}",
//                             query.from.username.as_deref().unwrap_or(&query.from.first_name),
//                             data
//                         );
//
//                         // Handle callback based on data
//                         match data.as_str() {
//                             Some("yes") => {
//                                 self.send_message(query.message.as_ref().unwrap().chat.id, "You selected Yes").await?;
//                             }
//                             Some("no") => {
//                                 self.send_message(query.message.as_ref().unwrap().chat.id, "You selected No").await?;
//                             }
//                             Some("confirm") => {
//                                 self.send_message(query.message.as_ref().unwrap().chat.id, "Action confirmed!").await?;
//                             }
//                             Some("cancel") => {
//                                 self.send_message(query.message.as_ref().unwrap().chat.id, "Action cancelled.").await?;
//                             }
//                             _ => {
//                                 // Custom callback data handling
//                             }
//                         }
//                     }
//
//                     // Answer the callback query
//                     self.answer_callback_query(&query.id, None, false).await?;
//                 }
//             }
//
//             // Handle regular messages...
//             if let Some(message) = update.get("message") {
//                 // ... existing message handling
//             }
//         }
//     }
// }

/// Documentation for integrating inline keyboards into TelegramChannel
///
/// To add inline keyboard support to TelegramChannel:
///
/// 1. Add these fields to TelegramChannel struct:
///    - callback_handlers: HashMap<String, Box<dyn Fn(&CallbackQuery) -> Result<()> + Send + Sync>>
///
/// 2. Implement send_with_keyboard() method (see example above)
///
/// 3. Implement answer_callback_query() method (see example above)
///
/// 4. Update listen() to handle callback_query in updates
///
/// 5. Add register_callback() method to allow registering custom handlers
///
/// Example usage:
///
/// ```rust
/// // Register a callback handler
/// channel.register_callback("confirm", |query| {
///     // Handle confirm action
///     async move {
///         // Do something
///         Ok(())
///     }
/// });
///
/// // Send message with keyboard
/// channel.send_with_keyboard(
///     chat_id,
///     "Choose an option:",
///     &InlineKeyboard::new()
///         .row(vec![
///             InlineKeyboardButton::callback("✅ Yes", "yes"),
///             InlineKeyboardButton::callback("❌ No", "no"),
///         ])
/// ).await?;
/// ```
