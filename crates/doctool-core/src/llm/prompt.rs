#[derive(Debug, Clone)]
pub struct TranslatePromptOptions {
    pub source_locale: String,
    pub target_locale: String,
}

#[derive(Debug, Clone)]
pub struct ImprovePromptInput {
    pub page_path: String,
    pub current_content: String,
    pub style_guide: String,
    pub openapi_context: String,
    pub code_context: String,
    pub competitor_context: String,
}

#[derive(Debug, Clone)]
pub struct SuggestLlmInput {
    pub drift_summary: String,
    pub next_steps: Vec<String>,
    pub code_context: String,
    pub knowledge_graph_stats: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SuggestRecommendation {
    pub priority: u8,
    pub title: String,
    pub command: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SuggestLlmPlan {
    pub summary: String,
    pub recommendations: Vec<SuggestRecommendation>,
}

fn language_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "fr" => "French",
        "es" => "Spanish",
        "zh" => "Chinese",
        _ => code,
    }
}

const MDX_TRANSLATE_REQUIREMENTS: &str = r#"Translation Requirements:
- Maintain exact MDX structure, indentation, and formatting
- Provide natural, culturally-adapted translations that sound native
- Never translate or change: method, path, operationId frontmatter values
- Preserve code fences, JSX components, and string literals inside code blocks exactly
- Keep internal link paths unchanged (e.g. /build/..., /api/...)
- Keep consistent capitalization, spacing, and line breaks
- Never change the order of segments
- Return one translated string per input segment, in the same order"#;

const MDX_SPECIFIC: &str = r#"MDX Specific Instructions:
- Preserve all Markdown formatting and syntax in prose and headings
- Do not translate content inside ``` fenced code blocks
- Do not translate JSX attribute values that are URLs or API paths
- Translate title and description frontmatter values naturally"#;

pub fn create_translate_prompt(
    content: &[(String, String)],
    options: &TranslatePromptOptions,
) -> String {
    let payload: Vec<serde_json::Value> = content
        .iter()
        .map(|(key, text)| serde_json::json!({ "key": key, "sourceText": text }))
        .collect();

    format!(
        "You are a professional translator working with MDX documentation files.\n\n\
         Task: Translate the content below from {} ({}) to {} ({}).\n\n\
         {}\n\
         {}\n\n\
         Respond with JSON only: {{\"translatedKeys\": [\"...\", ...]}} — one string per input segment, same order.\n\n\
         Content:\n{}",
        language_name(&options.source_locale),
        options.source_locale,
        language_name(&options.target_locale),
        options.target_locale,
        MDX_TRANSLATE_REQUIREMENTS,
        MDX_SPECIFIC,
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    )
}

pub fn create_improve_prompt(input: &ImprovePromptInput) -> String {
    format!(
        "You are a technical documentation editor for lomi., a payments platform for Africa.\n\n\
         Task: Improve the MDX page prose while preserving all factual API contracts.\n\n\
         Hard rules (never violate):\n\
         - Do not invent API paths, methods, or SDK methods not in the context below\n\
         - Do not mention internal provider ingress or banned infrastructure terms\n\
         - Preserve frontmatter keys method, path, operationId exactly when present\n\
         - Preserve code blocks and JSX verbatim\n\
         - Keep internal links as relative paths (/start/, /build/, /api/, /resources/)\n\
         - Output complete improved MDX only (no commentary)\n\n\
         Style guide:\n{}\n\n\
         OpenAPI / contract context:\n{}\n\n\
         Related code context:\n{}\n\n\
         Competitor style reference (tone/structure only, not facts):\n{}\n\n\
         Page path: {}\n\n\
         Current MDX:\n{}",
        input.style_guide,
        input.openapi_context,
        input.code_context,
        input.competitor_context,
        input.page_path,
        input.current_content
    )
}

pub fn create_suggest_prompt(input: &SuggestLlmInput) -> String {
    let steps = if input.next_steps.is_empty() {
        "(none)".into()
    } else {
        input
            .next_steps
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "You are a documentation maintainer for lomi., a payments platform.\n\n\
         Task: Given drift findings and code context, produce a prioritized maintenance plan.\n\n\
         Rules:\n\
         - Prefer executable `dt` / `pnpm` commands already listed when they apply\n\
         - Do not invent API paths or SDK methods not supported by the code context\n\
         - Separate deterministic fixes (scaffold, regenerate manifests) from prose/i18n work\n\
         - Keep recommendations actionable for a single PR cycle\n\n\
         Drift summary by category: {}\n\n\
         Deterministic next steps already detected:\n{}\n\n\
         Knowledge graph: {}\n\n\
         Relevant code snippets:\n{}\n\n\
         Respond with JSON only:\n\
         {{\"summary\": \"2-4 sentence overview\", \"recommendations\": [\
         {{\"priority\": 1, \"title\": \"short title\", \"command\": \"optional shell command\", \"rationale\": \"why\"}}\
         ]}}",
        input.drift_summary,
        steps,
        input.knowledge_graph_stats,
        input.code_context
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_prompt_includes_locales() {
        let opts = TranslatePromptOptions {
            source_locale: "en".into(),
            target_locale: "fr".into(),
        };
        let prompt = create_translate_prompt(&[("body:0".into(), "Hello".into())], &opts);
        assert!(prompt.contains("English"));
        assert!(prompt.contains("French"));
        assert!(prompt.contains("operationId"));
    }
}
