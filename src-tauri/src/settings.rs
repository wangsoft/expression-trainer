// Settings + custom-prompt persistence.
// Ported from the Electron main.js (loadSettings / saveSettings / migration).

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn default_provider_configs() -> Value {
    json!({
        "openai":   { "apiKey": "", "model": "gpt-4o-mini" },
        "deepseek": { "apiKey": "", "model": "deepseek-chat" },
        "ollama":   { "ollamaUrl": "http://localhost:11434", "model": "qwen2.5:7b" },
        "custom":   { "apiKey": "", "baseUrl": "", "model": "" }
    })
}

fn config_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .expect("cannot resolve app config dir");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn settings_path(app: &AppHandle) -> PathBuf {
    config_dir(app).join("settings.json")
}

fn custom_prompt_path(app: &AppHandle) -> PathBuf {
    config_dir(app).join("custom-prompt.json")
}

fn default_settings() -> Value {
    json!({ "provider": "deepseek", "providers": default_provider_configs() })
}

// Merge missing default keys into each provider block (shallow, defaults first).
fn ensure_provider_defaults(raw: &mut Value) {
    let defaults = default_provider_configs();
    let providers = raw
        .as_object_mut()
        .and_then(|o| o.get_mut("providers"))
        .and_then(|p| p.as_object_mut());
    if let (Some(providers), Some(defs)) = (providers, defaults.as_object()) {
        for (key, def) in defs {
            match providers.get(key) {
                None => {
                    providers.insert(key.clone(), def.clone());
                }
                Some(existing) => {
                    let mut merged = def.as_object().cloned().unwrap_or_default();
                    if let Some(cur) = existing.as_object() {
                        for (k, v) in cur {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                    providers.insert(key.clone(), Value::Object(merged));
                }
            }
        }
    }
}

// Migrate the legacy flat schema (apiKey/model at top level) to per-provider.
fn migrate_flat(raw: &Value) -> Value {
    let provider = raw
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("deepseek")
        .to_string();
    let mut migrated = json!({ "provider": provider, "providers": default_provider_configs() });
    let providers = migrated["providers"].as_object_mut().unwrap();
    let p = migrated_provider_key(raw);

    if let Some(v) = raw.get("apiKey").and_then(|v| v.as_str()) {
        providers[&p]["apiKey"] = json!(v);
    }
    if let Some(v) = raw.get("model").and_then(|v| v.as_str()) {
        providers[&p]["model"] = json!(v);
    }
    if let Some(v) = raw.get("ollamaUrl").and_then(|v| v.as_str()) {
        providers["ollama"]["ollamaUrl"] = json!(v);
    }
    if let Some(v) = raw.get("customEndpoint").and_then(|v| v.as_str()) {
        providers["custom"]["baseUrl"] = json!(v);
    }
    if let Some(v) = raw.get("customModel").and_then(|v| v.as_str()) {
        providers["custom"]["model"] = json!(v);
    }
    migrated
}

fn migrated_provider_key(raw: &Value) -> String {
    raw.get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("deepseek")
        .to_string()
}

pub fn load_settings(app: &AppHandle) -> Value {
    let path = settings_path(app);
    let Ok(text) = fs::read_to_string(&path) else {
        return default_settings();
    };
    let Ok(raw) = serde_json::from_str::<Value>(&text) else {
        return default_settings();
    };

    if raw.get("providers").is_none() {
        let migrated = migrate_flat(&raw);
        let _ = save_settings(app, &migrated);
        return migrated;
    }

    let mut raw = raw;
    ensure_provider_defaults(&mut raw);
    raw
}

pub fn save_settings(app: &AppHandle, settings: &Value) -> Result<(), String> {
    let path = settings_path(app);
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

pub fn load_custom_prompt(app: &AppHandle) -> Option<Value> {
    let path = custom_prompt_path(app);
    let text = fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

pub fn save_custom_prompt(app: &AppHandle, data: &Value) -> Result<(), String> {
    let path = custom_prompt_path(app);
    let text = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}
