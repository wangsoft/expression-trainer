// Chinese text analysis: fillers / hedges / vague words / emotion words.
// Ported 1:1 from the Electron `lib/lexicon.js` so output shape is identical.

use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::OnceLock;

// The emotion lexicon (146 emotion words) is embedded at compile time so the
// binary has no runtime dependency on the data/ folder.
const EMOTION_LEXICON: &str = include_str!("../../data/emotion-lexicon.json");

const FILLER_WORDS: &[&str] = &[
    "嗯", "啊", "呃", "额", "那个", "就是", "然后", "这个", "对吧", "是吧", "你知道",
    "怎么说呢", "反正", "基本上", "总之", "所以说",
];

const HEDGE_WORDS: &[&str] = &[
    "可能", "也许", "大概", "应该", "我觉得", "好像", "似乎", "或许", "不一定", "差不多",
    "算是", "某种程度上", "一般来说", "感觉",
];

fn vague_to_precise() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        ("开心", &["欣喜", "雀跃", "兴奋", "欣慰", "畅快", "满足"]),
        ("难过", &["心酸", "失落", "委屈", "心疼", "沮丧", "低落"]),
        ("害怕", &["恐惧", "焦虑", "不安", "慌张", "胆怯", "忐忑"]),
        ("生气", &["愤怒", "恼火", "窝火", "气愤", "不满", "暴躁"]),
        ("不舒服", &["压抑", "烦躁", "憋屈", "窒息", "煎熬", "疲惫"]),
        ("很好", &["出色", "精彩", "优秀", "惊艳", "完美", "理想"]),
        ("很多", &["大量", "海量", "充裕", "丰富", "密集", "可观"]),
        ("很快", &["迅速", "飞速", "立刻", "瞬间", "即刻", "火速"]),
        ("很大", &["巨大", "庞大", "显著", "惊人", "可观", "壮观"]),
        ("很小", &["微小", "细微", "轻微", "渺小", "微不足道", "些许"]),
        ("好看", &["精致", "优雅", "绚丽", "惊艳", "别致", "夺目"]),
        ("不好", &["糟糕", "恶劣", "拙劣", "不堪", "惨淡", "低劣"]),
        ("喜欢", &["热爱", "痴迷", "着迷", "钟爱", "倾心", "沉醉"]),
        ("讨厌", &["厌恶", "反感", "排斥", "憎恨", "鄙视", "嫌弃"]),
        ("觉得", &["认为", "判断", "确信", "推断", "意识到", "发现"]),
        ("想", &["渴望", "期待", "向往", "盼望", "企图", "打算"]),
        ("做", &["执行", "落实", "推进", "完成", "实施", "操作"]),
        ("看", &["审视", "观察", "注视", "打量", "端详", "凝视"]),
        ("说", &["表达", "阐述", "强调", "指出", "坦言", "声明"]),
        ("想想", &["反思", "回顾", "审视", "复盘", "琢磨", "斟酌"]),
    ]
}

fn emotions() -> &'static Value {
    static EMOTIONS: OnceLock<Value> = OnceLock::new();
    EMOTIONS.get_or_init(|| {
        let root: Value = serde_json::from_str(EMOTION_LEXICON).unwrap_or(json!({}));
        root.get("emotions").cloned().unwrap_or(json!({}))
    })
}

fn vague_lookup(word: &str) -> Option<&'static [&'static str]> {
    vague_to_precise()
        .iter()
        .find(|(w, _)| *w == word)
        .map(|(_, alts)| *alts)
}

// Forward-maximum-matching segmentation against the combined dictionary.
fn segment_text(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut dict: HashSet<String> = HashSet::new();
    for w in FILLER_WORDS.iter().chain(HEDGE_WORDS.iter()) {
        dict.insert((*w).to_string());
    }
    for (w, _) in vague_to_precise() {
        dict.insert((*w).to_string());
    }
    if let Some(map) = emotions().as_object() {
        for k in map.keys() {
            dict.insert(k.clone());
        }
    }

    let mut words = Vec::new();
    let mut i = 0;
    let max_len = 6;
    while i < chars.len() {
        let mut matched = false;
        let upper = max_len.min(chars.len() - i);
        for len in (2..=upper).rev() {
            let candidate: String = chars[i..i + len].iter().collect();
            if dict.contains(&candidate) {
                words.push(candidate);
                i += len;
                matched = true;
                break;
            }
        }
        if !matched {
            words.push(chars[i].to_string());
            i += 1;
        }
    }
    words
}

pub fn analyze_text(text: &str) -> Option<Value> {
    if text.trim().is_empty() {
        return None;
    }

    let words = segment_text(text);
    let total_words = words.len();

    let mut fillers = Vec::new();
    let mut hedges = Vec::new();
    let mut vague_words = Vec::new();
    let mut emotion_words = Vec::new();

    for (idx, word) in words.iter().enumerate() {
        if FILLER_WORDS.contains(&word.as_str()) {
            fillers.push(json!({ "word": word, "position": idx }));
        }
        if HEDGE_WORDS.contains(&word.as_str()) {
            hedges.push(json!({ "word": word, "position": idx }));
        }
        if let Some(alts) = vague_lookup(word) {
            vague_words.push(json!({ "word": word, "position": idx, "alternatives": alts }));
        }
        if let Some(info) = emotions().get(word) {
            let mut obj = serde_json::Map::new();
            obj.insert("word".into(), json!(word));
            obj.insert("position".into(), json!(idx));
            if let Some(map) = info.as_object() {
                for (k, v) in map {
                    obj.insert(k.clone(), v.clone());
                }
            }
            emotion_words.push(Value::Object(obj));
        }
    }

    let meaningful = total_words as i64 - fillers.len() as i64 - hedges.len() as i64;
    let density = if total_words > 0 {
        (meaningful as f64 / total_words as f64 * 100.0).round() as i64
    } else {
        100
    };

    let suggestions = generate_suggestions(&vague_words, &fillers, &hedges);

    Some(json!({
        "totalWords": total_words,
        "fillers": fillers,
        "hedges": hedges,
        "vagueWords": vague_words,
        "emotionWords": emotion_words,
        "density": density,
        "suggestions": suggestions,
    }))
}

fn generate_suggestions(vague: &[Value], fillers: &[Value], hedges: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();

    for item in vague {
        let word = item["word"].as_str().unwrap_or("");
        let alts: Vec<&str> = item["alternatives"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).take(3).collect())
            .unwrap_or_default();
        out.push(json!({
            "type": "vague",
            "original": word,
            "alternatives": alts,
            "message": format!("「{}」→ 试试更精准的：{}", word, alts.join("、")),
        }));
    }

    if fillers.len() >= 3 {
        let mut seen = Vec::new();
        for f in fillers {
            if let Some(w) = f["word"].as_str() {
                if !seen.contains(&w) {
                    seen.push(w);
                }
            }
        }
        let top: Vec<&str> = seen.into_iter().take(3).collect();
        out.push(json!({
            "type": "filler",
            "message": format!("填充词偏多（{}次）：{}。试试用停顿替代", fillers.len(), top.join("、")),
        }));
    }

    if hedges.len() >= 2 {
        out.push(json!({
            "type": "hedge",
            "message": format!("犹豫表达较多（{}次）。试试把「我觉得」改成直接陈述", hedges.len()),
        }));
    }

    out
}
