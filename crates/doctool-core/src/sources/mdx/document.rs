use std::collections::HashMap;

use md5::compute;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::{parse_frontmatter, split_frontmatter};

const FROZEN_FRONTMATTER_KEYS: &[&str] = &["method", "path", "operationId", "operationid"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyBlockKind {
    Prose,
    CodeFence,
    Jsx,
    Heading,
}

#[derive(Debug, Clone)]
pub struct BodyBlock {
    pub kind: BodyBlockKind,
    pub content: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct MdxDocument {
    pub relative_path: String,
    pub frontmatter: HashMap<String, String>,
    pub body_blocks: Vec<BodyBlock>,
}

impl MdxDocument {
    pub fn parse(relative_path: impl Into<String>, raw: &str) -> Self {
        let relative_path = relative_path.into();
        let (fm_raw, body) = split_frontmatter(raw);
        let frontmatter = parse_frontmatter(&fm_raw);
        let body_blocks = split_body_blocks(&body);
        Self {
            relative_path,
            frontmatter,
            body_blocks,
        }
    }

    /// Segment key → value map for lock-file hashing (EN source only).
    pub fn segment_values(&self) -> HashMap<String, String> {
        let mut segments = HashMap::new();

        for (key, value) in &self.frontmatter {
            let lower = key.to_lowercase();
            if FROZEN_FRONTMATTER_KEYS.contains(&lower.as_str()) {
                continue;
            }
            if key == "title" || key == "description" {
                segments.insert(format!("frontmatter:{key}"), value.clone());
            }
        }

        for block in &self.body_blocks {
            let key = format!("body:{}", block.index);
            segments.insert(key, block.content.clone());
        }

        segments
    }

    pub fn segment_hashes(&self) -> HashMap<String, String> {
        self.segment_values()
            .into_iter()
            .map(|(k, v)| (k, hash_value(&v)))
            .collect()
    }

    pub fn heading_count(&self) -> usize {
        self.body_blocks
            .iter()
            .filter(|b| b.kind == BodyBlockKind::Heading)
            .count()
    }

    pub fn internal_links(&self) -> Vec<String> {
        let re = regex::Regex::new(r"\]\(/(start|build|api|resources)/([^)\s#]+)").unwrap();
        let mut links = Vec::new();
        for block in &self.body_blocks {
            for cap in re.captures_iter(&block.content) {
                links.push(format!("{}/{}", &cap[1], &cap[2]));
            }
        }
        links.sort();
        links.dedup();
        links
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        if !self.frontmatter.is_empty() {
            out.push_str("---\n");
            let mut keys: Vec<_> = self.frontmatter.keys().collect();
            keys.sort();
            for key in keys {
                let value = &self.frontmatter[key];
                out.push_str(key);
                out.push_str(": ");
                if value.contains(':') || value.contains('"') {
                    out.push('"');
                    out.push_str(value);
                    out.push('"');
                } else {
                    out.push_str(value);
                }
                out.push('\n');
            }
            out.push_str("---\n\n");
        }
        for block in &self.body_blocks {
            out.push_str(&block.content);
            if !block.content.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
        out
    }
}

pub fn hash_value(value: &str) -> String {
    format!("{:x}", compute(value.as_bytes()))
}

fn split_body_blocks(body: &str) -> Vec<BodyBlock> {
    let mut blocks = Vec::new();
    let mut index = 0usize;

    // JSX / component blocks: lines starting with < that aren't markdown
    let mut prose_buf = String::new();
    let mut in_jsx = false;
    let mut jsx_buf = String::new();

    for line in body.lines() {
        let trimmed = line.trim();
        let looks_like_jsx = trimmed.starts_with('<')
            && !trimmed.starts_with("<!--")
            && !trimmed.starts_with("<http");

        if looks_like_jsx {
            if !prose_buf.is_empty() {
                flush_prose_blocks(&mut blocks, &mut index, &prose_buf);
                prose_buf.clear();
            }
            in_jsx = true;
            jsx_buf.push_str(line);
            jsx_buf.push('\n');
            continue;
        }

        if in_jsx {
            if trimmed.is_empty() {
                blocks.push(BodyBlock {
                    kind: BodyBlockKind::Jsx,
                    content: jsx_buf.trim_end().to_string(),
                    index,
                });
                index += 1;
                jsx_buf.clear();
                in_jsx = false;
            } else {
                jsx_buf.push_str(line);
                jsx_buf.push('\n');
            }
            continue;
        }

        prose_buf.push_str(line);
        prose_buf.push('\n');
    }

    if in_jsx && !jsx_buf.is_empty() {
        blocks.push(BodyBlock {
            kind: BodyBlockKind::Jsx,
            content: jsx_buf.trim_end().to_string(),
            index,
        });
        index += 1;
    } else if !prose_buf.is_empty() {
        flush_prose_blocks(&mut blocks, &mut index, &prose_buf);
    }

    blocks
}

fn flush_prose_blocks(blocks: &mut Vec<BodyBlock>, index: &mut usize, prose: &str) {
    let opts = Options::empty();
    let parser = Parser::new_ext(prose, opts);

    let mut current = String::new();
    let mut in_heading = false;
    let mut in_code = false;
    let mut code_lang = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                flush_prose(blocks, index, &current);
                current.clear();
                in_heading = true;
            }
            Event::End(TagEnd::Heading(..)) => {
                if !current.trim().is_empty() {
                    blocks.push(BodyBlock {
                        kind: BodyBlockKind::Heading,
                        content: current.trim().to_string(),
                        index: *index,
                    });
                    *index += 1;
                }
                current.clear();
                in_heading = false;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_prose(blocks, index, &current);
                current.clear();
                in_code = true;
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                current.push_str("```");
                if !code_lang.is_empty() {
                    current.push_str(&code_lang);
                }
                current.push('\n');
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code {
                    current.push_str("```");
                    blocks.push(BodyBlock {
                        kind: BodyBlockKind::CodeFence,
                        content: current.clone(),
                        index: *index,
                    });
                    *index += 1;
                    current.clear();
                    in_code = false;
                }
            }
            Event::Text(text) => {
                if in_heading || in_code {
                    current.push_str(&text);
                } else {
                    current.push_str(&text);
                }
            }
            Event::Code(text) => current.push_str(&text),
            Event::SoftBreak | Event::HardBreak => current.push('\n'),
            _ => {}
        }
    }

    if in_code && !current.is_empty() {
        blocks.push(BodyBlock {
            kind: BodyBlockKind::CodeFence,
            content: current,
            index: *index,
        });
        *index += 1;
    } else {
        flush_prose(blocks, index, &current);
    }
}

fn flush_prose(blocks: &mut Vec<BodyBlock>, index: &mut usize, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    blocks.push(BodyBlock {
        kind: BodyBlockKind::Prose,
        content: trimmed.to_string(),
        index: *index,
    });
    *index += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_segments() {
        let raw = r#"---
title: Hello
description: World
method: GET
path: /items
---

# Intro

Some prose here.
"#;
        let doc = MdxDocument::parse("build/test.mdx", raw);
        let segs = doc.segment_values();
        assert!(segs.contains_key("frontmatter:title"));
        assert!(segs.contains_key("frontmatter:description"));
        assert!(!segs.contains_key("frontmatter:method"));
        assert!(doc.heading_count() >= 1);
    }

    #[test]
    fn round_trip_preserves_title_hash() {
        let raw = r#"---
title: Test
---

# Heading

Paragraph one.

```ts
const x = 1;
```
"#;
        let doc = MdxDocument::parse("a.mdx", raw);
        let serialized = doc.serialize();
        let reparsed = MdxDocument::parse("a.mdx", &serialized);
        assert_eq!(
            doc.segment_hashes().get("frontmatter:title"),
            reparsed.segment_hashes().get("frontmatter:title")
        );
        assert!(reparsed.segment_hashes().len() >= doc.segment_hashes().len() - 1);
    }
}
