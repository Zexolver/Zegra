//! Minimal parser for Valve's KeyValues ("VDF") text format, the format used by
//! Steam's `libraryfolders.vdf` and per-game `appmanifest_*.acf` files.
//!
//! Only the subset needed to read those two file types is implemented: quoted
//! string keys/values and nested `{ }` blocks. Comments (`//`) are skipped.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum VdfValue {
    Str(String),
    Obj(Vec<(String, VdfValue)>),
}

impl VdfValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            VdfValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&[(String, VdfValue)]> {
        match self {
            VdfValue::Obj(entries) => Some(entries),
            _ => None,
        }
    }

    /// Looks up a direct child key (case-insensitive, matching Steam's own leniency).
    pub fn get(&self, key: &str) -> Option<&VdfValue> {
        self.as_obj()?
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    /// Convenience: returns a flat map of this object's direct string-valued children.
    pub fn flat_strings(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(entries) = self.as_obj() {
            for (k, v) in entries {
                if let VdfValue::Str(s) = v {
                    map.insert(k.clone(), s.clone());
                }
            }
        }
        map
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VdfError {
    #[error("unexpected end of input while parsing VDF")]
    UnexpectedEof,
    #[error("unexpected token at byte offset {0}")]
    UnexpectedToken(usize),
}

struct Tokenizer<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    OpenBrace,
    CloseBrace,
}

impl<'a> Tokenizer<'a> {
    fn new(src: &'a str) -> Self {
        Tokenizer {
            chars: src.char_indices().peekable(),
        }
    }

    fn skip_ignorable(&mut self) {
        loop {
            match self.chars.peek() {
                Some((_, c)) if c.is_whitespace() => {
                    self.chars.next();
                }
                Some((_, '/')) => {
                    let mut lookahead = self.chars.clone();
                    lookahead.next();
                    if let Some((_, '/')) = lookahead.peek() {
                        // line comment: consume until newline
                        for (_, c) in self.chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, VdfError> {
        self.skip_ignorable();
        let (start, c) = match self.chars.next() {
            Some(pair) => pair,
            None => return Ok(None),
        };
        match c {
            '{' => Ok(Some(Token::OpenBrace)),
            '}' => Ok(Some(Token::CloseBrace)),
            '"' => {
                let mut s = String::new();
                loop {
                    match self.chars.next() {
                        Some((_, '"')) => break,
                        Some((_, '\\')) => {
                            // Handle escaped characters (\" and \\ are the common ones in VDF).
                            if let Some((_, next)) = self.chars.next() {
                                s.push(next);
                            } else {
                                return Err(VdfError::UnexpectedEof);
                            }
                        }
                        Some((_, ch)) => s.push(ch),
                        None => return Err(VdfError::UnexpectedEof),
                    }
                }
                Ok(Some(Token::Str(s)))
            }
            _ => {
                // Bareword token (unquoted); read until whitespace or brace.
                let mut s = String::new();
                s.push(c);
                while let Some((_, ch)) = self.chars.peek().copied() {
                    if ch.is_whitespace() || ch == '{' || ch == '}' {
                        break;
                    }
                    s.push(ch);
                    self.chars.next();
                }
                if s.is_empty() {
                    return Err(VdfError::UnexpectedToken(start));
                }
                Ok(Some(Token::Str(s)))
            }
        }
    }
}

/// Parses a VDF document into a single root object containing all top-level keys.
pub fn parse(input: &str) -> Result<VdfValue, VdfError> {
    let mut tok = Tokenizer::new(input);
    let entries = parse_entries(&mut tok)?;
    Ok(VdfValue::Obj(entries))
}

fn parse_entries(tok: &mut Tokenizer) -> Result<Vec<(String, VdfValue)>, VdfError> {
    let mut entries = Vec::new();
    loop {
        let key = match tok.next_token()? {
            None => break,
            Some(Token::CloseBrace) => break,
            Some(Token::Str(s)) => s,
            Some(Token::OpenBrace) => return Err(VdfError::UnexpectedToken(0)),
        };
        let value = match tok.next_token()? {
            Some(Token::Str(s)) => VdfValue::Str(s),
            Some(Token::OpenBrace) => VdfValue::Obj(parse_entries(tok)?),
            Some(Token::CloseBrace) | None => return Err(VdfError::UnexpectedEof),
        };
        entries.push((key, value));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_flat_object() {
        let src = r#"
            "AppState"
            {
                "appid"     "440"
                "name"      "Team Fortress 2"
                "installdir"        "Team Fortress 2"
            }
        "#;
        let root = parse(src).unwrap();
        let app_state = root.get("AppState").unwrap();
        let flat = app_state.flat_strings();
        assert_eq!(flat.get("appid").unwrap(), "440");
        assert_eq!(flat.get("name").unwrap(), "Team Fortress 2");
        assert_eq!(flat.get("installdir").unwrap(), "Team Fortress 2");
    }

    #[test]
    fn parses_nested_library_folders() {
        let src = r#"
            "libraryfolders"
            {
                "0"
                {
                    "path"        "/home/user/.steam/steam"
                    "label"       ""
                    "apps"
                    {
                        "440"        "123456789"
                        "730"        "987654321"
                    }
                }
                "1"
                {
                    "path"        "/mnt/games/SteamLibrary"
                    "apps"
                    {
                        "570"       "555"
                    }
                }
            }
        "#;
        let root = parse(src).unwrap();
        let folders = root.get("libraryfolders").unwrap().as_obj().unwrap();
        assert_eq!(folders.len(), 2);
        let (_, first) = &folders[0];
        assert_eq!(first.get("path").unwrap().as_str().unwrap(), "/home/user/.steam/steam");
        let apps = first.get("apps").unwrap().as_obj().unwrap();
        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn handles_comments_and_case_insensitive_keys() {
        let src = r#"
            // this is a comment
            "AppState"
            {
                "AppID" "440" // trailing comment
            }
        "#;
        let root = parse(src).unwrap();
        let app_state = root.get("appstate").unwrap();
        assert_eq!(app_state.get("appid").unwrap().as_str().unwrap(), "440");
    }

    #[test]
    fn handles_escaped_quotes() {
        let src = r#"
            "name" "Some \"Quoted\" Game"
        "#;
        let root = parse(src).unwrap();
        assert_eq!(root.get("name").unwrap().as_str().unwrap(), "Some \"Quoted\" Game");
    }
}
