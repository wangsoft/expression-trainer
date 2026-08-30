// Multi-backend LLM client (OpenAI / DeepSeek / Ollama / custom OpenAI-compatible).
// Ported from the Electron `lib/ai-feedback.js`.

use crate::prompts;
use serde_json::{json, Value};

struct ProviderConfig {
    endpoint: String,
    api_key: String,
    model: String,
}

fn s(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

// Resolve endpoint/key/model from a full settings object (provider + providers map).
fn resolve(settings: &Value) -> Result<ProviderConfig, String> {
    let provider = settings
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("deepseek");
    let pc = settings
        .get("providers")
        .and_then(|p| p.get(provider))
        .cloned()
        .unwrap_or(json!({}));

    match provider {
        "openai" => Ok(ProviderConfig {
            endpoint: "https://api.openai.com/v1/chat/completions".into(),
            api_key: s(&pc, "apiKey"),
            model: {
                let m = s(&pc, "model");
                if m.is_empty() { "gpt-4o-mini".into() } else { m }
            },
        }),
        "deepseek" => Ok(ProviderConfig {
            endpoint: "https://api.deepseek.com/v1/chat/completions".into(),
            api_key: s(&pc, "apiKey"),
            model: {
                let m = s(&pc, "model");
                if m.is_empty() { "deepseek-chat".into() } else { m }
            },
        }),
        "ollama" => {
            let mut base = s(&pc, "ollamaUrl");
            if base.is_empty() {
                base = "http://localhost:11434".into();
            }
            Ok(ProviderConfig {
                endpoint: format!("{}/v1/chat/completions", base),
                api_key: "ollama".into(),
                model: {
                    let m = s(&pc, "model");
                    if m.is_empty() { "qwen2.5:7b".into() } else { m }
                },
            })
        }
        "custom" => {
            let base = s(&pc, "baseUrl");
            let base = base.trim_end_matches('/').to_string();
            let endpoint = if base.is_empty() {
                String::new()
            } else {
                format!("{}/chat/completions", base)
            };
            let custom_model = s(&pc, "customModel");
            let model = if custom_model.is_empty() { s(&pc, "model") } else { custom_model };
            Ok(ProviderConfig {
                endpoint,
                api_key: s(&pc, "apiKey"),
                model,
            })
        }
        other => Err(format!("未知的 provider: {}", other)),
    }
}

async fn call_api(
    cfg: &ProviderConfig,
    messages: Value,
    max_tokens: u32,
    temperature: f32,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(&cfg.endpoint)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .json(&json!({
            "model": cfg.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        }))
        .send()
        .await
        .map_err(|e| format!("连接失败: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_else(|_| "未知错误".into());
        return Err(format!("API 请求失败 ({}): {}", status.as_u16(), body));
    }

    let data: Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    data["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "响应中没有内容".to_string())
}

fn messages(system: &str, user: &str) -> Value {
    json!([
        { "role": "system", "content": system },
        { "role": "user", "content": user },
    ])
}

pub async fn send_feedback(
    text: &str,
    settings: &Value,
    custom: Option<&Value>,
) -> Result<String, String> {
    let cfg = resolve(settings)?;
    let p = prompts::realtime_prompt(text, custom);
    call_api(&cfg, messages(&p.system, &p.user), 150, 0.7).await
}

pub async fn send_report(
    full_text: &str,
    stats: &Value,
    settings: &Value,
    custom: Option<&Value>,
) -> Result<String, String> {
    let cfg = resolve(settings)?;
    let p = prompts::report_prompt(full_text, stats, custom);
    call_api(&cfg, messages(&p.system, &p.user), 8192, 0.7).await
}

// Returns {success, error?} exactly like the Electron testConnection.
pub async fn test_connection(settings: &Value) -> Value {
    let cfg = match resolve(settings) {
        Ok(c) => c,
        Err(e) => return json!({ "success": false, "error": e }),
    };
    if cfg.endpoint.is_empty() {
        return json!({ "success": false, "error": "端点地址未配置" });
    }
    let msgs = json!([{ "role": "user", "content": "OK" }]);
    match call_api(&cfg, msgs, 2, 0.0).await {
        Ok(_) => json!({ "success": true }),
        Err(e) => json!({ "success": false, "error": e }),
    }
}
