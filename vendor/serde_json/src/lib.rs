use std::collections::BTreeMap;
use std::ops::Index;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    String(String),
    Number(usize),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

static NULL: Value = Value::Null;

impl Value {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut parser = Parser::new(input);
        let value = parser.parse_value()?;
        parser.skip_ws();
        if parser.is_done() {
            Ok(value)
        } else {
            Err("trailing JSON input".to_string())
        }
    }

    pub fn field_value(&self, key: &str) -> Result<&Value, String> {
        match self {
            Value::Object(fields) => fields
                .get(key)
                .ok_or_else(|| format!("missing JSON field `{key}`")),
            _ => Err("expected JSON object".to_string()),
        }
    }

    pub fn field_string(&self, key: &str) -> Result<String, String> {
        match self.field_value(key)? {
            Value::String(value) => Ok(value.clone()),
            _ => Err(format!("field `{key}` must be a string")),
        }
    }

    pub fn field_option_string(&self, key: &str) -> Result<Option<String>, String> {
        match self.field_value(key)? {
            Value::String(value) => Ok(Some(value.clone())),
            Value::Null => Ok(None),
            _ => Err(format!("field `{key}` must be a string or null")),
        }
    }

    pub fn field_usize(&self, key: &str) -> Result<usize, String> {
        match self.field_value(key)? {
            Value::Number(value) => Ok(*value),
            _ => Err(format!("field `{key}` must be a number")),
        }
    }

    pub fn field_array(&self, key: &str) -> Result<&[Value], String> {
        match self.field_value(key)? {
            Value::Array(values) => Ok(values),
            _ => Err(format!("field `{key}` must be an array")),
        }
    }

    pub fn to_compact_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::String(value) => format!("\"{}\"", escape_string(value)),
            Value::Number(value) => value.to_string(),
            Value::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::to_compact_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Object(fields) => format!(
                "{{{}}}",
                fields
                    .iter()
                    .map(|(key, value)| format!(
                        "\"{}\":{}",
                        escape_string(key),
                        value.to_compact_string()
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

impl Index<&str> for Value {
    type Output = Value;

    fn index(&self, index: &str) -> &Self::Output {
        match self {
            Value::Object(fields) => fields.get(index).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, Value::String(value) if value == other)
    }
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        matches!(self, Value::String(value) if value == other)
    }
}

pub fn to_string_pretty<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, String> {
    Ok(value.to_json_pretty(0))
}

pub fn from_str<T: serde::Deserialize>(input: &str) -> Result<T, String> {
    T::from_json_str(input)
}

pub fn escape_string(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch => escaped.push(ch),
        }
    }
    escaped
}

struct Parser<'a> {
    chars: Vec<char>,
    position: usize,
    _input: &'a str,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().collect(),
            position: 0,
            _input: input,
        }
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(Value::String),
            Some('n') => self.parse_null(),
            Some(ch) if ch.is_ascii_digit() => self.parse_number().map(Value::Number),
            Some(ch) => Err(format!("unexpected JSON character `{ch}`")),
            None => Err("unexpected end of JSON input".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut fields = BTreeMap::new();
        self.skip_ws();
        if self.consume('}') {
            return Ok(Value::Object(fields));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.parse_value()?;
            fields.insert(key, value);
            self.skip_ws();
            if self.consume('}') {
                break;
            }
            self.expect(',')?;
        }
        Ok(Value::Object(fields))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.consume(']') {
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_ws();
            if self.consume(']') {
                break;
            }
            self.expect(',')?;
        }
        Ok(Value::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut value = String::new();
        while let Some(ch) = self.next() {
            match ch {
                '"' => return Ok(value),
                '\\' => {
                    let escaped = self
                        .next()
                        .ok_or_else(|| "unterminated JSON escape".to_string())?;
                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        other => return Err(format!("unsupported JSON escape `{other}`")),
                    }
                }
                other => value.push(other),
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn parse_null(&mut self) -> Result<Value, String> {
        for expected in ['n', 'u', 'l', 'l'] {
            self.expect(expected)?;
        }
        Ok(Value::Null)
    }

    fn parse_number(&mut self) -> Result<usize, String> {
        let start = self.position;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.position += 1;
        }
        self.chars[start..self.position]
            .iter()
            .collect::<String>()
            .parse::<usize>()
            .map_err(|error| format!("invalid JSON number: {error}"))
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.position += 1;
        }
    }

    fn is_done(&self) -> bool {
        self.position == self.chars.len()
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(format!("expected JSON character `{expected}`"))
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.position += 1;
        Some(ch)
    }
}

impl serde::Deserialize for Value {
    fn from_json_str(input: &str) -> Result<Self, String> {
        Value::parse(input)
    }
}
