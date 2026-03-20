use reqwest::Client;
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::db::get_setting_internal;

#[tauri::command]
pub async fn ask_ai(
    app_handle: AppHandle,
    prompt: String,
    system_prompt: Option<String>,
) -> Result<String, String> {
    // 1. Fetch settings
    let api_key = get_setting_internal(&app_handle, "ai_api_key")?
        .unwrap_or_default();
    let base_url = get_setting_internal(&app_handle, "ai_base_url")?
        .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());
    
    // We could make the model configurable as well, falling back to a default
    let model = get_setting_internal(&app_handle, "ai_model")?
        .unwrap_or_else(|| "deepseek-chat".to_string());

    if api_key.is_empty() {
        return Err("API Key is not configured. Please check your AI settings.".to_string());
    }

    let client = Client::new();

    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut messages = Vec::new();
    
    if let Some(sys_p) = system_prompt {
        messages.push(json!({
            "role": "system",
            "content": sys_p
        }));
    }

    messages.push(json!({
        "role": "user",
        "content": prompt
    }));

    let request_body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.3
    });

    let res = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !res.status().is_success() {
        let error_text = res.text().await.unwrap_or_default();
        return Err(format!("API Error: {}", error_text));
    }

    let text = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

    let response_json: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse response JSON: {}\nRaw response: {}", e, text))?;

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}
