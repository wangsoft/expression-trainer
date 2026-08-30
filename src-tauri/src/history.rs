// Training-session history: one JSON file per session under the app config dir.
// A session entry carries transcript + stats and (optionally) the generated report.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn history_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .expect("cannot resolve app config dir")
        .join("history");
    let _ = fs::create_dir_all(&dir);
    dir
}

// Guard against path traversal: ids are frontend timestamps, keep them tame.
fn sanitize_id(id: &str) -> Option<String> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Some(id.to_string())
    } else {
        None
    }
}

// Upsert an entry (keyed by its "id"); overwrites when the report is added later.
pub fn save(app: &AppHandle, entry: &Value) -> Value {
    let Some(id) = entry.get("id").and_then(|v| v.as_str()).and_then(sanitize_id) else {
        return json!({ "success": false, "error": "无效的历史 id" });
    };
    let path = history_dir(app).join(format!("{}.json", id));
    match serde_json::to_string_pretty(entry) {
        Ok(text) => match fs::write(&path, text) {
            Ok(()) => json!({ "success": true, "id": id }),
            Err(e) => json!({ "success": false, "error": e.to_string() }),
        },
        Err(e) => json!({ "success": false, "error": e.to_string() }),
    }
}

// Lightweight list (metadata + preview), newest first.
pub fn list(app: &AppHandle) -> Vec<Value> {
    let dir = history_dir(app);
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else { continue };
        let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };

        let full = v.get("fullText").and_then(|x| x.as_str()).unwrap_or("");
        let preview: String = full.chars().take(60).collect();
        let has_report = v
            .get("report")
            .and_then(|x| x.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        let pick = |k: &str| v.get(k).cloned().unwrap_or(json!(0));
        out.push(json!({
            "id": v.get("id").cloned().unwrap_or(Value::Null),
            "createdAt": v.get("createdAt").cloned().unwrap_or(Value::Null),
            "durationSec": pick("durationSec"),
            "totalWords": pick("totalWords"),
            "fillers": pick("fillers"),
            "hedges": pick("hedges"),
            "vagueWords": pick("vagueWords"),
            "preview": preview,
            "hasReport": has_report,
        }));
    }
    // ids are millisecond timestamps as strings → lexical desc == newest first
    out.sort_by(|a, b| {
        let ai = a.get("id").and_then(|x| x.as_str()).unwrap_or("");
        let bi = b.get("id").and_then(|x| x.as_str()).unwrap_or("");
        bi.cmp(ai)
    });
    out
}

pub fn get(app: &AppHandle, id: &str) -> Value {
    let Some(id) = sanitize_id(id) else {
        return Value::Null;
    };
    let path = history_dir(app).join(format!("{}.json", id));
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Value>(&text).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

pub fn delete(app: &AppHandle, id: &str) -> Value {
    let Some(id) = sanitize_id(id) else {
        return json!({ "success": false, "error": "无效的历史 id" });
    };
    let path = history_dir(app).join(format!("{}.json", id));
    let _ = fs::remove_file(&path);
    json!({ "success": true })
}
