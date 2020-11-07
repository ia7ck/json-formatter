#[derive(Debug, PartialEq)]
pub struct Key(pub String);

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq)]
pub enum Value {
    String(std::string::String),
    Number(std::string::String),
    Object(Vec<Pair>),
    Array(Vec<Value>),
}

#[derive(Debug, PartialEq)]
pub struct Pair {
    pub key: Key,
    pub value: Value,
}

impl Pair {
    pub fn new(key: Key, value: Value) -> Self {
        Self { key, value }
    }
}
