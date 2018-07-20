use lex::{expect, span, Lex, LexErrorKind, LexResult};
use rhs_types::Bytes;
use std::{
    fmt::{self, Debug, Formatter, Write},
    hash::{Hash, Hasher},
};

#[derive(Clone)]
pub struct Regex(::regex::bytes::Regex);

impl Regex {
    pub fn new(s: &str) -> Result<Self, ::regex::Error> {
        ::regex::bytes::RegexBuilder::new(s)
            .unicode(false)
            .build()
            .map(Regex)
    }

    pub fn is_match(&self, text: &[u8]) -> bool {
        self.0.is_match(text)
    }

    pub fn try_from(bytes: Bytes) -> Result<Self, ::regex::Error> {
        Regex::new(&match bytes {
            Bytes::Raw(bytes) => {
                let mut regex_str = String::with_capacity(bytes.len() * r"\x00".len());
                for b in &*bytes {
                    write!(regex_str, r"\x{:02X}", b).unwrap();
                }
                regex_str
            }
            Bytes::Str(s) => format!("(?u){}", ::regex::escape(&s)),
        })
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Regex) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for Regex {}

impl Hash for Regex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state)
    }
}

impl Debug for Regex {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'i> Lex<'i> for Regex {
    fn lex(input: &str) -> LexResult<Self> {
        let input = expect(input, "\"")?;
        let mut regex_buf = String::new();
        let mut in_char_class = false;
        let (regex_str, input) = {
            let mut iter = input.chars();
            loop {
                let before_char = iter.as_str();
                match iter
                    .next()
                    .ok_or_else(|| (LexErrorKind::MissingEndingQuote, input))?
                {
                    '\\' => {
                        if let Some(c) = iter.next() {
                            if in_char_class || c != '"' {
                                regex_buf.push('\\');
                            }
                            regex_buf.push(c);
                        }
                    }
                    '"' if !in_char_class => {
                        break (span(input, before_char), iter.as_str());
                    }
                    '[' if !in_char_class => {
                        in_char_class = true;
                        regex_buf.push('[');
                    }
                    ']' if in_char_class => {
                        in_char_class = false;
                        regex_buf.push(']');
                    }
                    c => {
                        regex_buf.push(c);
                    }
                };
            }
        };
        match Regex::new(&regex_buf) {
            Ok(regex) => Ok((regex, input)),
            Err(err) => Err((LexErrorKind::ParseRegex(err), regex_str)),
        }
    }
}

#[test]
fn test() {
    assert_ok!(
        Regex::lex(r#""[a-z"\]]+\d{1,10}\"";"#),
        Regex::new(r#"[a-z"\]]+\d{1,10}""#).unwrap(),
        ";"
    );
}
