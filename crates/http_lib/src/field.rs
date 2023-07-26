use std::collections::HashSet;
use std::{fmt, iter, slice};

use bytes::Bytes;
use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::chars::{CRLF, CTEXT_MAP, DATE_MAP, QUOTED_TEXT_MAP, TCHAR_MAP, TOKEN_MAP};
use crate::Advance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParsingError {
    Malformed,
    IncorrectlyTerminated,
    NameMissing,
    ValueTooLong,
    ValueInvalidToken,
    ValueInvalidQuotedText,
    InvalidCommentCharacter,
}

impl ParsingError {
    pub const fn as_str(self) -> &'static str {
        use ParsingError::*;
        match self {
            Malformed => "malformed",
            IncorrectlyTerminated => "incorrectly terminated",
            NameMissing => "name missing",
            ValueTooLong => "value too long",
            ValueInvalidToken => "value contains an invalid token character",
            ValueInvalidQuotedText => "value contains invalid quoted text",
            InvalidCommentCharacter => "comment contains an invalid character",
        }
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::error::Error for ParsingError {}

#[derive(Debug)]
pub struct InvalidData;

impl fmt::Display for InvalidData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid data")
    }
}

impl std::error::Error for InvalidData {}

static DATE_FIELDS: Lazy<HashSet<Vec<u8>>> = Lazy::new(|| {
    let values = ["Date", "Last-Modified", "Expires"];
    values.into_iter().map(Vec::from).collect()
});

#[derive(Clone, Copy)]
pub struct Config<'a> {
    pub map: &'a [u8; 256],
    pub comments: bool,
    pub quotes: bool,
    pub commas: bool,
}

impl<'a> Config<'a> {
    fn validate_byte(&self, byte: u8) -> bool {
        self.map[byte as usize] != 0
            || (self.comments && (byte == b'(' || byte == b')'))
            || (self.quotes && byte == b'"')
    }
}

#[derive(Clone, Copy, Debug)]
enum ValidationError {
    Quote,
    Comment,
    Token,
    Terminated,
}

impl ValidationError {
    const fn as_str(self) -> &'static str {
        use ValidationError::*;
        match self {
            Quote => "invalid quoted text",
            Comment => "invalid comment character",
            Token => "invalid token",
            Terminated => "value terminated",
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::error::Error for ValidationError {}

struct Validator<'a> {
    quoted: bool,
    backslash: bool,
    comment_stack: i32,
    config: Config<'a>,
    error: Option<ValidationError>,
}

#[inline]
fn is_opaque(b: u8) -> bool {
    b >= 0x80
}

impl<'a> Validator<'a> {
    const fn new(config: Config<'a>) -> Self {
        Self {
            quoted: false,
            backslash: false,
            comment_stack: 0,
            error: None,
            config,
        }
    }

    /// If the byte is valid, returns whether it is opaque.
    fn advance(&mut self, b: u8) -> Result<bool, ValidationError> {
        if let Some(err) = self.error {
            return Err(err);
        }

        macro_rules! err {
            ($err:expr) => {{
                self.error = Some($err);
                Err($err)
            }};
        }

        if self.quoted {
            return if self.backslash {
                self.backslash = false;
                Ok(true)
            } else if b == b'\\' {
                self.backslash = true;
                Ok(false)
            } else if b == b'"' {
                self.quoted = false;
                Ok(false)
            } else if QUOTED_TEXT_MAP[b as usize] == 0 {
                err!(ValidationError::Quote)
            } else {
                Ok(!is_opaque(b))
            };
        }

        if self.comment_stack > 0 {
            if CTEXT_MAP[b as usize] != 0 {
                return Ok(true);
            }

            return if self.config.quotes && b == b'"' {
                self.quoted = true;
                Ok(false)
            } else if b == b'(' {
                self.comment_stack += 1;
                Ok(true)
            } else if b == b')' {
                self.comment_stack -= 1;
                Ok(true)
            } else if is_opaque(b) {
                Ok(false)
            } else {
                err!(ValidationError::Comment)
            };
        }

        if self.config.map[b as usize] != 0 {
            return Ok(true);
        }

        if self.config.quotes && b == b'"' {
            self.quoted = true;
            Ok(false)
        } else if self.config.comments && b == b'(' {
            self.comment_stack += 1;
            Ok(true)
        } else if (self.config.commas && b == b',') || b == b'\r' {
            err!(ValidationError::Terminated)
        } else {
            err!(ValidationError::Token)
        }
    }

    fn quoted(&self) -> bool {
        self.quoted
    }

    fn commented(&self) -> bool {
        self.comment_stack > 0
    }

    fn in_root_scope(&self) -> bool {
        !(self.quoted() || self.commented())
    }
}

#[derive(Default)]
pub struct Value {
    value: Bytes,
    is_valid_ascii: bool,
}

fn write_escaped_bytes(dest: &mut Vec<u8>, src: &[u8], map: &[u8; 256], escape: u8) {
    let mut extend_from = 0;
    for (i, b) in src.iter().copied().enumerate() {
        if map[b as usize] == 0 {
            dest.extend_from_slice(&src[extend_from..i]);
            dest.push(escape);
            extend_from = i;
        }
    }

    dest.extend_from_slice(&src[extend_from..]);
}

impl Value {
    pub fn from_unquoted(src: &[u8], config: Config) -> Self {
        let mut is_valid_ascii = true;
        let mut invalid_quoted_bytes = 0;
        let mut validator = Validator::new(config);
        let mut contains_opaque_data = false;

        for b in src.iter().copied() {
            is_valid_ascii &= b < 0x80;

            if QUOTED_TEXT_MAP[b as usize] == 0 {
                invalid_quoted_bytes += 1;
            }

            match validator.advance(b) {
                Ok(true) => (),
                _ => contains_opaque_data = true,
            }
        }

        if !contains_opaque_data && validator.in_root_scope() {
            Self {
                value: src.to_vec().into(),
                is_valid_ascii,
            }
        } else {
            let mut value = Vec::with_capacity(src.len() + invalid_quoted_bytes + 2);
            value.push(b'"');
            write_escaped_bytes(&mut value, src, &QUOTED_TEXT_MAP, b'\\');
            value.push(b'"');

            Self {
                value: value.into(),
                is_valid_ascii,
            }
        }
    }

    fn from_bytes(bytes: &mut Bytes, config: Config) -> Result<Self, ParsingError> {
        const MAX_LEN: usize = 100_000;
        let mut is_valid_ascii = true;
        let mut validator = Validator::new(config);
        for (i, &b) in bytes.iter().enumerate() {
            if i >= MAX_LEN {
                return Err(ParsingError::ValueTooLong);
            }

            is_valid_ascii &= b < 0x80;

            match validator.advance(b) {
                Ok(_) => (),
                Err(ValidationError::Terminated) => {
                    return Ok(Self {
                        value: bytes.split_to(i),
                        is_valid_ascii,
                    })
                }
                Err(ValidationError::Token) => return Err(ParsingError::ValueInvalidToken),
                Err(ValidationError::Comment) => return Err(ParsingError::InvalidCommentCharacter),
                Err(ValidationError::Quote) => return Err(ParsingError::ValueInvalidQuotedText),
            }
        }

        Err(ParsingError::IncorrectlyTerminated)
    }

    unsafe fn from_raw(value: Bytes, is_valid_ascii: bool) -> Self {
        Self {
            value,
            is_valid_ascii,
        }
    }

    fn new(value: Bytes, config: &Config) -> Self {
        let is_valid_ascii = value.iter().all(|&b| config.validate_byte(b));
        Self {
            value,
            is_valid_ascii,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.value
    }

    pub fn as_str(&self) -> Option<&str> {
        if self.is_valid_ascii {
            // Safety: `is_valid_ascii` makes this safe, however it must always
            // be checked before a value is created.
            Some(unsafe { std::str::from_utf8_unchecked(&self.value) })
        } else {
            None
        }
    }

    pub fn unquote(&self, config: Config) -> Result<Vec<u8>, InvalidData> {
        let mut res = Vec::with_capacity(self.value.len());

        let mut extend_from = 0;
        let mut validator = Validator::new(config);

        for (i, b) in self.value.iter().copied().enumerate() {
            match validator.advance(b) {
                Ok(true) => (),
                Ok(false) => {
                    res.extend_from_slice(&self.value[extend_from..i]);
                    extend_from = i + 1;
                }
                Err(_) => return Err(InvalidData),
            }
        }

        if extend_from < self.value.len() {
            res.extend_from_slice(&self.value[extend_from..]);
        }

        Ok(res)
    }
}

pub struct Values<'a> {
    first: Value,
    extra: Vec<Value>,
    config: Config<'a>,
}

type ExtraSlicesIter<'a> = iter::Map<slice::Iter<'a, Value>, for<'b> fn(&'b Value) -> &'b [u8]>;
type SlicesIter<'a> = iter::Chain<iter::Once<&'a [u8]>, ExtraSlicesIter<'a>>;
impl<'a> Values<'a> {
    pub fn new(first: Bytes, config: Config<'a>) -> Self {
        Self {
            first: Value::new(first, &config),
            extra: Vec::new(),
            config,
        }
    }

    pub fn push(&mut self, value: Bytes) {
        self.extra.push(Value::new(value, &self.config));
    }

    fn extend_from_bytes(&mut self, bytes: &mut Bytes) -> Result<(), ParsingError> {
        while {
            bytes.advance_while(|&b| b == b' ');
            self.extra.push(Value::from_bytes(bytes, self.config)?);
            bytes.advance_byte(b',')
        } {}

        Ok(())
    }

    fn from_bytes(bytes: &mut Bytes, config: Config<'a>) -> Result<Self, ParsingError> {
        bytes.advance_while(|&b| b == b' ');

        let mut this = Self {
            first: Value::from_bytes(bytes, config)?,
            extra: Vec::new(),
            config,
        };

        if bytes.advance_byte(b',') {
            this.extend_from_bytes(bytes)?;
        }

        Ok(this)
    }

    pub fn first(&self) -> &Value {
        &self.first
    }

    pub fn first_slice(&self) -> &[u8] {
        self.first.as_slice()
    }

    pub fn extra(&self) -> &[Value] {
        &self.extra
    }

    pub fn extra_slice(&self) -> Vec<&[u8]> {
        self.extra.iter().map(Value::as_slice).collect()
    }

    pub fn is_single(&self) -> bool {
        self.extra.is_empty()
    }

    pub fn get_refs(&self) -> (&Value, &[Value]) {
        (self.first(), self.extra())
    }

    pub fn get_slices(&self) -> (&[u8], Vec<&[u8]>) {
        (self.first_slice(), self.extra_slice())
    }

    pub fn iter_refs(&self) -> iter::Chain<iter::Once<&Value>, slice::Iter<'_, Value>> {
        iter::once(self.first()).chain(self.extra().iter())
    }

    pub fn iter_slices(&self) -> SlicesIter<'_> {
        const AS_SLICE_PTR: for<'b> fn(&'b Value) -> &'b [u8] = Value::as_slice;
        iter::once(self.first_slice()).chain(self.extra().iter().map(AS_SLICE_PTR))
    }

    pub fn count(&self) -> usize {
        self.extra.len() + 1
    }

    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        let mut first = true;
        for value in self.iter_refs() {
            if !first {
                buffer.extend_from_slice(b", ");
            }

            buffer.extend(value.as_slice());
            first = false;
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let capacity = self.iter_refs().map(|v| v.as_slice().len() + 2).sum();
        let mut vec = Vec::with_capacity(capacity);
        self.write_to_buffer(&mut vec);
        vec
    }
}

fn config_for_name(name: &[u8]) -> Config<'static> {
    let map = if DATE_FIELDS.contains(name) {
        &DATE_MAP
    } else {
        &TOKEN_MAP
    };
    Config {
        map,
        comments: true,
        quotes: true,
        commas: true,
    }
}

fn field_name_from_bytes(bytes: &mut Bytes) -> Bytes {
    let field_name_len = bytes
        .iter()
        .copied()
        .take_while(|&c| TCHAR_MAP[c as usize] != 0)
        .take(1024)
        .count();
    bytes.split_to(field_name_len)
}

fn write_field_to_buffer(buffer: &mut Vec<u8>, name: &[u8], values: &Values) {
    buffer.extend_from_slice(name);
    buffer.extend_from_slice(b": ");
    values.write_to_buffer(buffer);
}

pub struct Fields(IndexMap<Bytes, Values<'static>>);

impl Fields {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(IndexMap::with_capacity(capacity))
    }

    pub fn from_bytes(bytes: &mut Bytes) -> Result<Self, ParsingError> {
        let mut fields: IndexMap<Bytes, Values> = IndexMap::new();

        while !bytes.starts_with(CRLF) && bytes.first().is_some_and(u8::is_ascii_alphanumeric) {
            let name = field_name_from_bytes(bytes);
            if name.is_empty() {
                return Err(ParsingError::NameMissing);
            }

            if !bytes.advance_byte(b':') {
                return Err(ParsingError::Malformed);
            }

            bytes.advance_byte(b' ');

            let config = config_for_name(&name);
            if let Some(values) = fields.get_mut(&name) {
                values.extend_from_bytes(bytes)?;
            } else {
                fields.insert(name, Values::from_bytes(bytes, config)?);
            }

            if !bytes.advance_bytes(CRLF) {
                return Err(ParsingError::Malformed);
            }
        }

        if !bytes.advance_bytes(CRLF) {
            return Err(ParsingError::IncorrectlyTerminated);
        }

        Ok(Self(fields))
    }

    pub fn copy_from_str<const N: usize>(raw_fields: [(&str, &[&str]); N]) -> Self {
        let inner = raw_fields
            .iter()
            .map(|(n, vs)| {
                let config = config_for_name(n.as_bytes());
                let mut values = Values::new(vs[0].to_string().into(), config);
                for v in &vs[1..] {
                    values.push((*v).to_string().into());
                }
                ((*n).to_string().into(), values)
            })
            .collect();
        Self(inner)
    }

    pub fn add_header_value(&mut self, name: Bytes, value: Bytes) {
        if let Some(values) = self.0.get_mut(&name) {
            values.push(value);
        } else {
            let config = config_for_name(&name);
            self.0.insert(name, Values::new(value, config));
        }
    }

    pub fn get(&self, name: &[u8]) -> Option<&Values<'static>> {
        self.0.get(name)
    }

    pub fn get_single(&self, name: &[u8]) -> Option<&[u8]> {
        if let Some(values) = self.get(name) {
            if values.extra().is_empty() {
                Some(values.first().as_slice())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn contains_name(&self, name: &[u8]) -> bool {
        self.0.contains_key(name)
    }

    pub fn contains_value(&self, name: &[u8], value: &[u8]) -> bool {
        self.0
            .get(name)
            .is_some_and(|vs| vs.iter_slices().any(|v| v == value))
    }

    pub fn contains_value_exact(&self, name: &[u8], value: &[u8]) -> bool {
        self.0
            .get(name)
            .is_some_and(|v| v.is_single() && v.first_slice() == value)
    }

    pub fn contains_values<const N: usize>(&self, name: &[u8], values: [&[u8]; N]) -> bool {
        self.0.get(name).is_some_and(|vs| {
            values
                .into_iter()
                .all(|ex| vs.iter_slices().any(|v| v == ex))
        })
    }

    pub fn contains_values_exact<const N: usize>(&self, name: &[u8], values: [&[u8]; N]) -> bool {
        self.0.get(name).is_some_and(|vs| {
            vs.count() == values.len()
                && values
                    .into_iter()
                    .all(|ex| vs.iter_slices().any(|v| v == ex))
        })
    }

    pub fn get_slices(&self, name: &[u8]) -> Option<(&[u8], Vec<&[u8]>)> {
        self.0.get(name).map(Values::get_slices)
    }

    pub fn from_inner(inner: IndexMap<Bytes, Values<'static>>) -> Self {
        Self(inner)
    }

    pub fn as_inner(&self) -> &IndexMap<Bytes, Values<'static>> {
        &self.0
    }

    pub fn into_inner(self) -> IndexMap<Bytes, Values<'static>> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        for (name, values) in &self.0 {
            write_field_to_buffer(buffer, name, values);
            buffer.extend_from_slice(CRLF);
        }

        if !self.is_empty() {
            buffer.extend_from_slice(CRLF);
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        self.write_to_buffer(&mut buffer);
        buffer
    }
}

impl Default for Fields {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    const SIMPLE_STRINGIFIED: &str = "\
        Host: example.com\r\n\
        User-Agent: curl/8.1.2\r\n\
        Accept: */*\r\n\
        \r\n";

    const SIMPLE_INTERNAL: [(&str, &[&str]); 3] = [
        ("Host", &["example.com"]),
        ("User-Agent", &["curl/8.1.2"]),
        ("Accept", &["*/*"]),
    ];

    const QUOTED_STRINGIFIED: &str = "\
        Host: example.com\r\n\
        Example-Dates: \"Sat, 04 May 1996\", \"Wed, 14 Sep 2005\"\r\n\
        Accept: */*\r\n\
        Backslash-Test: \"with a \\\\ backslash and a \\\" quote\"\r\n\
        User-Agent: curl/8.1.2\r\n\
        \r\n";

    const QUOTED_INTERNAL: [(&str, &[&str]); 5] = [
        ("Host", &["example.com"]),
        (
            "Example-Dates",
            &["\"Sat, 04 May 1996\"", "\"Wed, 14 Sep 2005\""],
        ),
        ("Accept", &["*/*"]),
        (
            "Backslash-Test",
            &["\"with a \\\\ backslash and a \\\" quote\""],
        ),
        ("User-Agent", &["curl/8.1.2"]),
    ];

    const CHROME_STRINGIFIED: &str = "\
        Host: localhost:8000\r\n\
        Connection: keep-alive\r\n\
        Pragma: no-cache\r\n\
        Cache-Control: no-cache\r\n\
        sec-ch-ua: \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"114\", \"Google Chrome\";v=\"114\"\r\n\
        sec-ch-ua-mobile: ?0\r\n\
        sec-ch-ua-platform: \"Linux\"\r\n\
        DNT: 1\r\n\
        Upgrade-Insecure-Requests: 1\r\n\
        User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36\r\n\
        Accept: text/html, application/xhtml+xml, application/xml;q=0.9, image/avif, image/webp, image/apng, */*;q=0.8, application/signed-exchange;v=b3;q=0.7\r\n\
        Sec-Fetch-Site: cross-site\r\n\
        Sec-Fetch-Mode: navigate\r\n\
        Sec-Fetch-User: ?1\r\n\
        Sec-Fetch-Dest: document\r\n\
        Accept-Encoding: gzip, deflate, br\r\n\
        Accept-Language: en-US, en;q=0.9, pl-PL;q=0.8, pl;q=0.7\r\n\
        \r\n";

    const CHROME_INTERNAL: [(&str, &[&str]); 17] = [
        ("Host", &["localhost:8000"]),
        ("Connection", &["keep-alive"]),
        ("Pragma", &["no-cache"]),
        ("Cache-Control", &["no-cache"]),
        ("sec-ch-ua", &[
            "\"Not.A/Brand\";v=\"8\"",
            "\"Chromium\";v=\"114\"",
            "\"Google Chrome\";v=\"114\""
        ]),
        ("sec-ch-ua-mobile", &["?0"]),
        ("sec-ch-ua-platform", &["\"Linux\""]),
        ("DNT", &["1"]),
        ("Upgrade-Insecure-Requests", &["1"]),
        ("User-Agent", &["Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36"]),
        ("Accept", &[
            "text/html",
            "application/xhtml+xml",
            "application/xml;q=0.9",
            "image/avif",
            "image/webp",
            "image/apng",
            "*/*;q=0.8",
            "application/signed-exchange;v=b3;q=0.7"
        ]),
        ("Sec-Fetch-Site", &["cross-site"]),
        ("Sec-Fetch-Mode", &["navigate"]),
        ("Sec-Fetch-User", &["?1"]),
        ("Sec-Fetch-Dest", &["document"]),
        ("Accept-Encoding", &["gzip", "deflate", "br"]),
        ("Accept-Language", &["en-US", "en;q=0.9", "pl-PL;q=0.8", "pl;q=0.7"]),
    ];

    pub fn assert_headers(actual: &Fields, expected: &[(&str, &[&str])]) {
        assert_eq!(actual.len(), expected.len());
        for (expected_key, expected_values) in expected {
            let Some(actual_values) = actual.get(expected_key.as_bytes()) else {
                panic!("field \"{expected_key}\" does not exist");
            };
            actual_values
                .iter_refs()
                .map(Value::as_str)
                .zip(expected_values.iter().copied())
                .for_each(|(av, ev)| {
                    if let Some(av) = av {
                        assert_eq!(av, ev);
                    }
                });
        }
    }

    #[test]
    fn simple_from_bytes() {
        let mut bytes = SIMPLE_STRINGIFIED.into();
        let actual = Fields::from_bytes(&mut bytes).unwrap();
        assert_headers(&actual, &SIMPLE_INTERNAL);
    }

    #[test]
    fn simple_to_string() {
        let actual = String::from_utf8(Fields::copy_from_str(SIMPLE_INTERNAL).to_buffer()).unwrap();
        assert_eq!(actual, SIMPLE_STRINGIFIED);
    }

    #[test]
    fn quoted_from_bytes() {
        let mut bytes = QUOTED_STRINGIFIED.into();
        let actual = Fields::from_bytes(&mut bytes).unwrap();
        assert_headers(&actual, &QUOTED_INTERNAL);
    }

    #[test]
    fn quoted_to_string() {
        let actual = String::from_utf8(Fields::copy_from_str(QUOTED_INTERNAL).to_buffer()).unwrap();
        assert_eq!(actual, QUOTED_STRINGIFIED);
    }

    #[test]
    fn chrome_from_bytes() {
        let mut bytes = CHROME_STRINGIFIED.into();
        let actual = Fields::from_bytes(&mut bytes).unwrap();
        assert_headers(&actual, &CHROME_INTERNAL);
    }

    #[test]
    fn chrome_to_buffer() {
        let actual = String::from_utf8(Fields::copy_from_str(CHROME_INTERNAL).to_buffer()).unwrap();
        assert_eq!(actual, CHROME_STRINGIFIED);
    }
}
