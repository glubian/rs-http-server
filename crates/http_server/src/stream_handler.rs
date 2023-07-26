use std::io::{self, Read as _, Write as _};
use std::net::TcpStream;

use bytes::BytesMut;
use log::{error, warn};

use crate::router::Router;
use http_lib::{response::Code, Request, Response};

const REQ_GROWTH_RATE: usize = 8192;
const REQ_MAX_CAPACITY: usize = REQ_GROWTH_RATE * 2;

// Buffers requests and sends responses.
pub struct StreamHandler {
    req_buffer: BytesMut,
    res_buffer: Vec<u8>,
    router: Router,
}

impl StreamHandler {
    pub fn new(router: Router) -> Self {
        let mut req_buffer = BytesMut::new();
        req_buffer.resize(REQ_GROWTH_RATE, 0);

        Self {
            req_buffer,
            res_buffer: Vec::with_capacity(8192),
            router,
        }
    }

    pub fn dispatch(&mut self, stream: &mut TcpStream) {
        let Self {
            req_buffer,
            res_buffer,
            router,
        } = self;

        if let Err(err) = buffer_request(stream, req_buffer) {
            warn!("An error occurred while buffering request {err}");
            return;
        }

        res_buffer.clear();

        if req_buffer.len() >= REQ_MAX_CAPACITY {
            warn!("Request too large, skipping!");

            Response::new(Code::PayloadTooLarge).write_to_buffer(res_buffer);
            if let Err(err) = stream.write_all(res_buffer) {
                error!("Failed to send the response: {err}");
            }

            return;
        }

        let req = match Request::from_bytes(&mut req_buffer.clone().into()) {
            Ok(req) => req,
            Err(err) => {
                warn!("Failed to parse request: {err}");

                Response::new(Code::InternalServerError).write_to_buffer(res_buffer);
                if let Err(err) = stream.write_all(res_buffer) {
                    error!("Failed to send the response: {err}");
                }

                return;
            }
        };

        router.handle(&req).write_to_buffer(res_buffer);
        if let Err(err) = stream.write_all(res_buffer) {
            error!("Failed to send the response: {err}");
        }
    }
}

fn buffer_request(stream: &mut TcpStream, req_buffer: &mut BytesMut) -> io::Result<()> {
    let mut writable_from = 0;
    req_buffer.resize(req_buffer.capacity(), 0);

    loop {
        let space = req_buffer.len() - writable_from;
        let read = match stream.read(&mut req_buffer[writable_from..]) {
            Ok(read) => read,
            Err(err) => {
                req_buffer.truncate(writable_from);
                return Err(err);
            }
        };
        let read_more = read == space && req_buffer.len() < REQ_MAX_CAPACITY;
        if read_more {
            writable_from = req_buffer.len();
            req_buffer.resize(req_buffer.len() + REQ_GROWTH_RATE, 0);
        } else {
            req_buffer.truncate(writable_from + read);
            return Ok(());
        }
    }
}
