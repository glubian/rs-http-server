use std::fmt;

use bytes::Bytes;

use crate::Advance;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Head,
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Connect,
    Trace,
}

impl Method {
    pub const fn as_str(self) -> &'static str {
        use Method::*;
        match self {
            Get => "GET",
            Head => "HEAD",
            Post => "POST",
            Put => "PUT",
            Delete => "DELETE",
            Patch => "PATCH",
            Options => "OPTIONS",
            Connect => "CONNECT",
            Trace => "TRACE",
        }
    }

    pub const fn as_bytes(self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    pub fn from_bytes(bytes: &mut Bytes) -> Option<Self> {
        use Method::*;
        if bytes.advance_bytes(Get.as_bytes()) {
            Some(Get)
        } else if bytes.advance_bytes(Head.as_bytes()) {
            Some(Head)
        } else if bytes.advance_bytes(Post.as_bytes()) {
            Some(Post)
        } else if bytes.advance_bytes(Put.as_bytes()) {
            Some(Put)
        } else if bytes.advance_bytes(Delete.as_bytes()) {
            Some(Delete)
        } else if bytes.advance_bytes(Patch.as_bytes()) {
            Some(Patch)
        } else if bytes.advance_bytes(Options.as_bytes()) {
            Some(Options)
        } else if bytes.advance_bytes(Connect.as_bytes()) {
            Some(Connect)
        } else if bytes.advance_bytes(Trace.as_bytes()) {
            Some(Trace)
        } else {
            None
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Method::{}", self.as_str())
    }
}
