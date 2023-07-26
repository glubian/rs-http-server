use std::fmt;

use bytes::Bytes;

use crate::{
    chars::{CRLF, URI_MAP},
    field, version, Fields, Method, Version,
};
use crate::Advance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParsingError {
    VersionMalformed,
    MethodUnsupported,
    MalformedStartLine,
    InvalidResource,
    Header(field::ParsingError),
    BodyLongerThanStream,
    Trailer(field::ParsingError),
}

impl ParsingError {
    pub const fn as_str(self) -> &'static str {
        use field::ParsingError as field;
        use ParsingError::*;
        match self {
            VersionMalformed => version::Malformed::DESCRIPTION,
            MethodUnsupported => "unsupported method",
            MalformedStartLine => "start line is malformed",
            InvalidResource => "invalid resource",
            Header(field::Malformed) => "malformed header",
            Header(field::IncorrectlyTerminated) => "incorrectly terminated header",
            Header(field::NameMissing) => "header name is missing",
            Header(field::ValueTooLong) => "header value too long",
            Header(field::ValueInvalidToken) => "header value contains an invalid token character",
            Header(field::ValueInvalidQuotedText) => "header value contains invalid quoted text",
            Header(field::InvalidCommentCharacter) => {
                "header comment contains an invalid character"
            }
            BodyLongerThanStream => "stream ended before Content-Length was reached",
            Trailer(field::Malformed) => "malformed trailer",
            Trailer(field::IncorrectlyTerminated) => "incorrectly terminated trailer",
            Trailer(field::NameMissing) => "trailer name is missing",
            Trailer(field::ValueTooLong) => "trailer value too long",
            Trailer(field::ValueInvalidToken) => {
                "trailer value contains an invalid token character"
            }
            Trailer(field::ValueInvalidQuotedText) => "trailer value contains invalid quoted text",
            Trailer(field::InvalidCommentCharacter) => {
                "trailer comment contains an invalid character"
            }
        }
    }
}

impl From<version::Malformed> for ParsingError {
    fn from(_: version::Malformed) -> Self {
        Self::VersionMalformed
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::error::Error for ParsingError {}

struct StartLine {
    method: Method,
    path: Bytes,
    version: Version,
}

impl StartLine {
    fn from_bytes(bytes: &mut Bytes) -> Result<Self, ParsingError> {
        let Some(method) = Method::from_bytes(bytes) else {
            return Err(ParsingError::MethodUnsupported);
        };

        if !bytes.advance_byte(b' ') {
            return Err(ParsingError::MalformedStartLine);
        }

        let path = bytes.split_while(|&c| URI_MAP[c as usize] != 0);
        if path.is_empty() {
            return Err(ParsingError::InvalidResource);
        }

        if !bytes.advance_byte(b' ') {
            return Err(ParsingError::MalformedStartLine);
        }

        let version = Version::from_bytes(bytes)?;

        if !bytes.advance_bytes(CRLF) {
            return Err(ParsingError::MalformedStartLine);
        }

        Ok(Self {
            method,
            path,
            version,
        })
    }
}

pub struct Request {
    pub method: Method,
    pub path: Bytes,
    pub version: Version,
    pub headers: Fields,
    pub body: Bytes,
    pub trailers: Fields,
}

impl Request {
    pub fn builder(host: Bytes, method: Method, path: Bytes) -> Builder {
        Builder::new(host, method, path)
    }

    pub fn new(host: Bytes, method: Method, path: Bytes) -> Self {
        let mut headers = Fields::new();
        headers.add_header_value("Host".into(), host);
        Self {
            method,
            path,
            version: Version(1, 1),
            headers,
            body: Bytes::new(),
            trailers: Fields::new(),
        }
    }

    pub fn add_header_value(&mut self, name: Bytes, value: Bytes) {
        self.headers.add_header_value(name, value);   
    }

    pub fn body(&mut self, body: String) {
        self.body_of_type(body.into(), "text/plain".into());
    }

    pub fn no_body(&mut self, body: &str) {
        self.no_body_of_type(body.as_bytes(), "text/plain".into());
    }

    pub fn body_of_type(&mut self, body: Bytes, content_type: Bytes) {
        self.no_body_of_type(&body, content_type);
        self.body = body;
    }

    pub fn no_body_of_type(&mut self, body: &[u8], content_type: Bytes) {
        self.add_header_value("Content-Length".into(), body.len().to_string().into());
        self.add_header_value("Content-Type".into(), content_type);
    }

    pub fn from_bytes(bytes: &mut Bytes) -> Result<Self, ParsingError> {
        let StartLine {
            method,
            path,
            version,
        } = StartLine::from_bytes(bytes)?;
        let headers = Fields::from_bytes(bytes).map_err(ParsingError::Header)?;
        let content_length = headers.get("Content-Length".as_bytes()).map_or(0, |c| {
            std::str::from_utf8(c.get_refs().0.as_slice())
                .unwrap_or("")
                .parse()
                .unwrap_or(0)
        });

        if content_length < bytes.len() {
            return Err(ParsingError::BodyLongerThanStream);
        }

        let body = bytes.split_to(content_length);

        Ok(Self {
            method,
            path,
            version,
            headers,
            body,
            trailers: Fields::new(),
        })
    }

    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(self.method.as_bytes());
        buffer.push(b' ');
        buffer.extend_from_slice(&self.path);
        buffer.push(b' ');
        self.version.write_to_buffer(buffer);
        buffer.extend_from_slice(CRLF);

        self.headers.write_to_buffer(buffer);
        buffer.extend_from_slice(&self.body);
        self.trailers.write_to_buffer(buffer);
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(256);
        self.write_to_buffer(&mut buffer);
        buffer
    }
}

pub struct Builder {
    request: Box<Request>,
}

impl Builder {
    pub fn new(host: Bytes, method: Method, path: Bytes) -> Self {
        Self {
            request: Box::new(Request::new(host, method, path))
        }
    }

    pub fn add_header_value(mut self, name: Bytes, value: Bytes) -> Self {
        self.request.add_header_value(name, value);
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.request.body(body);
        self
    }

    pub fn no_body(mut self, body: &str) -> Self {
        self.request.no_body(body);
        self
    }

    pub fn body_of_type(mut self, body: Bytes, content_type: Bytes) -> Self {
        self.request.body_of_type(body, content_type);
        self
    }

    pub fn no_body_of_type(mut self, body: &[u8], content_type: Bytes) -> Self {
        self.request.no_body_of_type(body, content_type);
        self
    }

    pub fn as_mut_ref(&mut self) -> &mut Request {
        self.request.as_mut()
    }

    pub fn finish(self) -> Request {
        *self.request
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::field::test::assert_headers;

    const HEAD_STRINGIFIED: &str = "\
        HEAD / HTTP/1.1\r\n\
        Host: example.com\r\n\
        User-Agent: curl/8.1.2\r\n\
        Accept: */*\r\n\r\n";

    const HEAD_HEADERS: [(&str, &[&str]); 3] = [
        ("Host", &["example.com"]),
        ("User-Agent", &["curl/8.1.2"]),
        ("Accept", &["*/*"]),
    ];

    const POST_STRINGIFIED: &str = "\
        POST / HTTP/1.1\r\n\
        Host: example.com\r\n\
        User-Agent: curl/8.1.2\r\n\
        Accept: */*\r\n\
        Content-Length: 12\r\n\
        Content-Type: application/x-www-form-urlencoded\r\n\
        \r\n\
        Hello world!";

    const POST_HEADERS: [(&str, &[&str]); 5] = [
        ("Host", &["example.com"]),
        ("User-Agent", &["curl/8.1.2"]),
        ("Accept", &["*/*"]),
        ("Content-Length", &["12"]),
        ("Content-Type", &["application/x-www-form-urlencoded"]),
    ];

    fn assert_start_line(req: &Request, method: Method, path: &str, version: Version) {
        assert_eq!(req.method, method);
        assert_eq!(std::str::from_utf8(&req.path).unwrap(), path);
        assert_eq!(req.version, version);
    }

    #[test]
    fn head_from_bytes() {
        let mut bytes = Bytes::copy_from_slice(HEAD_STRINGIFIED.as_bytes());
        let req = Request::from_bytes(&mut bytes).unwrap();
        assert_start_line(&req, Method::Head, "/", Version(1, 1));
        assert_headers(&req.headers, &HEAD_HEADERS);
        assert!(req.body.is_empty());
        assert!(req.trailers.is_empty());
    }

    #[test]
    fn head_to_string() {
        let req = Request {
            method: Method::Head,
            path: "/".into(),
            version: Version(1, 1),
            headers: Fields::copy_from_str(HEAD_HEADERS),
            body: Bytes::new(),
            trailers: Fields::new(),
        };
        assert_eq!(String::from_utf8(req.to_buffer()).unwrap(), HEAD_STRINGIFIED);
    }

    #[test]
    fn post_from_bytes() {
        let mut bytes = Bytes::copy_from_slice(POST_STRINGIFIED.as_bytes());
        let req = Request::from_bytes(&mut bytes).unwrap();
        assert_start_line(&req, Method::Post, "/", Version(1, 1));
        assert_headers(&req.headers, &POST_HEADERS);
        assert_eq!(&*req.body, b"Hello world!");
        assert!(req.trailers.is_empty());
    }

    #[test]
    fn post_to_string() {
        let req = Request {
            method: Method::Post,
            path: "/".into(),
            version: Version(1, 1),
            headers: Fields::copy_from_str(POST_HEADERS),
            body: "Hello world!".into(),
            trailers: Fields::new(),
        };
        assert_eq!(String::from_utf8(req.to_buffer()).unwrap(), POST_STRINGIFIED);
    }
}
