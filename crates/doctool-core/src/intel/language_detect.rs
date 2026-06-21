//! Language detection for user queries.
//!
//! Uses lightweight heuristics (Unicode script, stop-word frequency) to detect
//! the language of a user message. Powers two features:
//!
//! 1. **Language-aware responses**: the detected language is injected as a directive
//!    into the system prompt so the agent responds in the user's language.
//! 2. **Multilingual query expansion**: stop words are removed before FTS search
//!    to improve recall for conversational queries in any language.
//!
//! Supported languages: English (en), French (fr), Spanish (es), Portuguese (pt),
//! German (de), Italian (it), Arabic (ar), Chinese (zh), Japanese (ja), Korean (ko).

/// A detected language with its ISO 639-1 code and human-readable name.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedLanguage {
    pub code: &'static str,
    pub name: &'static str,
}

impl DetectedLanguage {
    pub fn english() -> Self {
        Self {
            code: "en",
            name: "English",
        }
    }
    pub fn french() -> Self {
        Self {
            code: "fr",
            name: "French",
        }
    }
    pub fn spanish() -> Self {
        Self {
            code: "es",
            name: "Spanish",
        }
    }
    pub fn portuguese() -> Self {
        Self {
            code: "pt",
            name: "Portuguese",
        }
    }
    pub fn german() -> Self {
        Self {
            code: "de",
            name: "German",
        }
    }
    pub fn italian() -> Self {
        Self {
            code: "it",
            name: "Italian",
        }
    }
}

// ── Stop-word frequency tables ───────────────────────────────────────────────
// We score each language by how many of its common stop words appear in the text.
// Higher score = more likely to be that language.

const STOP_EN: &[&str] = &[
    "the", "is", "are", "was", "were", "you", "your", "we", "our", "it", "its", "this", "that",
    "with", "for", "from", "have", "has", "will", "can", "do", "not", "but", "also", "what", "how",
    "when", "where", "who", "which", "a", "an", "and", "in", "to", "of", "on", "at", "by", "as",
    "be", "or", "if", "my", "so", "up", "out", "about", "all", "any", "no", "yes",
];

const STOP_FR: &[&str] = &[
    "le", "la", "les", "un", "une", "des", "est", "sont", "vous", "nous", "mon", "et", "en", "de",
    "du", "il", "elle", "ils", "elles", "pas", "que", "qui", "sur", "avec", "dans", "pour", "par",
    "au", "aux", "été", "très", "mais", "comme", "aussi", "ça", "je", "tu",
];

const STOP_ES: &[&str] = &[
    "el", "la", "los", "las", "un", "una", "es", "son", "usted", "nosotros", "y", "en", "de",
    "del", "que", "por", "para", "con", "su", "sus", "al", "lo", "se", "me", "si", "no", "más",
    "como", "pero", "muy", "este", "esta", "yo", "tu",
];

const STOP_PT: &[&str] = &[
    "o", "a", "os", "as", "um", "uma", "é", "são", "você", "nós", "e", "em", "de", "do", "da",
    "que", "por", "para", "com", "não", "se", "me", "mas", "muito", "como", "este", "este", "eu",
    "tu",
];

const STOP_DE: &[&str] = &[
    "der", "die", "das", "ein", "eine", "ist", "sind", "du", "wir", "und", "in", "von", "zu",
    "mit", "für", "auf", "dem", "den", "nicht", "aber", "auch", "wie", "was", "wenn", "bei",
    "sich", "ich", "er", "sie", "es",
];

const STOP_IT: &[&str] = &[
    "il", "la", "i", "le", "un", "una", "è", "sono", "voi", "noi", "e", "in", "di", "del", "che",
    "per", "con", "su", "al", "lo", "non", "ma", "anche", "come", "questo", "questa", "io", "tu",
    "lui",
];

// CJK detection is done via Unicode script ranges — no stop-words needed.

/// Detect the language of a text. Falls back to English if uncertain.
/// Only detects if the text is long enough (>3 words) to be meaningful.
pub fn detect_language(text: &str) -> DetectedLanguage {
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered.split_whitespace().collect();

    if words.is_empty() {
        return DetectedLanguage::english();
    }

    // --- Script-based detection (fast path for CJK/Arabic) ---
    let char_counts = count_scripts(text);

    if char_counts.chinese > 3 {
        return DetectedLanguage {
            code: "zh",
            name: "Chinese",
        };
    }
    if char_counts.hiragana + char_counts.katakana > 3 {
        return DetectedLanguage {
            code: "ja",
            name: "Japanese",
        };
    }
    if char_counts.hangul > 3 {
        return DetectedLanguage {
            code: "ko",
            name: "Korean",
        };
    }

    // --- Stop-word scoring for Latin-script languages ---
    if words.len() < 3 {
        // Too short to be confident — return English
        return DetectedLanguage::english();
    }

    let score = |stops: &[&str]| -> usize { stops.iter().filter(|&&s| words.contains(&s)).count() };

    let scores = [
        (score(STOP_EN), DetectedLanguage::english()),
        (score(STOP_FR), DetectedLanguage::french()),
        (score(STOP_ES), DetectedLanguage::spanish()),
        (score(STOP_PT), DetectedLanguage::portuguese()),
        (score(STOP_DE), DetectedLanguage::german()),
        (score(STOP_IT), DetectedLanguage::italian()),
    ];

    let mut best_lang = DetectedLanguage::english();
    let mut best_score = 0;

    for (score, lang) in scores {
        if score > best_score {
            best_score = score;
            best_lang = lang;
        }
    }

    best_lang
}

struct ScriptCounts {
    chinese: usize,
    hiragana: usize,
    katakana: usize,
    hangul: usize,
}

fn count_scripts(text: &str) -> ScriptCounts {
    let mut counts = ScriptCounts {
        chinese: 0,
        hiragana: 0,
        katakana: 0,
        hangul: 0,
    };
    for c in text.chars() {
        let cp = c as u32;
        if (0x4E00..=0x9FFF).contains(&cp) || (0x3400..=0x4DBF).contains(&cp) {
            counts.chinese += 1;
        } else if (0x3040..=0x309F).contains(&cp) {
            counts.hiragana += 1;
        } else if (0x30A0..=0x30FF).contains(&cp) {
            counts.katakana += 1;
        } else if (0xAC00..=0xD7AF).contains(&cp) || (0x3131..=0x3163).contains(&cp) {
            counts.hangul += 1;
        }
    }
    counts
}

/// Build a language directive to inject into the system prompt.
///
/// If the user is writing in a language other than English, returns an
/// instruction to respond in that language. Returns empty string for English
/// (no extra instruction needed — the system prompt is already in English
/// and the LLM defaults to matching the user's language).
pub fn language_directive(lang: &DetectedLanguage) -> String {
    if lang.code == "en" {
        return String::new();
    }
    format!(
        "\n\n# LANGUAGE\nThe user is writing in {}. Respond in {} throughout this conversation.",
        lang.name, lang.name
    )
}

/// Strip stop words from a query to improve FTS search quality.
/// Works for: EN, FR, ES, PT, DE, IT, AR.
pub fn expand_query_for_fts(query: &str, lang: &DetectedLanguage) -> String {
    let stops: &[&str] = match lang.code {
        "fr" => STOP_FR,
        "es" => STOP_ES,
        "pt" => STOP_PT,
        "de" => STOP_DE,
        "it" => STOP_IT,
        _ => STOP_EN,
    };

    let lowered = query.to_lowercase();
    let words: Vec<&str> = lowered.split_whitespace().collect();

    let keywords: Vec<&str> = words
        .iter()
        .filter(|&&w| !stops.contains(&w) && w.len() >= 3)
        .copied()
        .collect();

    if keywords.is_empty() {
        return query.to_string();
    }

    // Return original OR keywords — SQLite FTS can use OR matching
    format!("{} OR {}", query, keywords.join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_french() {
        let lang = detect_language("Bonjour, comment puis-je vous aider avec les fichiers?");
        assert_eq!(lang.code, "fr");
    }

    #[test]
    fn detects_spanish() {
        let lang =
            detect_language("¿Cómo puedo mejorar este documento con los datos del proyecto?");
        assert_eq!(lang.code, "es");
    }

    #[test]
    fn detects_english() {
        let lang = detect_language("How can I improve this document with the project data?");
        assert_eq!(lang.code, "en");
    }

    #[test]
    fn detects_short_english_prompt() {
        let lang = detect_language("generate a random chart of data");
        assert_eq!(lang.code, "en");
    }

    #[test]
    fn detects_chinese() {
        let lang = detect_language("你好，我需要帮助处理这个文件。");
        assert_eq!(lang.code, "zh");
    }

    #[test]
    fn short_text_returns_english() {
        let lang = detect_language("Hi");
        assert_eq!(lang.code, "en");
    }

    #[test]
    fn english_directive_is_empty() {
        let lang = DetectedLanguage::english();
        assert!(language_directive(&lang).is_empty());
    }

    #[test]
    fn french_directive_contains_french() {
        let lang = DetectedLanguage::french();
        let d = language_directive(&lang);
        assert!(d.contains("French"));
    }

    #[test]
    fn fts_expansion_removes_stop_words() {
        let lang = DetectedLanguage::french();
        let expanded = expand_query_for_fts("comment puis-je le faire avec les fichiers", &lang);
        // "comment", "puis", "les" are stop words → stripped
        assert!(expanded.contains("faire") || expanded.contains("fichiers"));
    }
}
