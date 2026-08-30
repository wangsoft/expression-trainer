// Streaming ASR engine wrapping sherpa-onnx's OnlineRecognizer.
// Mirrors the Electron `lib/asr.js` behaviour (feed frames, endpoint reset).
//
// OnlineRecognizer / OnlineStream are marked Send+Sync by the crate, so we can
// keep them in Tauri-managed state behind a Mutex — no dedicated audio thread.

use serde_json::{json, Value};
use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const MODEL_SUBDIR: &str = "sherpa-onnx-streaming-paraformer-bilingual-zh-en";
const SAMPLE_RATE: i32 = 16000;

struct Engine {
    recognizer: OnlineRecognizer,
    stream: Option<OnlineStream>,
    running: bool,
}

#[derive(Default)]
pub struct AsrState {
    engine: Mutex<Option<Engine>>,
}

// A detected streaming-paraformer model: encoder + decoder file names.
// int8 quantised variants are preferred, full-precision used as fallback.
fn detect_paraformer(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    if !dir.join("tokens.txt").exists() {
        return None;
    }
    let pick = |int8: &str, fp32: &str| -> Option<PathBuf> {
        let a = dir.join(int8);
        let b = dir.join(fp32);
        if a.exists() {
            Some(a)
        } else if b.exists() {
            Some(b)
        } else {
            None
        }
    };
    let encoder = pick("encoder.int8.onnx", "encoder.onnx")?;
    let decoder = pick("decoder.int8.onnx", "decoder.onnx")?;
    Some((encoder, decoder))
}

// Resolve the model directory: the bundled resource dir (packaged app) first,
// then dev-time relative paths. The int8 model ships inside the app bundle.
fn resolve_model_dir(app: &AppHandle) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join("models").join(MODEL_SUBDIR));
    }
    // dev: cwd is src-tauri when running `tauri dev`
    candidates.push(PathBuf::from("..").join("models").join(MODEL_SUBDIR));
    candidates.push(PathBuf::from("models").join(MODEL_SUBDIR));

    candidates.into_iter().find(|d| detect_paraformer(d).is_some())
}

fn build_config(dir: &Path) -> Option<OnlineRecognizerConfig> {
    let (encoder, decoder) = detect_paraformer(dir)?;
    let s = |p: PathBuf| p.to_string_lossy().into_owned();
    let mut config = OnlineRecognizerConfig::default();
    config.model_config.paraformer.encoder = Some(s(encoder));
    config.model_config.paraformer.decoder = Some(s(decoder));
    config.model_config.tokens = Some(dir.join("tokens.txt").to_string_lossy().into_owned());
    config.model_config.provider = Some("cpu".to_string());
    config.model_config.num_threads = 2;
    config.enable_endpoint = true;
    config.decoding_method = Some("greedy_search".to_string());
    Some(config)
}

impl AsrState {
    // Returns {success, error?} to match the Electron init-asr handler.
    pub fn init(&self, app: &AppHandle) -> Value {
        let mut guard = self.engine.lock().unwrap();

        // Reuse an existing recognizer, just start a fresh stream (like asr.js).
        if let Some(engine) = guard.as_mut() {
            engine.stream = Some(engine.recognizer.create_stream());
            engine.running = true;
            return json!({ "success": true });
        }

        let Some(dir) = resolve_model_dir(app) else {
            return json!({
                "success": false,
                "error": "未找到内置语音模型（应用包内 models 缺失）"
            });
        };

        let Some(config) = build_config(&dir) else {
            return json!({ "success": false, "error": "模型目录缺少 encoder/decoder/tokens 文件" });
        };
        let recognizer = match OnlineRecognizer::create(&config) {
            Some(r) => r,
            None => {
                return json!({ "success": false, "error": "识别引擎初始化失败（模型加载失败）" })
            }
        };
        let stream = recognizer.create_stream();
        *guard = Some(Engine {
            recognizer,
            stream: Some(stream),
            running: true,
        });
        json!({ "success": true })
    }

    // Feed one chunk of 16kHz mono samples; returns {text, isFinal} or null.
    pub fn feed(&self, samples: Vec<f32>) -> Option<Value> {
        let mut guard = self.engine.lock().unwrap();
        let engine = guard.as_mut()?;
        if !engine.running {
            return None;
        }
        let stream = engine.stream.as_ref()?;

        stream.accept_waveform(SAMPLE_RATE, &samples);
        while engine.recognizer.is_ready(stream) {
            engine.recognizer.decode(stream);
        }

        let text = engine
            .recognizer
            .get_result(stream)
            .map(|r| r.text.trim().to_string())
            .unwrap_or_default();
        let is_endpoint = engine.recognizer.is_endpoint(stream);

        if is_endpoint && !text.is_empty() {
            engine.recognizer.reset(stream);
            Some(json!({ "text": text, "isFinal": true }))
        } else if !text.is_empty() {
            Some(json!({ "text": text, "isFinal": false }))
        } else {
            None
        }
    }

    // Flush the tail and return the last unconfirmed text.
    pub fn stop(&self) -> String {
        let mut guard = self.engine.lock().unwrap();
        let Some(engine) = guard.as_mut() else {
            return String::new();
        };
        engine.running = false;

        let final_text = if let Some(stream) = engine.stream.as_ref() {
            stream.input_finished();
            while engine.recognizer.is_ready(stream) {
                engine.recognizer.decode(stream);
            }
            engine
                .recognizer
                .get_result(stream)
                .map(|r| r.text.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        engine.stream = None; // keep recognizer for cheap re-init
        final_text
    }
}
