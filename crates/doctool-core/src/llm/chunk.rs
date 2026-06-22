use super::prompt::TranslatePromptOptions;
use super::tokeniser::estimate_tokens_for_content;

const MAX_INPUT_TOKENS: usize = 128_000;
const MAX_OUTPUT_TOKENS: usize = 16_000;
const MIN_CHUNK_SIZE: usize = 1;
const MAX_CHUNK_SIZE: usize = 100;

/// Port of Languine `calculateChunkSize`.
pub fn calculate_chunk_size(
    content: &[(String, String)],
    options: Option<&TranslatePromptOptions>,
) -> usize {
    if content.is_empty() {
        return MIN_CHUNK_SIZE;
    }

    let estimated_tokens = estimate_tokens_for_content(content, options).max(1);
    let budget = MAX_INPUT_TOKENS.saturating_sub(MAX_OUTPUT_TOKENS);
    let items_per_chunk = ((budget / estimated_tokens) * content.len()).max(MIN_CHUNK_SIZE);

    items_per_chunk.min(MAX_CHUNK_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_min() {
        assert_eq!(calculate_chunk_size(&[], None), 1);
    }

    #[test]
    fn small_content_fits_many() {
        let items: Vec<_> = (0..5)
            .map(|i| (format!("body:{i}"), "Short text.".into()))
            .collect();
        let size = calculate_chunk_size(&items, None);
        assert!(size >= 1 && size <= 100);
    }
}
