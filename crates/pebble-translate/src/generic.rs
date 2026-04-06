use pebble_core::{PebbleError, Result};

use crate::deeplx::build_segments;
use crate::types::TranslateResult;

#[allow(clippy::too_many_arguments)]
pub async fn translate(
    endpoint: &str,
    api_key: Option<&str>,
    source_lang_param: &str,
    target_lang_param: &str,
    text_param: &str,
    result_path: &str,
    text: &str,
    from: &str,
    to: &str,
) -> Result<TranslateResult> {
    let client = reqwest::Client::new();
    let mut body_map = serde_json::Map::new();
    body_map.insert(text_param.to_string(), serde_json::Value::String(text.to_string()));
    body_map.insert(source_lang_param.to_string(), serde_json::Value::String(from.to_string()));
    body_map.insert(target_lang_param.to_string(), serde_json::Value::String(to.to_string()));
    let body = serde_json::Value::Object(body_map);

    let mut req = client.post(endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| PebbleError::Translate(format!("Translate API request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(PebbleError::Translate(format!(
            "Translate API error {status}: {body}"
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| PebbleError::Translate(format!("Translate API parse failed: {e}")))?;

    let translated = resolve_json_path(&json, result_path)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(TranslateResult {
        segments: build_segments(text, &translated),
        translated,
    })
}

pub fn resolve_json_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in path.split('.') {
        if let Ok(index) = part.parse::<usize>() {
            current = current.get(index)?;
        } else {
            current = current.get(part)?;
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_json_path_nested() {
        let json: serde_json::Value = serde_json::json!({
            "data": {
                "translations": [
                    { "translatedText": "hello" }
                ]
            }
        });
        let result = resolve_json_path(&json, "data.translations.0.translatedText");
        assert_eq!(result.unwrap().as_str().unwrap(), "hello");
    }

    #[test]
    fn test_resolve_json_path_missing() {
        let json: serde_json::Value = serde_json::json!({"a": 1});
        let result = resolve_json_path(&json, "b.c");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_json_path_simple() {
        let json: serde_json::Value = serde_json::json!({"text": "translated"});
        let result = resolve_json_path(&json, "text");
        assert_eq!(result.unwrap().as_str().unwrap(), "translated");
    }
}
