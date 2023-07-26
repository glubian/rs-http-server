use std::fmt;

use bytes::Bytes;

use crate::{chars::CRLF, field, version, Fields, Version};
use crate::Advance as _;

pub mod code;
pub use code::Code;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParsingError {
    VersionMalformed,
    Code(code::ParsingError),
    MalformedStartLine,
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
            Code(err) => err.as_str(),
            MalformedStartLine => "start line is malformed",
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

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::error::Error for ParsingError {}

pub struct Response {
    pub version: Version,
    pub code: Code,
    pub headers: Fields,
    pub body: Bytes,
    pub trailers: Fields,
}

impl Response {
    pub fn builder(code: Code) -> Builder {
        Builder::new(code)
    }

    pub fn new(code: Code) -> Self {
        let date = httpdate::fmt_http_date(std::time::SystemTime::now());
        let headers = Fields::copy_from_str([("Date", &[&date])]);
        Self {
            version: Version(1, 1),
            code,
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
        let version = Version::from_bytes(bytes).map_err(|_| ParsingError::VersionMalformed)?;

        if !bytes.advance_byte(b' ') {
            return Err(ParsingError::MalformedStartLine);
        }

        let code = Code::from_bytes(bytes).map_err(ParsingError::Code)?;

        if !bytes.advance_bytes(CRLF) {
            return Err(ParsingError::MalformedStartLine);
        }

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
            version,
            code,
            headers,
            body,
            trailers: Fields::new(),
        })
    }

    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        let Self {
            version,
            code,
            headers,
            body,
            trailers,
        } = self;
        version.write_to_buffer(buffer);
        buffer.push(b' ');
        buffer.extend_from_slice(code.as_bytes());
        buffer.extend_from_slice(CRLF);

        headers.write_to_buffer(buffer);
        buffer.extend_from_slice(body);
        trailers.write_to_buffer(buffer);
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(256);
        self.write_to_buffer(&mut buffer);
        buffer
    }
}

pub struct Builder {
    response: Box<Response>,
}

impl Builder {
    pub fn new(code: Code) -> Self {
        Self {
            response: Box::new(Response::new(code)),
        }
    }

    pub fn add_header_value(mut self, name: Bytes, value: Bytes) -> Self {
        self.response.add_header_value(name, value);
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.response.body(body);
        self
    }

    pub fn no_body(mut self, body: &str) -> Self {
        self.response.no_body(body);
        self
    }

    pub fn body_of_type(mut self, body: Bytes, content_type: Bytes) -> Self {
        self.response.body_of_type(body, content_type);
        self
    }

    pub fn no_body_of_type(mut self, body: &[u8], content_type: Bytes) -> Self {
        self.response.no_body_of_type(body, content_type);
        self
    }

    pub fn as_mut_ref(&mut self) -> &mut Response {
        self.response.as_mut()
    }

    pub fn finish(self) -> Response {
        *self.response
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::field::test::assert_headers;

    const STRINGIFIED: &str = "\
        HTTP/1.1 301 Moved Permanently\r\n\
        Location: http://www.example.com/\r\n\
        Date: Fri, 09 Jun 2023 18:36:58 GMT\r\n\
        Expires: Sun, 09 Jul 2023 18:36:58 GMT\r\n\
        Cache-Control: public, max-age=2592000\r\n\
        Server: example\r\n\
        Content-Length: 0\r\n\
        X-XSS-Protection: 0\r\n\
        X-Frame-Options: SAMEORIGIN\r\n\
        \r\n";

    const HEADERS: [(&str, &[&str]); 8] = [
        ("Location", &["http://www.example.com/"]),
        ("Date", &["Fri, 09 Jun 2023 18:36:58 GMT"]),
        ("Expires", &["Sun, 09 Jul 2023 18:36:58 GMT"]),
        ("Cache-Control", &["public", "max-age=2592000"]),
        ("Server", &["example"]),
        ("Content-Length", &["0"]),
        ("X-XSS-Protection", &["0"]),
        ("X-Frame-Options", &["SAMEORIGIN"]),
    ];

    #[test]
    fn from_bytes() {
        let mut bytes = Bytes::from_static(STRINGIFIED.as_bytes());
        let res = Response::from_bytes(&mut bytes).unwrap();
        assert_eq!(res.version, Version(1, 1));
        assert_eq!(res.code, Code::MovedPermanently);
        assert_headers(&res.headers, &HEADERS);
    }

    #[test]
    fn to_buffer() {
        let res = Response {
            version: Version(1, 1),
            code: Code::MovedPermanently,
            headers: Fields::copy_from_str(HEADERS),
            body: Bytes::new(),
            trailers: Fields::new(),
        };
        assert_eq!(String::from_utf8(res.to_buffer()).unwrap(), STRINGIFIED);
    }
}
