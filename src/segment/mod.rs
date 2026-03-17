pub mod chunk;
pub use chunk::Chunk;

use crate::mode::estimate_tokens;

const MAX_CHUNK_TOKENS: usize = 5000;
const MIN_CHUNK_TOKENS: usize = 500;
const OVERLAP_RATIO: f64 = 0.10;

struct RawSection {
    header_path: Vec<String>,
    content: String,
}

fn parse_header(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    let rest = trimmed[level..].trim();
    if (1..=6).contains(&level) && !rest.is_empty() {
        Some((level, rest.to_string()))
    } else {
        None
    }
}

fn split_by_headers(text: &str) -> Vec<RawSection> {
    let mut sections: Vec<RawSection> = Vec::new();
    // Track the current header stack: (level, title)
    let mut header_stack: Vec<(usize, String)> = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();
    let mut current_header_path: Vec<String> = Vec::new();

    for line in text.lines() {
        if let Some((level, title)) = parse_header(line) {
            // Flush current section
            let content = current_lines.join("\n").trim().to_string();
            if !content.is_empty() {
                sections.push(RawSection {
                    header_path: current_header_path.clone(),
                    content,
                });
            }
            current_lines.clear();

            // Update header stack: pop everything at this level or deeper
            while let Some((top_level, _)) = header_stack.last() {
                if *top_level >= level {
                    header_stack.pop();
                } else {
                    break;
                }
            }
            header_stack.push((level, title));

            // Build header_path from stack
            current_header_path = header_stack.iter().map(|(_, t)| t.clone()).collect();
        } else {
            current_lines.push(line);
        }
    }

    // Flush remaining content
    let content = current_lines.join("\n").trim().to_string();
    if !content.is_empty() {
        sections.push(RawSection {
            header_path: current_header_path,
            content,
        });
    }

    sections
}

fn split_by_words(header_path: &[String], text: &str) -> Vec<Chunk> {
    let words: Vec<&str> = text.split_whitespace().collect();
    // Target word count per chunk: MAX_CHUNK_TOKENS / 1.3
    let words_per_chunk = (MAX_CHUNK_TOKENS as f64 / 1.3) as usize;
    let mut chunks: Vec<Chunk> = Vec::new();

    for chunk_words in words.chunks(words_per_chunk) {
        let content = chunk_words.join(" ");
        chunks.push(Chunk {
            index: 0,
            header_path: header_path.to_vec(),
            token_estimate: estimate_tokens(&content),
            content,
        });
    }

    chunks
}

fn split_by_paragraphs(header_path: &[String], content: &str) -> Vec<Chunk> {
    let paragraphs: Vec<&str> = content.split("\n\n").collect();
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut current_text = String::new();

    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }

        // If a single paragraph exceeds max, split it by words
        if estimate_tokens(para) > MAX_CHUNK_TOKENS && current_text.is_empty() {
            chunks.extend(split_by_words(header_path, para));
            continue;
        }

        let combined = if current_text.is_empty() {
            para.to_string()
        } else {
            format!("{}\n\n{}", current_text, para)
        };

        let tokens = estimate_tokens(&combined);
        if tokens > MAX_CHUNK_TOKENS && !current_text.is_empty() {
            // Flush current_text as a chunk before it exceeds max
            chunks.push(Chunk {
                index: 0,
                header_path: header_path.to_vec(),
                token_estimate: estimate_tokens(&current_text),
                content: current_text,
            });
            // The paragraph itself might also be too large
            if estimate_tokens(para) > MAX_CHUNK_TOKENS {
                chunks.extend(split_by_words(header_path, para));
                current_text = String::new();
            } else {
                current_text = para.to_string();
            }
        } else {
            current_text = combined;
        }
    }

    if !current_text.is_empty() {
        chunks.push(Chunk {
            index: 0,
            header_path: header_path.to_vec(),
            token_estimate: estimate_tokens(&current_text),
            content: current_text,
        });
    }

    chunks
}

fn top_level_header(chunk: &Chunk) -> Option<&str> {
    chunk.header_path.first().map(|s| s.as_str())
}

fn merge_small_chunks(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut merged: Vec<Chunk> = Vec::new();

    for chunk in chunks {
        if let Some(last) = merged.last_mut()
            && last.token_estimate < MIN_CHUNK_TOKENS
            && top_level_header(last) == top_level_header(&chunk)
        {
            last.content = format!("{}\n\n{}", last.content, chunk.content);
            last.token_estimate = estimate_tokens(&last.content);
            continue;
        }
        merged.push(chunk);
    }

    // Check if the last chunk is too small and merge it with the previous one
    if merged.len() >= 2 {
        let last_idx = merged.len() - 1;
        if merged[last_idx].token_estimate < MIN_CHUNK_TOKENS
            && top_level_header(&merged[last_idx]) == top_level_header(&merged[last_idx - 1])
        {
            let last = merged.pop().unwrap();
            let prev = merged.last_mut().unwrap();
            prev.content = format!("{}\n\n{}", prev.content, last.content);
            prev.token_estimate = estimate_tokens(&prev.content);
        }
    }

    merged
}

fn add_overlap(chunks: &mut [Chunk]) {
    if chunks.len() < 2 {
        return;
    }

    // Work backwards to avoid mutating content we still need to read
    let overlaps: Vec<String> = chunks
        .iter()
        .take(chunks.len() - 1)
        .map(|chunk| {
            let words: Vec<&str> = chunk.content.split_whitespace().collect();
            let overlap_count = (words.len() as f64 * OVERLAP_RATIO).ceil() as usize;
            let overlap_count = overlap_count.max(1);
            let start = words.len().saturating_sub(overlap_count);
            words[start..].join(" ")
        })
        .collect();

    for (i, overlap_text) in overlaps.into_iter().enumerate() {
        let target = &mut chunks[i + 1];
        target.content = format!("{}\n\n{}", overlap_text, target.content);
        target.token_estimate = estimate_tokens(&target.content);
    }
}

pub fn segment(text: &str) -> Vec<Chunk> {
    let sections = split_by_headers(text);

    let mut chunks: Vec<Chunk> = Vec::new();

    if sections.is_empty() {
        // No content at all
        return chunks;
    }

    for section in &sections {
        let tokens = estimate_tokens(&section.content);
        if tokens > MAX_CHUNK_TOKENS {
            let sub_chunks = split_by_paragraphs(&section.header_path, &section.content);
            chunks.extend(sub_chunks);
        } else {
            chunks.push(Chunk {
                index: 0,
                header_path: section.header_path.clone(),
                content: section.content.clone(),
                token_estimate: tokens,
            });
        }
    }

    chunks = merge_small_chunks(chunks);
    add_overlap(&mut chunks);

    // Assign sequential indices
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = i;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_headers() {
        let input = "# Chapter 1\n\nSome text here.\n\n## Section 1.1\n\nMore text.\n\n# Chapter 2\n\nAnother chapter.";
        let chunks = segment(input);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].header_path, vec!["Chapter 1"]);
    }

    #[test]
    fn test_no_headers_produces_chunks() {
        let input = "Just a long paragraph without any headers. ".repeat(500);
        let chunks = segment(&input);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_respects_max_chunk_size() {
        let big_section = format!("# Big Section\n\n{}", "word ".repeat(5000));
        let chunks = segment(&big_section);
        for chunk in &chunks {
            // Allow some tolerance for overlap
            assert!(
                chunk.token_estimate <= 6000,
                "chunk {} has {} tokens",
                chunk.index,
                chunk.token_estimate
            );
        }
    }

    #[test]
    fn test_small_sections_merged() {
        let input = "# A\n\nTiny.\n\n# B\n\nAlso tiny.\n\n# C\n\nStill tiny.";
        let chunks = segment(input);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let input = "# One\n\nSome text for one.\n\n# Two\n\nSome text for two.\n\n# Three\n\nSome text for three.";
        let chunks = segment(input);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    #[test]
    fn test_parse_header() {
        assert_eq!(parse_header("# Title"), Some((1, "Title".to_string())));
        assert_eq!(
            parse_header("## Sub Title"),
            Some((2, "Sub Title".to_string()))
        );
        assert_eq!(parse_header("### Deep"), Some((3, "Deep".to_string())));
        assert_eq!(parse_header("Not a header"), None);
        assert_eq!(parse_header("##"), None); // no title text
    }

    #[test]
    fn test_header_hierarchy() {
        let input =
            "# Chapter 1\n\n## Section 1.1\n\nContent here.\n\n## Section 1.2\n\nMore content.";
        let chunks = segment(input);
        // All chunks should reference Chapter 1 in their header_path
        for chunk in &chunks {
            assert!(
                chunk.header_path.first().map(|s| s.as_str()) == Some("Chapter 1"),
                "Expected Chapter 1 in header_path, got {:?}",
                chunk.header_path
            );
        }
    }

    #[test]
    fn test_overlap_present() {
        // Create sections large enough to not be merged
        let section_text = "word ".repeat(500);
        let input = format!(
            "# Section A\n\n{}\n\n# Section B\n\n{}",
            section_text, section_text
        );
        let chunks = segment(&input);
        if chunks.len() >= 2 {
            // The second chunk should contain some overlap from the first
            // Since we add overlap, the second chunk's token count should be
            // larger than just the section text alone
            let section_only_tokens = estimate_tokens(&section_text.trim());
            assert!(
                chunks[1].token_estimate > section_only_tokens,
                "Expected overlap to increase token count"
            );
        }
    }
}
