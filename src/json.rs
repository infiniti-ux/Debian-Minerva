//! Minimal, dependency-free JSON implementation.
//!
//! Debian Minerva deliberately has no crates.io dependencies: `serde_json` is
//! the only thing a session daemon really needs, and a ~250 line parser keeps
//! the packaging story trivial (`dh-cargo` with an empty vendor dir).

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    /// Object kept as an ordered vector: JSON output stays deterministic,
    /// which makes the on-disk session diffable.
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn obj(items: Vec<(&str, Value)>) -> Value {
        Value::Obj(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    pub fn arr(items: Vec<Value>) -> Value {
        Value::Arr(items)
    }
    pub fn s(v: impl Into<String>) -> Value {
        Value::Str(v.into())
    }
    pub fn n(v: f64) -> Value {
        Value::Num(v)
    }

    pub fn get(&self, key: &str) -> Value {
        match self {
            Value::Obj(items) => items
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Null),
            _ => Value::Null,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Value::Str(s) => s,
            _ => "",
        }
    }
    pub fn as_f64(&self) -> f64 {
        match self {
            Value::Num(n) => *n,
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Str(s) => s.parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }
    pub fn as_i64(&self) -> i64 {
        self.as_f64() as i64
    }
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0,
            Value::Str(s) => s == "true" || s == "1",
            _ => false,
        }
    }
    pub fn as_arr(&self) -> &[Value] {
        match self {
            Value::Arr(a) => a,
            _ => &[],
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

pub fn stringify(v: &Value) -> String {
    let mut out = String::new();
    write_value(v, &mut out);
    out
}

fn write_value(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                let _ = write!(out, "{}", *n as i64);
            } else if n.is_finite() {
                let _ = write!(out, "{}", n);
            } else {
                out.push_str("null");
            }
        }
        Value::Str(s) => write_string(s, out),
        Value::Arr(a) => {
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Obj(o) => {
            out.push('{');
            for (i, (k, val)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_value(val, out);
            }
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn parse(input: &str) -> Result<Value, String> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let v = p.value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(format!("trailing data at byte {}", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.bump() == Some(c) {
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.pos))
        }
    }
    fn literal(&mut self, word: &str) -> Result<(), String> {
        for expected in word.bytes() {
            if self.bump() != Some(expected) {
                return Err(format!("invalid literal at byte {}", self.pos));
            }
        }
        Ok(())
    }
    fn value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => {
                self.literal("true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(Value::Bool(false))
            }
            Some(b'n') => {
                self.literal("null")?;
                Ok(Value::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(c) => Err(format!("unexpected byte '{}' at {}", c as char, self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }
    fn object(&mut self) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.pos += 1;
                break;
            }
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            let val = self.value()?;
            items.push((key, val));
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.pos)),
            }
        }
        Ok(Value::Obj(items))
    }
    fn array(&mut self) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                break;
            }
            items.push(self.value()?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => break,
                _ => return Err(format!("expected ',' or ']' at byte {}", self.pos)),
            }
        }
        Ok(Value::Arr(items))
    }
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let c = self.bump().ok_or("unterminated string")?;
            match c {
                b'"' => break,
                b'\\' => {
                    let esc = self.bump().ok_or("unterminated escape")?;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            if (0xD800..=0xDBFF).contains(&cp) {
                                // surrogate pair
                                if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                    return Err("invalid surrogate pair".into());
                                }
                                let low = self.hex4()?;
                                let full =
                                    0x10000 + ((cp - 0xD800) << 10) + (low.saturating_sub(0xDC00));
                                if let Some(ch) = char::from_u32(full) {
                                    out.push(ch);
                                }
                            } else if let Some(ch) = char::from_u32(cp) {
                                out.push(ch);
                            }
                        }
                        _ => return Err(format!("invalid escape at byte {}", self.pos)),
                    }
                }
                c if c < 0x20 => return Err(format!("control char in string at {}", self.pos)),
                _ => {
                    // Consume a full UTF-8 codepoint.
                    let start = self.pos - 1;
                    let len = utf8_len(c);
                    for _ in 1..len {
                        self.bump();
                    }
                    let slice = &self.bytes[start..self.pos];
                    out.push_str(std::str::from_utf8(slice).unwrap_or("\u{fffd}"));
                }
            }
        }
        Ok(out)
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let c = self.bump().ok_or("short unicode escape")?;
            v = v * 16
                + match c {
                    b'0'..=b'9' => (c - b'0') as u32,
                    b'a'..=b'f' => (c - b'a' + 10) as u32,
                    b'A'..=b'F' => (c - b'A' + 10) as u32,
                    _ => return Err("bad unicode escape".into()),
                };
        }
        Ok(v)
    }
    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("0");
        text.parse::<f64>()
            .map(Value::Num)
            .map_err(|_| format!("invalid number at byte {}", start))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let src = r#"{"name":"Minerva","n":42,"f":1.5,"ok":true,"none":null,"a":[1,"é",{"x":false}]}"#;
        let v = parse(src).unwrap();
        assert_eq!(v.get("name").as_str(), "Minerva");
        assert_eq!(v.get("n").as_i64(), 42);
        assert_eq!(v.get("a").as_arr()[1].as_str(), "é");
        assert_eq!(v.get("a").as_arr()[2].get("x").as_bool(), false);
        let out = stringify(&v);
        assert_eq!(parse(&out).unwrap(), v);
    }

    #[test]
    fn escapes() {
        let v = parse(r#""a\nb\t\"\\""#).unwrap();
        assert_eq!(v.as_str(), "a\nb\t\"\\");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("{\"a\":}").is_err());
        assert!(parse("[1,2").is_err());
        assert!(parse("nope").is_err());
    }
}
