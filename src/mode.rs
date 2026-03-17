use crate::cli::Mode;

const TOKEN_THRESHOLD: usize = 30_000;

pub fn detect_mode(forced: Option<Mode>, estimated_tokens: usize) -> Mode {
    if let Some(mode) = forced {
        return mode;
    }
    if estimated_tokens >= TOKEN_THRESHOLD {
        Mode::Book
    } else {
        Mode::Article
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    (word_count as f64 * 1.3) as usize
}

pub fn is_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Mode;

    #[test]
    fn test_small_token_count_is_article() {
        let detected = detect_mode(None, 10_000);
        assert_eq!(detected, Mode::Article);
    }

    #[test]
    fn test_large_token_count_is_book() {
        let detected = detect_mode(None, 50_000);
        assert_eq!(detected, Mode::Book);
    }

    #[test]
    fn test_threshold_boundary() {
        assert_eq!(detect_mode(None, 29_999), Mode::Article);
        assert_eq!(detect_mode(None, 30_000), Mode::Book);
    }

    #[test]
    fn test_forced_mode_overrides() {
        assert_eq!(detect_mode(Some(Mode::Book), 1_000), Mode::Book);
        assert_eq!(detect_mode(Some(Mode::Article), 100_000), Mode::Article);
    }

    #[test]
    fn test_estimate_tokens() {
        // "hello world" = 2 words, 2 * 1.3 = 2.6 -> 2
        assert_eq!(estimate_tokens("hello world"), 2);
        // 10 words -> 13
        let text = "one two three four five six seven eight nine ten";
        assert_eq!(estimate_tokens(text), 13);
    }

    #[test]
    fn test_input_is_url() {
        assert!(is_url("https://example.com/article"));
        assert!(is_url("http://example.com/page"));
        assert!(!is_url("./local-file.pdf"));
        assert!(!is_url("/home/user/book.epub"));
    }
}
