use super::prompt::{create_translate_prompt, TranslatePromptOptions};

/// Rough token estimate: chars / 4 plus prompt overhead.
pub fn estimate_tokens_for_content(
    content: &[(String, String)],
    options: Option<&TranslatePromptOptions>,
) -> usize {
    let content_tokens: usize = content
        .iter()
        .map(|(_, text)| (text.chars().count() / 4).max(1))
        .sum();

    let prompt_tokens = options
        .map(|opts| {
            let prompt = create_translate_prompt(content, opts);
            (prompt.len() / 4).max(1)
        })
        .unwrap_or(0);

    content_tokens + prompt_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_nonzero_for_content() {
        let items = vec![("body:0".into(), "Hello world".into())];
        assert!(estimate_tokens_for_content(&items, None) > 0);
    }
}
