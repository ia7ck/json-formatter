use crate::ast::{Key, Pair, Value};
use anyhow::{anyhow, bail, ensure, Result};
use std::io::{BufRead, Lines};

pub struct Parser<R> {
    pos: usize,
    lines: Lines<R>,
    line: Option<Vec<char>>,
}

impl<R: BufRead> Parser<R> {
    pub fn new(reader: R) -> Self {
        let mut lines = reader.lines();
        let line = lines.next().transpose().unwrap();
        Self {
            pos: 0,
            lines,
            line: line.map(|l| l.chars().collect()),
        }
    }
    fn succ(&mut self) {
        self.pos += 1;
        while let Some(line) = self.line.as_ref() {
            if self.pos < line.len() {
                break;
            }
            // move to next line
            self.pos = 0; // reset
            let line = self.lines.next().transpose().unwrap();
            self.line = line.map(|s| s.chars().collect());
        }
    }
    fn get_cur_char(&self) -> Option<&char> {
        self.line.as_ref().and_then(|l| l.get(self.pos))
    }
    fn cur_char_is(&self, ch: char) -> bool {
        match self.get_cur_char() {
            Some(&c) => c == ch,
            None => false,
        }
    }
    fn expect_char(&self, expect: char) -> Result<()> {
        let &actual = self
            .get_cur_char()
            .ok_or(anyhow!("expected `{}`", expect))?;
        ensure!(
            expect == actual,
            format!("expected: `{}`, found: `{}`", expect, actual)
        );
        Ok(())
    }
    // check current token and skip it
    fn consume_char(&mut self, ch: char) -> Result<()> {
        self.expect_char(ch)?;
        self.succ();
        Ok(())
    }
    fn skip_whitespace(&mut self) {
        while self.get_cur_char().map_or(false, |ch| ch.is_whitespace()) {
            self.succ();
        }
    }
    fn parse_inner_string(&mut self) -> Result<String> {
        self.consume_char('"')?; // left quotes
        let mut s = String::new();
        while let Some(&ch) = self.get_cur_char() {
            if ch == '"' {
                break;
            }
            s.push(ch);
            self.succ();
        }
        self.consume_char('"')?; // right quotes
        Ok(s)
    }
    fn parse_string_value(&mut self) -> Result<Value> {
        let s = self.parse_inner_string()?;
        Ok(Value::String(s))
    }
    fn parse_string_key(&mut self) -> Result<Key> {
        let s = self.parse_inner_string()?;
        Ok(Key(s))
    }
    fn parse_number(&mut self) -> Result<Value> {
        let mut num = String::new();
        if let Some(&ch) = self.get_cur_char() {
            if ch == '-' {
                num.push(ch);
                self.succ();
            }
        }
        let valid = |ch: char| ch.is_digit(10) || ch == '.';
        while let Some(&ch) = self.get_cur_char() {
            if valid(ch) {
                num.push(ch);
                self.succ();
            } else {
                break;
            }
        }
        Ok(Value::Number(num))
    }
    fn parse_object(&mut self) -> Result<Value> {
        self.expect_char('{')?;
        self.succ();
        self.skip_whitespace();
        if self.cur_char_is('}') {
            self.succ();
            return Ok(Value::Object(Vec::new()));
        }
        let mut pairs = vec![];
        loop {
            self.skip_whitespace();
            let key = self.parse_string_key()?;
            self.skip_whitespace();
            self.consume_char(':')?;
            let value = self.parse_value()?;
            self.skip_whitespace();
            pairs.push(Pair::new(key, value));
            match self.get_cur_char() {
                Some(',') => self.succ(),
                Some('}') => break,
                Some(&other) => bail!("expected: `,` or `}}`, found: `{}`", other),
                None => bail!("expected `,` or `}`"),
            }
        }
        self.expect_char('}')?;
        self.succ();
        Ok(Value::Object(pairs))
    }
    fn parse_array(&mut self) -> Result<Value> {
        self.consume_char('[')?;
        self.skip_whitespace();
        if self.cur_char_is(']') {
            self.succ();
            return Ok(Value::Array(Vec::new()));
        }
        let mut values = vec![];
        loop {
            let value = self.parse_value()?;
            self.skip_whitespace();
            values.push(value);
            match self.get_cur_char() {
                Some(',') => self.succ(),
                Some(']') => break,
                Some(&other) => bail!("expected: `,` or `]`, found: {}", other),
                None => bail!("expected `,` or `]`"),
            }
        }
        self.consume_char(']')?;
        Ok(Value::Array(values))
    }
    pub fn parse_value(&mut self) -> Result<Value> {
        self.skip_whitespace();
        match self.get_cur_char() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string_value(),
            Some(&ch) if (ch == '-' || ch.is_digit(10)) => self.parse_number(),
            Some(&other) => bail!("invalid token: `{}`", other),
            None => bail!("no token found"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn string(s: &str) -> Value {
        Value::String(String::from(s))
    }

    fn number(s: &str) -> Value {
        Value::Number(String::from(s))
    }

    fn key(s: &str) -> Key {
        Key(String::from(s))
    }

    #[test]
    fn test_parse_string() {
        let mut p = Parser::new(Cursor::new(r#""abc de f""#));
        let v = p.parse_string_value().unwrap();
        assert_eq!(v, string("abc de f"));
    }
    #[test]
    fn test_ng_parse_string() {
        let mut p = Parser::new(Cursor::new(r#"abc de f""#));
        assert!(p.parse_string_value().is_err());

        let mut p = Parser::new(Cursor::new(r#""abc de f"#));
        assert!(p.parse_string_value().is_err());
    }
    #[test]
    fn test_parse_number() {
        let mut p = Parser::new(Cursor::new("-123.45"));
        let v = p.parse_number().unwrap();
        assert_eq!(v, number("-123.45"));
    }
    fn test_object<F>(v: Value, f: F)
    where
        F: Fn(Vec<Pair>),
    {
        match v {
            Value::Object(pairs) => {
                f(pairs);
            }
            _ => unreachable!("not object"),
        }
    }
    #[test]
    fn test_parse_empty_object() {
        let mut p = Parser::new(Cursor::new("{     }"));
        let v: Value = p.parse_object().unwrap();
        test_object(v, |pairs: Vec<Pair>| {
            assert!(pairs.is_empty());
        });
    }
    #[test]
    fn test_parse_object() {
        let mut p = Parser::new(Cursor::new(r#"{"a" : 123,      "bc"  :"xyz"   }"#));
        let v: Value = p.parse_object().unwrap();
        test_object(v, |pairs: Vec<Pair>| {
            let expected = vec![
                Pair::new(key("a"), number("123")),
                Pair::new(key("bc"), string("xyz")),
            ];
            assert_eq!(pairs, expected);
        });
    }
    #[test]
    fn test_parse_nested_object() {
        let mut p = Parser::new(Cursor::new(
            r#"{"a": {"bc": 12345, "def": "xyz"}, "ijk": 0}"#,
        ));
        let v: Value = p.parse_object().unwrap();
        test_object(v, |pairs: Vec<Pair>| {
            let expected = vec![
                Pair::new(
                    key("a"),
                    Value::Object(vec![
                        Pair::new(key("bc"), number("12345")),
                        Pair::new(key("def"), string("xyz")),
                    ]),
                ),
                Pair::new(key("ijk"), number("0")),
            ];
            assert_eq!(pairs, expected);
        });
    }
    fn test_array<F>(v: Value, f: F)
    where
        F: Fn(Vec<Value>),
    {
        match v {
            Value::Array(values) => {
                f(values);
            }
            _ => unreachable!("not array"),
        }
    }
    #[test]
    fn test_parse_array() {
        let mut p = Parser::new(Cursor::new(r#"[1, 23, 456, "xyz"]"#));
        let v: Value = p.parse_array().unwrap();
        test_array(v, |values: Vec<Value>| {
            let expected: Vec<Value> =
                vec![number("1"), number("23"), number("456"), string("xyz")];
            assert_eq!(values, expected);
        });
    }

    #[test]
    fn test_parse_nested_array() {
        let mut p = Parser::new(Cursor::new(r#"[1, [23, 456], "xyz"]"#));
        let v: Value = p.parse_array().unwrap();
        test_array(v, |values: Vec<Value>| {
            let expected: Vec<Value> = vec![
                number("1"),
                Value::Array(vec![number("23"), number("456")]),
                string("xyz"),
            ];
            assert_eq!(values, expected);
        });
    }

    #[test]
    fn test_parse_combined_object_array() {
        let mut p = Parser::new(Cursor::new(
            r#"{"a": [{"id": 1, "text": "xxx"}, {"id": 2, "text": "yyy"}]}"#,
        ));
        let v: Value = p.parse_object().unwrap();
        test_object(v, |pairs: Vec<Pair>| {
            let expected: Vec<Pair> = vec![Pair::new(
                key("a"),
                Value::Array(vec![
                    Value::Object(vec![
                        Pair::new(key("id"), number("1")),
                        Pair::new(key("text"), string("xxx")),
                    ]),
                    Value::Object(vec![
                        Pair::new(key("id"), number("2")),
                        Pair::new(key("text"), string("yyy")),
                    ]),
                ]),
            )];
            assert_eq!(pairs, expected);
        });
    }
}
