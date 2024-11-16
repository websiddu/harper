#![doc = include_str!("../README.md")]

use harper_core::language_detection::is_doc_likely_english;
use harper_core::linting::{LintGroup, LintGroupConfig, Linter};
use harper_core::parsers::{IsolateEnglish, PlainEnglish};
use harper_core::{remove_overlaps, Document, FullDictionary, Lrc};
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Mutex;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

static LINTER: Lazy<Mutex<LintGroup<Lrc<FullDictionary>>>> = Lazy::new(|| {
    Mutex::new(LintGroup::new(
        LintGroupConfig::default(),
        FullDictionary::curated(),
    ))
});

#[wasm_bindgen]
pub fn clean_mdx_content(mdx: &str) -> String {
    // Regex to match HTML tags and preserve attribute values.
    let tag_regex = Regex::new(r#"<(/?[\w\-]+)([^>]*)>"#).unwrap();
    let attr_regex = Regex::new(r#"\b[\w\-]+="([^"]*)""#).unwrap();
    // Regex for Markdown image tags.
    let image_tag_regex = Regex::new(r#"\!\[([^\]]+)\]\([^\)]+\)"#).unwrap();
    // Regex for Markdown links.
    let link_tag_regex = Regex::new(r#"\[([^\]]+)\]\([^\)]+\)"#).unwrap();

    // Regex for URLs (simple matching).
    let url_regex = Regex::new(r#"(https?://[^\s]+)"#).unwrap();
    // Regex for email addresses.
    let email_regex = Regex::new(r#"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"#).unwrap();
    // Regex for code blocks enclosed in triple backticks.
    let code_block_regex =
        Regex::new(r#"\s*```\s*[\s\S]*?\s*```\s*|\s*``````\s*[\s\S]*?\s*``````\s*"#).unwrap();
    // Regex for inline code snippets enclosed in backticks.
    let inline_code_regex = Regex::new(r#"`([^`]+)`"#).unwrap();
    // Regex for emojis (general Unicode range for emojis).
    let emoji_regex = Regex::new(r#"[\p{Emoji}]"#).unwrap();
    // Regex for sequences of dashes.
    let dash_regex = Regex::new(r#"-{2,}"#).unwrap();
    // Regex to match non-English characters.
    let non_english_words_regex = Regex::new(r#"[^\x00-\x7F]+"#).unwrap();

    // Step 1: Replace Markdown image tags, preserving their alt text but placing a space before it.
    let cleaned = image_tag_regex.replace_all(mdx, |caps: &regex::Captures| {
        let alt_text = &caps[1];
        format!(
            "  {}{}",
            alt_text,
            " ".repeat(caps[0].chars().count() - alt_text.chars().count() - 2)
        )
    });

    // Step 2: Replace Markdown link tags, preserving their link text but placing a space before it.
    let cleaned = link_tag_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        let link_text = &caps[1];
        format!(
            " {}{}",
            link_text,
            " ".repeat(caps[0].chars().count() - link_text.chars().count() - 1)
        )
    });

    // Step 3: Replace code blocks with spaces.
    let cleaned = code_block_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        " ".repeat(caps[0].chars().count())
    });

    // Step 4: Clean up HTML tags while preserving attribute values.
    let cleaned = tag_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        let tag_name = &caps[1];
        let attributes = &caps[2];

        // Replace the tag name with spaces.
        let mut result = " ".repeat(tag_name.chars().count() + 1);

        // Preserve the attribute values while replacing attribute names with spaces.
        let cleaned_attributes =
            attr_regex.replace_all(attributes, |attr_caps: &regex::Captures| {
                format!(
                    "{}\"{}\"",
                    " ".repeat(attr_caps[0].chars().count() - attr_caps[1].chars().count() - 2),
                    &attr_caps[1]
                )
            });

        result.push_str(&cleaned_attributes);
        result.push_str(" ");
        result
    });

    // Step 5: Replace URLs with spaces.
    let cleaned = url_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        " ".repeat(caps[0].chars().count())
    });

    // Step 6: Replace email addresses with spaces.
    let cleaned = email_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        " ".repeat(caps[0].chars().count())
    });

    // Step 7: Replace properly closed inline code snippets with spaces.
    let cleaned = inline_code_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        let content = &caps[1];
        // Replace only if the content is short (e.g., less than 50 characters) and does not span multiple lines.
        if content.chars().count() <= 50 && !content.contains('\n') {
            " ".repeat(caps[0].chars().count())
        } else {
            caps[0].to_string() // Leave it unchanged if it doesn't meet the criteria.
        }
    });

    // Step 8: Replace emojis with spaces, accounting for their UTF-16 length.
    let cleaned = emoji_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        // Calculate the number of UTF-16 code units.
        let utf16_length = caps[0].encode_utf16().count();
        " ".repeat(utf16_length)
    });

    // Step 9: Replace sequences of two or more dashes with spaces.
    let cleaned = dash_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        " ".repeat(caps[0].chars().count())
    });

    // Step 10: Replace non-English words, preserving grapheme cluster length.
    let cleaned = non_english_words_regex.replace_all(&cleaned, |caps: &regex::Captures| {
        " ".repeat(caps[0].chars().count())
    });

    cleaned.to_string()
}

/// Setup the WebAssembly module's logging.
///
/// Not strictly necessary for anything to function, but makes bug-hunting less
/// painful.
#[wasm_bindgen(start)]
pub fn setup() {
    console_error_panic_hook::set_once();

    tracing_wasm::set_as_global_default();
}

/// Helper method to quickly check if a plain string is likely intended to be English
#[wasm_bindgen]
pub fn is_likely_english(text: String) -> bool {
    let document = Document::new_plain_english_curated(&text);
    is_doc_likely_english(&document, &FullDictionary::curated())
}

/// Helper method to remove non-English text from a plain English document.
#[wasm_bindgen]
pub fn isolate_english(text: String) -> String {
    let dict = FullDictionary::curated();

    let document = Document::new_curated(
        &text,
        &mut IsolateEnglish::new(Box::new(PlainEnglish), dict.clone()),
    );

    document.to_string()
}

#[wasm_bindgen]
pub fn get_lint_config_as_object() -> JsValue {
    let linter = LINTER.lock().unwrap();
    serde_wasm_bindgen::to_value(&linter.config).unwrap()
}

#[wasm_bindgen]
pub fn set_lint_config_from_object(object: JsValue) -> Result<(), String> {
    let mut linter = LINTER.lock().unwrap();
    linter.config = serde_wasm_bindgen::from_value(object).map_err(|v| v.to_string())?;
    Ok(())
}

/// Perform the configured linting on the provided text.
#[wasm_bindgen]
pub fn lint(text: String) -> Vec<Lint> {
    let source: Vec<_> = text.chars().collect();
    let source = Lrc::new(source);

    let document = Document::new_from_vec(
        source.clone(),
        &mut PlainEnglish,
        &FullDictionary::curated(),
    );

    let mut lints = LINTER.lock().unwrap().lint(&document);

    remove_overlaps(&mut lints);

    lints
        .into_iter()
        .map(|l| Lint::new(l, source.clone()))
        .collect()
}

#[wasm_bindgen]
pub fn apply_suggestion(
    text: String,
    span: Span,
    suggestion: &Suggestion,
) -> Result<String, String> {
    let mut source: Vec<_> = text.chars().collect();
    let span: harper_core::Span = span.into();

    suggestion.inner.apply(span, &mut source);

    Ok(source.iter().collect())
}

#[wasm_bindgen]
pub struct Suggestion {
    inner: harper_core::linting::Suggestion,
}

#[wasm_bindgen]
pub enum SuggestionKind {
    Replace,
    Remove,
}

#[wasm_bindgen]
impl Suggestion {
    pub(crate) fn new(inner: harper_core::linting::Suggestion) -> Self {
        Self { inner }
    }

    /// Get the text that is going to replace error.
    /// If [`Self::kind`] is `SuggestionKind::Remove`, this will return an empty
    /// string.
    pub fn get_replacement_text(&self) -> String {
        match &self.inner {
            harper_core::linting::Suggestion::Remove => "".to_string(),
            harper_core::linting::Suggestion::ReplaceWith(chars) => chars.iter().collect(),
        }
    }

    pub fn kind(&self) -> SuggestionKind {
        match &self.inner {
            harper_core::linting::Suggestion::Remove => SuggestionKind::Remove,
            harper_core::linting::Suggestion::ReplaceWith(_) => SuggestionKind::Replace,
        }
    }
}

#[wasm_bindgen]
pub struct Lint {
    inner: harper_core::linting::Lint,
    source: Lrc<Vec<char>>,
}

#[wasm_bindgen]
impl Lint {
    pub(crate) fn new(inner: harper_core::linting::Lint, source: Lrc<Vec<char>>) -> Self {
        Self { inner, source }
    }

    /// Get the content of the source material pointed to by [`Self::span`]
    pub fn get_problem_text(&self) -> String {
        self.inner.span.get_content_string(&self.source)
    }

    /// Get a string representing the general category of the lint.
    pub fn lint_kind(&self) -> String {
        self.inner.lint_kind.to_string()
    }

    pub fn suggestion_count(&self) -> usize {
        self.inner.suggestions.len()
    }

    pub fn suggestions(&self) -> Vec<Suggestion> {
        self.inner
            .suggestions
            .iter()
            .map(|s| Suggestion::new(s.clone()))
            .collect()
    }

    pub fn span(&self) -> Span {
        self.inner.span.into()
    }

    pub fn message(&self) -> String {
        self.inner.message.clone()
    }
}

#[wasm_bindgen]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[wasm_bindgen]
impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl From<Span> for harper_core::Span {
    fn from(value: Span) -> Self {
        harper_core::Span::new(value.start, value.end)
    }
}

impl From<harper_core::Span> for Span {
    fn from(value: harper_core::Span) -> Self {
        Span::new(value.start, value.end)
    }
}
