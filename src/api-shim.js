// Bridges the legacy Electron `window.api` surface onto Tauri commands.
// Loaded before app.js / settings.js / the prompt-editor inline script so the
// existing frontend code runs unchanged on Tauri.

(function () {
  const invoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
  if (!invoke) {
    console.error('[api-shim] Tauri invoke unavailable — is withGlobalTauri enabled?');
    return;
  }

  window.api = {
    // Settings
    getSettings: () => invoke('get_settings'),
    saveSettings: (settings) => invoke('save_settings', { settings }),
    openSettings: () => invoke('open_settings'),

    // Prompt editor
    openPromptEditor: () => invoke('open_prompt_editor'),
    getCustomPrompt: () => invoke('get_custom_prompt'),
    saveCustomPrompt: (data) => invoke('save_custom_prompt', { data }),
    closeWindow: () => invoke('close_current_window'),

    // ASR (return-value streaming model, same as the Electron preload)
    initASR: () => invoke('init_asr'),
    feedAudio: (samples) => invoke('feed_audio', { samples: Array.from(samples) }),
    stopASR: () => invoke('stop_asr'),
    onASRResult: () => {},
    removeASRListener: () => {},

    // Lexicon
    analyzeText: (text) => invoke('analyze_text', { text }),

    // AI feedback
    getRealtimeFeedback: (text) => invoke('get_realtime_feedback', { text }),
    getFinalReport: (data) => invoke('get_final_report', { args: data }),
    testLLMConnection: (settings) => invoke('test_llm_connection', { settings }),

    // Training history
    saveHistory: (entry) => invoke('save_history', { entry }),
    listHistory: () => invoke('list_history'),
    getHistory: (id) => invoke('get_history', { id }),
    deleteHistory: (id) => invoke('delete_history', { id }),

    // File save
    saveFile: (content, filename) => invoke('save_file', { content, filename }),
  };
})();

// Window dragging: WKWebView ignores `-webkit-app-region: drag`, so we replicate
// the Electron "drag from anywhere except interactive areas" behaviour by driving
// Tauri's startDragging. The exclusion list mirrors the no-drag CSS rules.
(function setupWindowDrag() {
  const tauriWindow = window.__TAURI__ && window.__TAURI__.window;
  if (!tauriWindow || !tauriWindow.getCurrentWindow) return;

  const NO_DRAG =
    'button, input, select, textarea, a, label, [data-no-drag], ' +
    '.subtitle-scroll, .feedback-content, .modal-body';

  document.addEventListener('mousedown', (e) => {
    if (e.button !== 0) return; // left button only
    if (e.target.closest && e.target.closest(NO_DRAG)) return;
    tauriWindow.getCurrentWindow().startDragging().catch(() => {});
  });
})();
