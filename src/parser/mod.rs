//! Constrained Lua-like blueprint parser. Does not execute Lua; parses tables, strings, numbers, booleans only.

mod lua_value;

pub use lua_value::LuaValue;

use crate::config::MAX_BLUEPRINT_FILE_BYTES;
use std::str::FromStr;

/// Parse blueprint content into a Lua table (root). Fails on syntax error or oversized input.
/// Real FAF unit files use `UnitBlueprint{ ... }`; we strip the prefix and parse the inner table.
pub fn parse_blueprint(content: &str) -> Result<LuaValue, ParseError> {
    if content.len() > MAX_BLUEPRINT_FILE_BYTES {
        return Err(ParseError::InputTooLarge);
    }
    let trimmed = content.trim_start();
    // Real FAF: UnitBlueprint{ ... } or MeshBlueprint{ ... } etc. Strip "FooBlueprint" and parse from first `{`.
    let start = if let Some(open) = trimmed.find('{') {
        if open == 0 {
            0
        } else {
            let prefix = trimmed.get(..open).unwrap_or("").trim_end();
            if prefix.ends_with("Blueprint") && prefix.chars().all(|c| c.is_ascii_alphabetic() || c == '_') {
                open
            } else {
                0
            }
        }
    } else {
        0
    };
    let slice = if start > 0 { &trimmed[start..] } else { trimmed };
    let mut p = Parser::new(slice);
    p.parse_value().and_then(|v| {
        p.skip_whitespace_and_comments();
        if p.rest().trim().is_empty() {
            Ok(v)
        } else {
            Err(ParseError::TrailingContent)
        }
    })
}

/// Parse a single value (table, string, number, boolean). For partial files.
pub fn parse_value(content: &str) -> Result<LuaValue, ParseError> {
    let mut p = Parser::new(content);
    p.parse_value()
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
    UnexpectedChar(char),
    InvalidNumber { at: usize },
    UnclosedString,
    InputTooLarge,
    TrailingContent,
    NestedTooDeep,
    InvalidEscape,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "unexpected end of input"),
            ParseError::UnexpectedChar(c) => write!(f, "unexpected character: {:?}", c),
            ParseError::InvalidNumber { at } => write!(f, "invalid number at byte {}", at),
            ParseError::UnclosedString => write!(f, "unclosed string"),
            ParseError::InputTooLarge => write!(f, "input exceeds maximum size"),
            ParseError::TrailingContent => write!(f, "trailing content after value"),
            ParseError::NestedTooDeep => write!(f, "nesting too deep"),
            ParseError::InvalidEscape => write!(f, "invalid escape in string"),
        }
    }
}

impl std::error::Error for ParseError {}

const MAX_DEPTH: u32 = 128;

struct Parser<'a> {
    s: &'a str,
    pos: usize,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser {
            s,
            pos: 0,
            depth: 0,
        }
    }

    fn rest(&self) -> &'a str {
        &self.s[self.pos..]
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            if rest.len() != trimmed.len() {
                self.pos += rest.len() - trimmed.len();
                continue;
            }
            if rest.starts_with("--") {
                if rest.starts_with("--[[") {
                    if let Some(end) = rest.find("]]") {
                        self.pos += end + 2;
                        continue;
                    }
                    break;
                }
                if let Some(nl) = rest.find('\n') {
                    self.pos += nl + 1;
                } else {
                    self.pos = self.s.len();
                }
                continue;
            }
            break;
        }
    }

    fn parse_value(&mut self) -> Result<LuaValue, ParseError> {
        self.skip_whitespace_and_comments();
        let rest = self.rest();
        if rest.is_empty() {
            return Err(ParseError::UnexpectedEof);
        }
        let c = rest.chars().next().unwrap();
        if c == '{' {
            self.parse_table()
        } else if c == '"' || c == '\'' {
            self.parse_string()
        } else if rest.starts_with("true") {
            self.pos += 4;
            Ok(LuaValue::Bool(true))
        } else if rest.starts_with("false") {
            self.pos += 5;
            Ok(LuaValue::Bool(false))
        } else if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            self.parse_number()
        } else if c.is_ascii_alphabetic() || c == '_' {
            self.parse_identifier_or_call_table()
        } else {
            Err(ParseError::UnexpectedChar(c))
        }
    }

    fn parse_identifier(&mut self) -> Result<LuaValue, ParseError> {
        let rest = self.rest();
        let start = self.pos;
        let mut end = start;
        for (i, c) in rest.char_indices() {
            if c.is_ascii_alphanumeric() || c == '_' {
                end = start + i + c.len_utf8();
            } else {
                break;
            }
        }
        if end <= start {
            return Err(ParseError::UnexpectedChar(
                rest.chars().next().unwrap_or(' '),
            ));
        }
        let ident = self.s[start..end].to_string();
        self.pos = end;
        Ok(LuaValue::String(ident))
    }

    /// Parse identifier; if followed by `{`, parse the table and return it (real FAF uses e.g. Sound { ... }).
    fn parse_identifier_or_call_table(&mut self) -> Result<LuaValue, ParseError> {
        let val = self.parse_identifier()?;
        self.skip_whitespace_and_comments();
        if self.rest().starts_with('{') {
            self.parse_table()
        } else {
            Ok(val)
        }
    }

    fn parse_table(&mut self) -> Result<LuaValue, ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError::NestedTooDeep);
        }
        self.depth += 1;

        self.skip_whitespace_and_comments();
        let rest = self.rest();
        if !rest.starts_with('{') {
            return Err(ParseError::UnexpectedChar(
                rest.chars().next().unwrap_or(' '),
            ));
        }
        self.pos += 1;

        let mut map = std::collections::BTreeMap::new();
        let mut next_index = 1u32;

        loop {
            self.skip_whitespace_and_comments();
            let rest = self.rest();
            if rest.is_empty() {
                self.depth -= 1;
                return Err(ParseError::UnexpectedEof);
            }
            if rest.starts_with('}') {
                self.pos += 1;
                break;
            }
            let (key, value): (LuaValue, LuaValue);
            if rest.starts_with('[') {
                self.pos += 1;
                key = match self.parse_value() {
                    Ok(k) => k,
                    Err(e) => {
                        self.depth -= 1;
                        return Err(e);
                    }
                };
                self.skip_whitespace_and_comments();
                if !self.rest().starts_with(']') {
                    self.depth -= 1;
                    return Err(ParseError::UnexpectedChar(
                        self.rest().chars().next().unwrap_or(' '),
                    ));
                }
                self.pos += 1;
                self.skip_whitespace_and_comments();
                if !self.rest().starts_with('=') {
                    self.depth -= 1;
                    return Err(ParseError::UnexpectedChar(
                        self.rest().chars().next().unwrap_or(' '),
                    ));
                }
                self.pos += 1;
                self.skip_whitespace_and_comments();
                value = match self.parse_value() {
                    Ok(v) => v,
                    Err(e) => {
                        self.depth -= 1;
                        return Err(e);
                    }
                };
            } else {
                let first = match self.parse_value() {
                    Ok(v) => v,
                    Err(e) => {
                        self.depth -= 1;
                        return Err(e);
                    }
                };
                self.skip_whitespace_and_comments();
                if self.rest().starts_with('=') {
                    self.pos += 1;
                    key = first;
                    self.skip_whitespace_and_comments();
                    value = match self.parse_value() {
                        Ok(v) => v,
                        Err(e) => {
                            self.depth -= 1;
                            return Err(e);
                        }
                    };
                } else {
                    key = LuaValue::Number(next_index as f64);
                    value = first;
                }
            }
            let key_idx = match &key {
                LuaValue::Number(n) if *n >= 1.0 && *n == (n.floor()) => *n as u32,
                _ => next_index,
            };
            if let LuaValue::Number(n) = &key {
                if *n >= 1.0 && *n == n.floor() {
                    next_index = (*n as u32).max(next_index).saturating_add(1);
                }
            }
            map.insert(LuaKey::from_value(key_idx, key), value);
            self.skip_whitespace_and_comments();
            let rest = self.rest();
            if rest.starts_with(',') || rest.starts_with(';') {
                self.pos += 1;
            } else if rest.starts_with('}') {
                self.pos += 1;
                break;
            } else {
                self.depth -= 1;
                return Err(ParseError::UnexpectedChar(
                    rest.chars().next().unwrap_or(' '),
                ));
            }
        }
        self.depth -= 1;
        Ok(LuaValue::Table(map))
    }

    fn parse_string(&mut self) -> Result<LuaValue, ParseError> {
        let rest = self.rest();
        let quote = if rest.starts_with('"') {
            '"'
        } else if rest.starts_with('\'') {
            '\''
        } else {
            return Err(ParseError::UnexpectedChar(
                rest.chars().next().unwrap_or(' '),
            ));
        };
        self.pos += 1;
        let mut s = String::new();
        loop {
            let rest = self.rest();
            if rest.is_empty() {
                return Err(ParseError::UnclosedString);
            }
            let c = rest.chars().next().unwrap();
            if c == quote {
                self.pos += 1;
                break;
            }
            if c == '\\' {
                self.pos += 1;
                let rest = self.rest();
                if rest.is_empty() {
                    return Err(ParseError::InvalidEscape);
                }
                let (esc, len) = parse_escape(rest)?;
                s.push(esc);
                self.pos += len;
                continue;
            }
            s.push(c);
            self.pos += c.len_utf8();
        }
        Ok(LuaValue::String(s))
    }

    fn parse_number(&mut self) -> Result<LuaValue, ParseError> {
        let rest = self.rest().trim_start();
        self.pos += self.rest().len() - rest.len();
        let start_pos = self.pos;
        let mut end = start_pos;
        let mut it = rest.char_indices();
        if rest.starts_with('-') || rest.starts_with('+') {
            it.next();
            end = start_pos + 1;
        }
        for (i, c) in it {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '-' || c == '+' {
                end = start_pos + i + c.len_utf8();
            } else {
                break;
            }
        }
        if end <= start_pos {
            return Err(ParseError::InvalidNumber { at: self.pos });
        }
        let slice = &self.s[start_pos..end];
        let n: f64 =
            f64::from_str(slice).map_err(|_| ParseError::InvalidNumber { at: start_pos })?;
        self.pos = end;
        // Real FAF uses RateOfFire = 10/20 (ticks); parse optional / <number> as division
        self.skip_whitespace_and_comments();
        let rest = self.rest().trim_start();
        if rest.starts_with('/') {
            self.pos += self.rest().len() - rest.len() + 1;
            self.skip_whitespace_and_comments();
            let second = self.parse_number()?;
            let denom = second.as_number().unwrap_or(1.0);
            Ok(LuaValue::Number(if denom != 0.0 { n / denom } else { n }))
        } else {
            Ok(LuaValue::Number(n))
        }
    }
}

fn parse_escape(rest: &str) -> Result<(char, usize), ParseError> {
    let c = rest.chars().next().ok_or(ParseError::InvalidEscape)?;
    match c {
        'n' => Ok(('\n', 1)),
        'r' => Ok(('\r', 1)),
        't' => Ok(('\t', 1)),
        '\\' => Ok(('\\', 1)),
        '"' => Ok(('"', 1)),
        '\'' => Ok(('\'', 1)),
        _ => Err(ParseError::InvalidEscape),
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum LuaKey {
    String(String),
    Number(u32),
}

impl LuaKey {
    fn from_value(next: u32, key: LuaValue) -> Self {
        match key {
            LuaValue::String(s) => LuaKey::String(s),
            LuaValue::Number(n) if n >= 1.0 && n == n.floor() => LuaKey::Number(n as u32),
            _ => LuaKey::Number(next),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_table() {
        let v = parse_blueprint("{}").unwrap();
        assert!(v.as_table().unwrap().is_empty());
    }

    #[test]
    fn parse_string_and_number() {
        let v = parse_blueprint(r#"{ foo = "bar", x = 42 }"#).unwrap();
        let t = v.as_table().unwrap();
        assert_eq!(
            t.get(&LuaKey::String("foo".to_string()))
                .and_then(LuaValue::as_str),
            Some("bar")
        );
        assert_eq!(
            t.get(&LuaKey::String("x".to_string()))
                .and_then(LuaValue::as_number),
            Some(42.0)
        );
    }

    #[test]
    fn parse_weapon_like_snippet() {
        let s = r#"
        {
            BlueprintId = "Weapon1",
            Damage = 100,
            RateOfFire = 2,
            MaxRadius = 25,
            ProjectilesPerOnFire = 1
        }
        "#;
        let v = parse_blueprint(s).unwrap();
        assert_eq!(v.get_str("BlueprintId"), Some("Weapon1"));
        assert_eq!(v.get_num("Damage"), Some(100.0));
        assert_eq!(v.get_num("RateOfFire"), Some(2.0));
    }

    #[test]
    fn parse_trailing_content_fails() {
        assert!(matches!(
            parse_blueprint("{} extra"),
            Err(ParseError::TrailingContent)
        ));
    }

    #[test]
    fn parse_unclosed_string_fails() {
        let r = parse_blueprint(r#"{ x = "unclosed }"#);
        assert!(r.is_err());
    }

    #[test]
    fn parse_fixture_uel0101() {
        let s = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/units/uel0101.lua"),
        )
        .unwrap();
        let v = parse_blueprint(&s).expect("parse uel0101 fixture");
        assert!(v.get_str("BlueprintId").is_some());
        assert!(v.get_table("Weapon").is_some());
    }
}
