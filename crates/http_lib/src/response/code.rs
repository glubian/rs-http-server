use std::fmt;

use bytes::Bytes;

use crate::Advance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParsingError {
    InvalidCode,
    MalformedCode,
}

impl ParsingError {
    pub const fn as_str(self) -> &'static str {
        use ParsingError::*;
        match self {
            InvalidCode => "invalid code",
            MalformedCode => "malformed code",
        }
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::error::Error for ParsingError {}

// Automatically generates Code::LOOKUP
macro_rules! make_codes_enum {
    ($name:ident $repr:ty { $( $variant:ident = $value:expr, )* }) => {
        #[allow(unused)]
        #[repr($repr)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum $name {
            $( $variant = $value, )*
        }

        impl $name {
            #[allow(unused)]
            const LOOKUP: [bool; 500] = {
                let values = [$($value,)*];
                let mut lookup = [false; 500];

                let mut i = 0;
                while i < values.len() {
                    let v = values[i];
                    let l = v - 100;
                    lookup[l] = true;
                    i += 1;
                }

                lookup
            };

            #[allow(unused)]
            const SHORTEST: usize = {
                use $name::*;
                let mut min = usize::MAX;
                let all_variants = [$($variant,)*];
                let mut i = 0;
                while i < all_variants.len() {
                    let len = all_variants[i].as_str().len();
                    if len < min {
                        min = len;
                    }
                    i += 1;
                }

                min
            };
        }
    }
}

make_codes_enum!(Code u16 {
    // 1xx Informational
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,

    // 2xx Success
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    IMUsed = 226,

    // 3xx Redirection
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // 4xx Client Error
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    URITooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    MisdirectedRequest = 421,
    UnprocessableContent = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    // 5xx Server Error
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HTTPVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
});

impl Code {
    pub const fn as_str(self) -> &'static str {
        use Code::*;
        match self {
            Continue => "100 Continue",
            SwitchingProtocols => "101 Switching Protocols",
            Processing => "102 Processing",
            EarlyHints => "103 Early Hints",

            Ok => "200 OK",
            Created => "201 Created",
            Accepted => "202 Accepted",
            NonAuthoritativeInformation => "203 Non-Authoritative Information",
            NoContent => "204 No Content",
            ResetContent => "205 Reset Content",
            PartialContent => "206 Partial Content",
            MultiStatus => "207 Multi-Status",
            AlreadyReported => "208 Already Reported",
            IMUsed => "226 IM Used",

            MultipleChoices => "300 Multiple Choices",
            MovedPermanently => "301 Moved Permanently",
            Found => "302 Found",
            SeeOther => "303 See Other",
            NotModified => "304 Not Modified",
            UseProxy => "305 Use Proxy",
            TemporaryRedirect => "307 Temporary Redirect",
            PermanentRedirect => "308 Permanent Redirect",

            BadRequest => "400 Bad Request",
            Unauthorized => "401 Unauthorized",
            PaymentRequired => "402 Payment Required",
            Forbidden => "403 Forbidden",
            NotFound => "404 Not Found",
            MethodNotAllowed => "405 Method Not Allowed",
            NotAcceptable => "406 Not Acceptable",
            ProxyAuthenticationRequired => "407 Proxy Authentication Required",
            RequestTimeout => "408 Request Timeout",
            Conflict => "409 Conflict",
            Gone => "410 Gone",
            LengthRequired => "411 Length Required",
            PreconditionFailed => "412 Precondition Failed",
            PayloadTooLarge => "413 Payload Too Large",
            URITooLong => "414 URI Too Long",
            UnsupportedMediaType => "415 Unsupported Media Type",
            RangeNotSatisfiable => "416 Range Not Satisfiable",
            ExpectationFailed => "417 Expectation Failed",
            ImATeapot => "418 I'm a teapot",
            MisdirectedRequest => "421 Misdirected Request",
            UnprocessableContent => "422 Unprocessable Content",
            Locked => "423 Locked",
            FailedDependency => "424 Failed Dependency",
            TooEarly => "425 Too Early",
            UpgradeRequired => "426 Upgrade Required",
            PreconditionRequired => "428 Precondition Required",
            TooManyRequests => "429 Too Many Requests",
            RequestHeaderFieldsTooLarge => "431 Request Header Fields Too Large",
            UnavailableForLegalReasons => "451 Unavailable For Legal Reasons",

            InternalServerError => "500 Internal Server Error",
            NotImplemented => "501 Not Implemented",
            BadGateway => "502 Bad Gateway",
            ServiceUnavailable => "503 Service Unavailable",
            GatewayTimeout => "504 Gateway Timeout",
            HTTPVersionNotSupported => "505 HTTP Version Not Supported",
            VariantAlsoNegotiates => "506 Variant Also Negotiates",
            InsufficientStorage => "507 Insufficient Storage",
            LoopDetected => "508 Loop Detected",
            NotExtended => "510 Not Extended",
            NetworkAuthenticationRequired => "511 Network Authentication Required",
        }
    }

    pub const fn as_bytes(self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    pub fn from_bytes(bytes: &mut Bytes) -> Result<Self, ParsingError> {
        if bytes.len() < 3 {
            return Err(ParsingError::InvalidCode);
        } else if bytes.len() < 5 {
            return Err(ParsingError::MalformedCode);
        }

        let raw = std::str::from_utf8(&bytes[..3])
            .map_err(|_| ParsingError::InvalidCode)?
            .parse()
            .map_err(|_| ParsingError::InvalidCode)?;

        let Some(code) = Self::from_raw(raw) else {
            return Err(ParsingError::InvalidCode)
        };

        if !bytes.advance_bytes(code.as_bytes()) {
            return Err(ParsingError::MalformedCode);
        }

        Ok(code)
    }

    pub const fn from_raw(code: u16) -> Option<Self> {
        if code < 100 {
            return None;
        }

        let idx = (code - 100) as usize;
        if idx < Self::LOOKUP.len() && Self::LOOKUP[idx] {
            Some(unsafe { std::mem::transmute(code) })
        } else {
            None
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_from_bytes(src: &[u8]) -> Result<Code, ParsingError> {
        let mut bytes = Bytes::copy_from_slice(src);
        Code::from_bytes(&mut bytes)
    }

    #[test]
    fn from_bytes() {
        assert!(test_from_bytes(b"404 Not Found").is_ok_and(|c| c == Code::NotFound));
        assert!(test_from_bytes(b"200 OK").is_ok_and(|c| c == Code::Ok));
        assert!(test_from_bytes(b"200").is_err_and(|e| e == ParsingError::MalformedCode));
        assert!(test_from_bytes(b"20").is_err_and(|e| e == ParsingError::InvalidCode));
        assert!(test_from_bytes(b"000 Nonexistent").is_err_and(|e| e == ParsingError::InvalidCode));
    }
}
