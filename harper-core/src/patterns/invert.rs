use crate::Token;

use super::Pattern;

/// A struct that matches any pattern __except__ the one provided.
pub struct Invert {
    inner: Box<dyn Pattern>,
}

impl Invert {
    pub fn new(inner: Box<dyn Pattern>) -> Self {
        Self { inner }
    }
}

impl Pattern for Invert {
    fn matches(&self, tokens: &[Token], source: &[char]) -> usize {
        if self.inner.matches(tokens, source) != 0 {
            0
        } else {
            1
        }
    }
}
