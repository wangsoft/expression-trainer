# 🚀 宇宙无敌表达训练系统 - 本地桌面版

一个帮你训练口语表达精准度的本地桌面应用。实时语音识别 → 词库匹配 → AI 反馈。语音识别与词库分析完全离线本地处理，AI 反馈按需联网。

桌面端已从 Electron 迁移到 **Tauri v2**（Rust 后端 + 系统 WebView），安装包更小、启动更快，并内置语音模型开箱即用。

## ⬇️ 下载安装（推荐）

前往 **[Releases](https://github.com/wangsoft/expression-trainer/releases/latest)** 下载最新 `.dmg`：

- 适用于 **macOS 11+ / Apple Silicon (arm64)**
- **内置 int8 语音模型，无需额外下载**，装好即用
- 打开 `.dmg` → 把「宇宙无敌表达训练」拖入「应用程序」
- ⚠️ **必看**：应用未签名/未公证，从网上下载的 dmg 会被 macOS 打上隔离标记，直接双击通常会提示 **“应用已损坏，无法打开”**。装好后在终端执行**一次**下面的命令即可正常使用：
  ```bash
  xattr -cr "/Applications/宇宙无敌表达训练.app"
  ```
  （执行后正常双击打开即可；首次也可尝试右键 →「打开」。）

> 需要其它平台/架构（Intel Mac、Windows、Linux）请自行[从源码构建](#从源码构建)。

## 功能

- 🎙️ **实时语音识别**：基于 [Sherpa-ONNX](https://github.com/k2-fsa/sherpa-onnx) 流式 paraformer 中英双语模型，完全离线
- 🖥️ **全屏字幕显示**：黑底大字，实时显示你说的每一句话
- 🔍 **词库分析**：自动检测填充词、犹豫词、笼统词，给出精准替代
- 🤖 **AI 反馈**：支持 OpenAI / DeepSeek / Ollama / 自定义 OpenAI 兼容接口
- 📊 **分析报告**：多维度深度分析（逻辑 / 直接性 / 填充词 / 密度 / 词汇 / 亮点）
- 🕘 **训练历史**：每次训练自动存档（原文 + 统计 + 报告），可浏览、重看、删除
- 📋 **逐字稿模式**：粘贴已有文字直接分析，无需录音

## 使用说明

1. **点击「开始录制」** → 首次会请求麦克风权限，允许后对着麦克风说话
2. **实时字幕**在屏幕中央显示你说的内容，并高亮词类
3. **左侧面板**实时统计填充词 / 犹豫词 / 笼统词 / 表达密度
4. **右侧面板**给出 AI 实时反馈（需在设置里填好 LLM 的 API Key）
5. **说完点「结束」** → 点「生成报告」获取完整分析
6. 点顶栏 🕘 **历史**图标查看过往训练记录

### 配置 AI 后端

点击右上角设置图标进入设置页：

| 后端 | 费用 | 速度 | 获取方式 |
|------|------|------|----------|
| DeepSeek（推荐） | 极低 | 快 | [platform.deepseek.com](https://platform.deepseek.com) |
| OpenAI | 中等 | 快 | [platform.openai.com](https://platform.openai.com) |
| Ollama | 免费 | 取决于硬件 | [ollama.com](https://ollama.com) 本地运行 |
| 自定义 | 视服务商 | — | 任意 OpenAI 兼容 Base URL |

**推荐 DeepSeek**：生成报告质量高，成本极低。

## 字幕颜色含义

| 颜色 | 含义 |
|------|------|
| 🔴 红色波浪下划线 | 填充词（嗯、啊、那个、然后…） |
| 🟠 橙色 | 犹豫词（可能、也许、我觉得…） |
| 🟡 黄色虚线 | 笼统词（有精准替代建议） |
| 🟢 绿色 | 有力表达（好句子！） |

## 从源码构建

需要 **Node.js 20+**、**Rust**（稳定版）、**Xcode Command Line Tools**。

```bash
# 1) 安装前端依赖 + Tauri CLI
npm install

# 2) 下载语音模型到 models/（打包会内置，dev 运行也用它）
cd models
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-paraformer-bilingual-zh-en.tar.bz2
tar xvf sherpa-onnx-streaming-paraformer-bilingual-zh-en.tar.bz2
cd ..

# 3) 开发模式（热重载）
npm run tauri:dev

# 4) 打包 .dmg（内置 int8 模型）
npm run tauri:build
# 产物在 src-tauri/target/release/bundle/
```

下载后 `models/` 目录应包含（只用到 int8 版本）：

```
models/
└── sherpa-onnx-streaming-paraformer-bilingual-zh-en/
    ├── encoder.int8.onnx
    ├── decoder.int8.onnx
    └── tokens.txt
```

> ⚠️ 用 `npm run tauri:dev` 从终端启动时，麦克风可能因 macOS TCC 责任进程归属而受限（报 “not allowed by the user agent”）。**验证麦克风请用打包后的 `.app`**——它有独立应用身份，可正常授权。

## 技术架构

```
┌──────────────────────────────────────────────┐
│ Tauri v2 后端 (Rust，静态链接 sherpa-onnx)     │
│  ├── asr.rs      流式语音识别 (OnlineRecognizer)│
│  ├── lexicon.rs  词库匹配 (emotion-lexicon)     │
│  ├── llm.rs      AI 反馈 (reqwest, 多后端)      │
│  ├── settings.rs 设置持久化                     │
│  └── history.rs  训练历史                       │
├──────────────────────────────────────────────┤
│ 前端 (系统 WebView)                            │
│  ├── 全屏字幕 / 实时统计 / 反馈面板            │
│  ├── 报告 & 历史弹窗                           │
│  └── api-shim.js  (window.api → Tauri invoke)  │
└──────────────────────────────────────────────┘
```

语音识别使用官方 [`sherpa-onnx`](https://crates.io/crates/sherpa-onnx) Rust crate，静态链接进可执行文件，无需附带动态库。

## 词库说明

`data/emotion-lexicon.json` 基于大连理工情感词库 7 大类结构，包含：

- **130+ 情绪词**：分类（喜怒哀惧恶惊）+ 强度（1-9）
- **笼统词 → 精准词映射**：25 组高频替代建议
- **填充词表**：24 个常见口头禅
- **犹豫词表**：19 个弱化表达
- **程度词梯度**：弱 → 中 → 强 → 极 四级
- **画面化描述**：10 组「抽象 → 具象」转换
- **犹豫 → 直接转换**：8 组对照示例

## 目录结构

```
├── src/                    # 前端（HTML/CSS/JS）
│   ├── index.html          # 主界面
│   ├── api-shim.js         # window.api → Tauri invoke 桥接
│   ├── app.js  styles.css  # 前端逻辑 / 样式
│   ├── settings.html/.js   # 设置页
│   └── prompt-editor.html  # 训练规则定制
├── src-tauri/              # Tauri v2 + Rust 后端
│   ├── src/                # asr / llm / lexicon / prompts / settings / history / lib
│   ├── tauri.conf.json     # 应用配置（内置模型、窗口等）
│   ├── Cargo.toml
│   └── icons/              # 应用图标
├── data/emotion-lexicon.json
├── models/                 # sherpa-onnx int8 模型（发行版已内置；源码构建需下载）
└── main.js  preload.js  lib/   # 旧 Electron 版（保留，非主线）
```

## 系统要求

- **发行版 dmg**：macOS 11+，Apple Silicon (arm64)
- **从源码构建**：Node.js 20+、Rust 稳定版、Xcode Command Line Tools
- 麦克风权限
- （可选）网络连接：仅 AI 反馈需要，语音识别与词库分析可离线

## License

MIT
