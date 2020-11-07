use crate::ast::{Pair, Value};

pub struct Formatter {
    depth: usize,
}

impl Formatter {
    pub fn new() -> Self {
        Self { depth: 0 }
    }
    fn indent(&self) -> String {
        " ".repeat(4).repeat(self.depth)
    }
    fn format_string(&self, s: String) -> String {
        format!("\"{}\"", s)
    }
    fn format_number(&self, num: String) -> String {
        num
    }
    fn format_object(&mut self, pairs: Vec<Pair>) -> String {
        if pairs.is_empty() {
            return String::from("{}");
        }
        let open_brace = '{';
        self.depth += 1;
        let inner = pairs
            .into_iter()
            .map(|p| format!("{}\"{}\": {}", self.indent(), p.key, self.format(p.value)))
            .collect::<Vec<String>>()
            .join(",\n");
        self.depth -= 1;
        let close_brace = format!("{}}}", self.indent());
        format!("{}\n{}\n{}", open_brace, inner, close_brace)
    }
    fn format_array(&mut self, values: Vec<Value>) -> String {
        if values.is_empty() {
            return String::from("[]");
        }
        let open_bracket = '[';
        self.depth += 1;
        let inner = values
            .into_iter()
            .map(|v| format!("{}{}", self.indent(), self.format(v)))
            .collect::<Vec<String>>()
            .join(",\n");
        self.depth -= 1;
        let close_bracket = format!("{}]", self.indent());
        format!("{}\n{}\n{}", open_bracket, inner, close_bracket)
    }
    pub fn format(&mut self, v: Value) -> String {
        match v {
            Value::String(s) => self.format_string(s),
            Value::Number(num) => self.format_number(num),
            Value::Object(pairs) => self.format_object(pairs),
            Value::Array(values) => self.format_array(values),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use std::io::Cursor;

    fn format(text: &str) -> String {
        let mut p = Parser::new(Cursor::new(text));
        let v = p.parse_value().unwrap();
        let mut f = Formatter::new();
        f.format(v)
    }

    #[test]
    fn test_format_string() {
        #[rustfmt::skip]
        let tests = vec![
            (r#""abc""#, r#""abc""#),
            (r#""  a bc   def""#, r#""  a bc   def""#)
        ];
        for (input, expected) in tests {
            assert_eq!(format(input), String::from(expected));
        }
    }

    #[test]
    fn test_format_number() {
        #[rustfmt::skip]
        let tests = vec![
            ("123", "123"),
            ("-123.45", "-123.45")
        ];
        for (input, expected) in tests {
            assert_eq!(format(input), String::from(expected));
        }
    }

    #[test]
    fn test_format_object() {
        #[rustfmt::skip]
        let tests = vec![
            ("{}", "{}"),
            ("{    }", "{}"),
            (
                r#"{"a":123}"#,
r#"{
    "a": 123
}"#,
            ),
            (
                r#"{"a":123,"bc":45}"#,
r#"{
    "a": 123,
    "bc": 45
}"#,
            ),
            (
                r#"{"a":123,"b":{"c":45,"d":6789}}"#,
r#"{
    "a": 123,
    "b": {
        "c": 45,
        "d": 6789
    }
}"#,
            ),
            (
                r#"{"a": {}}"#,
r#"{
    "a": {}
}"#,
            ),
            (
                r#"{"a": []}"#,
r#"{
    "a": []
}"#,
            ),
        ];
        for (input, expected) in tests {
            assert_eq!(format(input), String::from(expected));
        }
    }

    #[test]
    fn test_format_array() {
        #[rustfmt::skip]
        let tests = vec![
            ("[]", "[]"),
            ("[    ]", "[]"),
            (
                "[123]",
r#"[
    123
]"#,
            ),
            (
                "[1,23,4]",
r#"[
    1,
    23,
    4
]"#,
            ),
            (
                r#"[{"a": 1}, {"b": 23}]"#,
r#"[
    {
        "a": 1
    },
    {
        "b": 23
    }
]"#,
            ),
            (
                r#"[{"a": []}]"#,
r#"[
    {
        "a": []
    }
]"#,
            ),
        ];
        for (input, expected) in tests {
            assert_eq!(format(input), String::from(expected));
        }
    }
}
