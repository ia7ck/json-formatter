use crate::ast::{Key, Pair, Value};
use anyhow::{anyhow, bail, ensure, Result};
use std::io::{BufRead, Lines};

pub struct Parser<R> {
    pos: usize,
    lines: Lines<R>,
    line: Option<Vec<char>>,
    line_number: usize,
}

#[derive(Debug)]
pub struct ParseError {
    message: String,
    line_number: usize,
}

impl ParseError {
    pub fn new(message: String, line_number: usize) -> Self {
        Self {
            message,
            line_number,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error at line {}: {}", self.line_number, self.message)
    }
}

impl<R: BufRead> Parser<R> {
    pub fn new(reader: R) -> Self {
        let mut lines = reader.lines();
        let line = lines.next().transpose().unwrap();
        Self {
            pos: 0,
            lines,
            line: line.map(|l| l.chars().collect()),
            line_number: 1,
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
            self.line_number += 1;
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
        let &actual = self.get_cur_char().ok_or(anyhow!(ParseError::new(
            format!("expected `{}`", expect),
            self.line_number
        )))?;
        ensure!(
            expect == actual,
            ParseError::new(
                format!("expected: `{}`, found: `{}`", expect, actual),
                self.line_number
            )
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
                Some(&other) => bail!(ParseError::new(
                    format!("expected: `,` or `}}`, found: `{}`", other),
                    self.line_number
                )),
                None => bail!(ParseError::new(
                    "expected `,` or `}}`".to_string(),
                    self.line_number
                )),
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
                Some(&other) => bail!(ParseError::new(
                    format!("expected: `,` or `]`, found: {}", other),
                    self.line_number,
                )),
                None => bail!(ParseError::new(
                    "expected `,` or `]`".to_string(),
                    self.line_number
                )),
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
            Some(&other) => bail!(ParseError::new(
                format!("invalid token: `{}`", other),
                self.line_number,
            )),
            None => bail!(ParseError::new(
                "no token found".to_string(),
                self.line_number
            )),
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

    fn parser(s: &str) -> Parser<Cursor<&str>> {
        Parser::new(Cursor::new(s))
    }

    #[test]
    fn test_parse_string() {
        let test = |input: &str, result: &str| {
            let mut p = parser(input);
            let v = p.parse_string_value().unwrap();
            assert_eq!(v, string(result));
        };
        test(r#""""#, ""); // ""
        test(r#""   ""#, "   "); // "   "
        test(r#""abc de f""#, "abc de f"); // "abc de f"
        test(r#""abc\nde f""#, "abc\\nde f"); // "abc\nde f"
    }

    #[test]
    fn test_ng_parse_string() {
        let test = |input: &str| {
            let mut p = parser(input);
            assert!(p.parse_string_value().is_err());
        };
        test(r#"abc de f""#); // "abc de f""
        test(r#""abc de f"#); // ""abc de f"
    }

    #[test]
    fn test_parse_number() {
        let test = |input: &str, result: &str| {
            let mut p = parser(input);
            let v = p.parse_number().unwrap();
            assert_eq!(v, number(result));
        };
        test("-123.45", "-123.45");
        test("-0", "-0");
        test(".123", ".123");
        test("-.123", "-.123");
        test("123.", "123.");
        test("000123", "000123");

        // TODO: 落ちてほしい
        test("1.23.45", "1.23.45");
        test("-", "-");
        test("-.", "-.");
        test("123abc", "123");
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
        let mut p = parser("{     }");
        let v: Value = p.parse_object().unwrap();
        test_object(v, |pairs: Vec<Pair>| {
            assert!(pairs.is_empty());
        });
    }

    #[test]
    fn test_parse_object() {
        let mut p = parser(r#"{"a" : 123,      "bc"  :"xyz"   }"#);
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
        let mut p = parser(r#"{"a": {"bc": 12345, "def": "xyz"}, "ijk": 0}"#);
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

    #[test]
    fn test_ng_parse_object() {
        let test = |input: &str| {
            let mut p = parser(input);
            assert!(p.parse_object().is_err());
        };
        test(r#"{ , }"#);
        test(r#"{"a": 123"#); // {"a":123
        test(r#""a":123}"#); // "a":123}
        test(r#"{"a":123  "bc":"xyz"}"#); // missing comma
        test(r#"{"a"  123}"#); // missing colon
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
        let mut p = parser(r#"[1, 23, 456, "xyz"]"#);
        let v: Value = p.parse_array().unwrap();
        test_array(v, |values: Vec<Value>| {
            let expected: Vec<Value> =
                vec![number("1"), number("23"), number("456"), string("xyz")];
            assert_eq!(values, expected);
        });
    }

    #[test]
    fn test_parse_nested_array() {
        let mut p = parser(r#"[1, [23, 456], "xyz"]"#);
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
        let mut p = parser(r#"{"a": [{"id": 1, "text": "xxx"}, {"id": 2, "text": "yyy"}]}"#);
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

    #[test]
    fn test_ng_parse_array() {
        let test = |input: &str| {
            let mut p = parser(input);
            assert!(p.parse_array().is_err());
        };
        test(r#"[ , ]"#);
        test(r#"["a""#); // ["a"
        test(r#"["a","#); // ["a",
        test(r#""a"]"#); // "a"]
        test(r#","a"]"#); // ,"a"]
        test(r#"["a"  123]"#); // missing comma
    }

    #[test]
    fn test_error_line() {
        let test = |input: &str, line_number: usize| {
            let mut p = parser(input);
            match p.parse_object() {
                Err(e) => {
                    let e = e.downcast::<ParseError>().unwrap();
                    assert_eq!(e.line_number, line_number);
                }
                _ => unreachable!(),
            }
        };

        // missing colon
        #[rustfmt::skip]
            test(
r#"{
"a": 123,
"b"  45
}"#,
            3,
        );

        // missing comma
        #[rustfmt::skip]
        test(
r#"{
"a": 123
"b": 45
}"#,
            3,
        );

        // missing close brace
        #[rustfmt::skip]
            test(
r#"{
"a": 123,
"b": 45
"#,
            4,
        );
    }
}
