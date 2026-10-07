pub const MASK_CHAR: char = '•';

/// Pattern formatting/masking tokens for input fields.
#[derive(Clone, PartialEq, Debug)]
pub enum MaskToken {
    /// Matches digits '0'..='9'
    Digit,
    /// Matches letters 'a'..='z' and 'A'..='Z'
    Letter,
    /// Matches any alphanumeric character
    AlphaNumeric,
    /// Fixed literal character that cannot be edited directly
    Literal(char),
}

#[derive(Clone, Debug, Default)]
pub struct MaskPattern {
    pub tokens: Vec<MaskToken>,
}

impl MaskPattern {
    pub fn parse(pattern_str: &str) -> Self {
        let mut tokens = Vec::new();
        let mut chars = pattern_str.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '0' | '9' => tokens.push(MaskToken::Digit),
                'a' | 'A' => tokens.push(MaskToken::Letter),
                '*' => tokens.push(MaskToken::AlphaNumeric),
                '\\' => {
                    if let Some(next) = chars.next() {
                        tokens.push(MaskToken::Literal(next));
                    }
                }
                lit => tokens.push(MaskToken::Literal(lit)),
            }
        }
        Self { tokens }
    }

    pub fn format(&self, raw: &str) -> String {
        let mut result = String::new();
        let mut raw_chars = raw.chars();

        for token in &self.tokens {
            match token {
                MaskToken::Literal(c) => result.push(*c),
                MaskToken::Digit => {
                    while let Some(c) = raw_chars.next() {
                        if c.is_ascii_digit() {
                            result.push(c);
                            break;
                        }
                    }
                }
                MaskToken::Letter => {
                    while let Some(c) = raw_chars.next() {
                        if c.is_alphabetic() {
                            result.push(c);
                            break;
                        }
                    }
                }
                MaskToken::AlphaNumeric => {
                    while let Some(c) = raw_chars.next() {
                        if c.is_alphanumeric() {
                            result.push(c);
                            break;
                        }
                    }
                }
            }
        }
        result
    }
}
