//! Approximate token estimation.
//!
//! A deterministic heuristic based on the widely used ~4 characters per token
//! approximation. It is deliberately model-agnostic and cheap; exact counts
//! belong to a tokenizer, not an optimizer.

/// Approximate the number of tokens in `text`.
///
/// Uses `ceil(chars / 4)` with a floor of one token, which closely matches
/// common LLM tokenizer behavior for English and source code.
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    estimate_tokens_from_chars(chars)
}

/// Approximate tokens from a character count.
pub fn estimate_tokens_from_chars(chars: usize) -> usize {
    (chars as f64 / 4.0).ceil().max(1.0) as usize
}

/// Approximate tokens for a byte count, assuming roughly one char per byte.
pub fn estimate_tokens_from_bytes(bytes: u64) -> usize {
    estimate_tokens_from_chars(bytes as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_chars_per_token() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 / 4 -> 1.25 -> ceil 2
        assert_eq!(estimate_tokens("hello world"), 3); // 11 / 4 -> 2.75 -> ceil 3
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens_from_bytes(8), 2);
    }
}
