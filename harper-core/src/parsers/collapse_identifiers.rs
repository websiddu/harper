use crate::Lrc;
use std::collections::VecDeque;

use itertools::Itertools;

use super::{Parser, TokenKind};
use crate::patterns::{PatternExt, SequencePattern};
use crate::{Dictionary, FullDictionary, MergedDictionary, Span, Token, VecExt, WordMetadata};

/// A parser that wraps any other parser to collapse token strings that match
/// the pattern word_word or word-word.
pub struct CollapseIdentifiers {
    inner: Box<dyn Parser>,
    dict: Lrc<MergedDictionary<FullDictionary>>,
}

impl CollapseIdentifiers {
    pub fn new(inner: Box<dyn Parser>, dict: &Lrc<MergedDictionary<FullDictionary>>) -> Self {
        Self {
            inner,
            dict: dict.clone(),
        }
    }
}

thread_local! {
    static WORD_OR_NUMBER: Lrc<SequencePattern> = Lrc::new(SequencePattern::default()
                .then_any_word()
                .then_one_or_more(Box::new(SequencePattern::default()
        .then_case_separator()
        .then_any_word())));
}

impl Parser for CollapseIdentifiers {
    fn parse(&mut self, source: &[char]) -> Vec<Token> {
        let mut tokens = self.inner.parse(source);

        let mut removal_indexes: VecDeque<usize> = VecDeque::default();
        let replacements = WORD_OR_NUMBER
            .with(|v| v.clone())
            .find_all_matches(&tokens, source)
            .into_iter()
            .map(|s| {
                let start_tok = tokens
                    .get(s.start)
                    .expect("Token not at expected position.");
                let end_tok = tokens
                    .get(s.end - 1)
                    .expect("Token not at expected position.");
                let char_span = Span::new(start_tok.span.start, end_tok.span.end);
                (
                    s.start,
                    s.end,
                    Token::new(char_span, TokenKind::Word(WordMetadata::default())),
                    char_span.get_content_string(source),
                )
            })
            .filter(|(_, _, _, st)| self.dict.contains_word_str(st))
            .collect_vec();

        replacements.into_iter().for_each(|(s, e, t, _)| {
            (s + 1..=e).for_each(|n| removal_indexes.push_front(n));
            tokens[s] = t;
        });
        tokens.remove_indices(removal_indexes.into_iter().sorted().unique().collect());

        tokens
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::{PlainEnglish, StrParser};

    use super::*;

    #[test]
    fn no_collapse() {
        let dict = FullDictionary::curated();
        let source = "This is a test.";

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(dict.into()))
            .parse_str(source);
        assert_eq!(tokens.len(), 8);
    }

    #[test]
    fn one_collapse() {
        let source = "This is a separated_identifier, wow!";
        let default_dict = FullDictionary::curated();

        let tokens = CollapseIdentifiers::new(
            Box::new(PlainEnglish),
            &Lrc::new(default_dict.clone().into()),
        )
        .parse_str(source);
        assert_eq!(tokens.len(), 13);

        let mut dict = FullDictionary::new();
        dict.append_word(
            "separated_identifier".chars().collect_vec(),
            WordMetadata::default(),
        );

        let mut merged_dict = MergedDictionary::from(default_dict);
        merged_dict.add_dictionary(Lrc::new(dict));

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(merged_dict))
            .parse_str(source);
        assert_eq!(tokens.len(), 10);
    }

    #[test]
    fn kebab_collapse() {
        let source = "This is a separated-identifier, wow!";
        let default_dict = FullDictionary::curated();

        let tokens = CollapseIdentifiers::new(
            Box::new(PlainEnglish),
            &Lrc::new(default_dict.clone().into()),
        )
        .parse_str(source);
        assert_eq!(tokens.len(), 13);

        let mut dict = FullDictionary::new();
        dict.append_word(
            "separated-identifier".chars().collect_vec(),
            WordMetadata::default(),
        );

        let mut merged_dict = MergedDictionary::from(default_dict);
        merged_dict.add_dictionary(Lrc::new(dict));

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(merged_dict))
            .parse_str(source);
        assert_eq!(tokens.len(), 10);
    }

    #[test]
    fn double_collapse() {
        let source = "This is a separated_identifier_token, wow!";
        let default_dict = FullDictionary::curated();

        let tokens = CollapseIdentifiers::new(
            Box::new(PlainEnglish),
            &Lrc::new(default_dict.clone().into()),
        )
        .parse_str(source);
        assert_eq!(tokens.len(), 15);

        let mut dict = FullDictionary::new();
        dict.append_word(
            "separated_identifier_token".chars().collect_vec(),
            WordMetadata::default(),
        );

        let mut merged_dict = MergedDictionary::from(default_dict);
        merged_dict.add_dictionary(Lrc::new(dict));

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(merged_dict))
            .parse_str(source);
        assert_eq!(tokens.len(), 10);
    }

    #[test]
    fn two_collapses() {
        let source = "This is a separated_identifier, wow! separated_identifier";
        let default_dict = FullDictionary::curated();

        let tokens = CollapseIdentifiers::new(
            Box::new(PlainEnglish),
            &Lrc::new(default_dict.clone().into()),
        )
        .parse_str(source);
        assert_eq!(tokens.len(), 17);

        let mut dict = FullDictionary::new();
        dict.append_word(
            "separated_identifier".chars().collect_vec(),
            WordMetadata::default(),
        );

        let mut merged_dict = MergedDictionary::from(default_dict);
        merged_dict.add_dictionary(Lrc::new(dict));

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(merged_dict))
            .parse_str(source);
        assert_eq!(tokens.len(), 12);
    }

    #[test]
    fn overlapping_identifiers() {
        let source = "This is a separated_identifier_token, wow!";
        let default_dict = FullDictionary::curated();

        let tokens = CollapseIdentifiers::new(
            Box::new(PlainEnglish),
            &Lrc::new(default_dict.clone().into()),
        )
        .parse_str(source);
        assert_eq!(tokens.len(), 15);

        let mut dict = FullDictionary::new();
        dict.append_word(
            "separated_identifier".chars().collect_vec(),
            WordMetadata::default(),
        );
        dict.append_word(
            "identifier_token".chars().collect_vec(),
            WordMetadata::default(),
        );

        let mut merged_dict = MergedDictionary::from(default_dict);
        merged_dict.add_dictionary(Lrc::new(dict));

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(merged_dict))
            .parse_str(source);
        assert_eq!(tokens.len(), 15);
    }

    #[test]
    fn nested_identifiers() {
        let source = "This is a separated_identifier_token, wow!";
        let default_dict = FullDictionary::curated();

        let tokens = CollapseIdentifiers::new(
            Box::new(PlainEnglish),
            &Lrc::new(default_dict.clone().into()),
        )
        .parse_str(source);
        assert_eq!(tokens.len(), 15);

        let mut dict = FullDictionary::new();
        dict.append_word(
            "separated_identifier_token".chars().collect_vec(),
            WordMetadata::default(),
        );
        dict.append_word(
            "separated_identifier".chars().collect_vec(),
            WordMetadata::default(),
        );

        let mut merged_dict = MergedDictionary::from(default_dict);
        merged_dict.add_dictionary(Lrc::new(dict));

        let tokens = CollapseIdentifiers::new(Box::new(PlainEnglish), &Lrc::new(merged_dict))
            .parse_str(source);
        assert_eq!(tokens.len(), 10);
    }
}
