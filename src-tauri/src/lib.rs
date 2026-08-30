// Tauri application entry. Backend commands mirror the Electron `window.api`
// surface so the existing frontend works through a thin api-shim.js.

mod asr;
mod history;
mod lexicon;
mod llm;
mod prompts;
mod settings;

use asr::AsrState;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Window};
use tauri_plugin_dialog::DialogExt;

// ===== Settings =====

#[tauri::command]
fn get_settings(app: AppHandle) -> Value {
    settings::load_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Value) -> Value {
    match settings::save_settings(&app, &settings) {
        Ok(()) => json!({ "success": true }),
        Err(e) => json!({ "success": false, "error": e }),
    }
}

#[tauri::command]
fn get_custom_prompt(app: AppHandle) -> Value {
    settings::load_custom_prompt(&app).unwrap_or(Value::Null)
}

#[tauri::command]
fn save_custom_prompt(app: AppHandle, data: Value) -> Value {
    match settings::save_custom_prompt(&app, &data) {
        Ok(()) => json!({ "success": true }),
        Err(e) => json!({ "success": false, "error": e }),
    }
}

// ===== Windows =====

fn open_child(app: &AppHandle, label: &str, file: &str, title: &str, w: f64, h: f64, resizable: bool) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, label, WebviewUrl::App(file.into()))
        .title(title)
        .inner_size(w, h)
        .resizable(resizable)
        .build();
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    open_child(&app, "settings", "settings.html", "设置", 600.0, 520.0, true);
}

#[tauri::command]
fn open_prompt_editor(app: AppHandle) {
    open_child(&app, "prompt-editor", "prompt-editor.html", "Prompt 编辑器", 720.0, 700.0, true);
}

#[tauri::command]
fn close_current_window(window: Window) {
    let _ = window.close();
}

// ===== ASR =====

#[tauri::command]
fn init_asr(app: AppHandle, state: tauri::State<AsrState>) -> Value {
    state.init(&app)
}

#[tauri::command]
fn feed_audio(state: tauri::State<AsrState>, samples: Vec<f32>) -> Option<Value> {
    state.feed(samples)
}

#[tauri::command]
fn stop_asr(state: tauri::State<AsrState>) -> Value {
    let final_text = state.stop();
    json!({ "success": true, "finalText": final_text })
}

// ===== Lexicon =====

#[tauri::command]
fn analyze_text(text: String) -> Option<Value> {
    lexicon::analyze_text(&text)
}

// ===== LLM =====

#[tauri::command]
async fn test_llm_connection(settings: Value) -> Value {
    llm::test_connection(&settings).await
}

#[tauri::command]
async fn get_realtime_feedback(app: AppHandle, text: String) -> Value {
    let settings = settings::load_settings(&app);
    let custom = settings::load_custom_prompt(&app);
    match llm::send_feedback(&text, &settings, custom.as_ref()).await {
        Ok(feedback) => json!({ "success": true, "feedback": feedback }),
        Err(e) => json!({ "success": false, "error": e }),
    }
}

#[derive(serde::Deserialize)]
struct ReportArgs {
    #[serde(rename = "fullText")]
    full_text: String,
    stats: Value,
}

#[tauri::command]
async fn get_final_report(app: AppHandle, args: ReportArgs) -> Value {
    let settings = settings::load_settings(&app);
    let custom = settings::load_custom_prompt(&app);
    match llm::send_report(&args.full_text, &args.stats, &settings, custom.as_ref()).await {
        Ok(report) => json!({ "success": true, "report": report }),
        Err(e) => json!({ "success": false, "error": e }),
    }
}

// ===== History =====

#[tauri::command]
fn save_history(app: AppHandle, entry: Value) -> Value {
    history::save(&app, &entry)
}

#[tauri::command]
fn list_history(app: AppHandle) -> Vec<Value> {
    history::list(&app)
}

#[tauri::command]
fn get_history(app: AppHandle, id: String) -> Value {
    history::get(&app, &id)
}

#[tauri::command]
fn delete_history(app: AppHandle, id: String) -> Value {
    history::delete(&app, &id)
}

// ===== File save =====

#[tauri::command]
fn save_file(app: AppHandle, content: String, filename: String) -> Value {
    let mut builder = app
        .dialog()
        .file()
        .set_title("保存报告")
        .set_file_name(&filename)
        .add_filter("Markdown", &["md"]);
    if let Ok(desktop) = app.path().desktop_dir() {
        builder = builder.set_directory(desktop);
    }

    match builder.blocking_save_file() {
        Some(path) => match path.into_path() {
            Ok(p) => match std::fs::write(&p, content) {
                Ok(()) => json!({ "success": true, "path": p.to_string_lossy() }),
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            },
            Err(e) => json!({ "success": false, "error": e.to_string() }),
        },
        None => json!({ "success": false }),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AsrState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_custom_prompt,
            save_custom_prompt,
            open_settings,
            open_prompt_editor,
            close_current_window,
            init_asr,
            feed_audio,
            stop_asr,
            analyze_text,
            test_llm_connection,
            get_realtime_feedback,
            get_final_report,
            save_history,
            list_history,
            get_history,
            delete_history,
            save_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
