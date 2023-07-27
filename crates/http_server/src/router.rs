use std::fs;

use bytes::Bytes;
use handlebars::Handlebars;
use log::{error, info, warn};
use serde::Serialize;

use crate::config::Config;
use http_lib::{response::Code, Method, Request, Response};

pub const DEFAULT_PORT: u16 = 80;

// Responds to requests with appropriate resources.
pub struct Router {
    handlebars: Handlebars<'static>,
    host_ip: Vec<u8>,
    host_ip_without_port: usize,
    host_ns: Vec<u8>,
    host_ns_without_port: usize,
}

#[derive(Serialize)]
struct DirTemplateData<'a> {
    path: &'a str,
    contents: Vec<String>,
}

impl Router {
    pub fn new(handlebars: Handlebars<'static>, config: &Config) -> Self {
        let Config {
            address,
            port,
            host,
            ..
        } = config;

        let host_ip = format!("{address}:{port}");
        let host_ns = if host.is_empty() {
            String::new()
        } else {
            format!("{host}:{port}")
        };

        let mut host_ip_without_port = 0;
        let mut host_ns_without_port = 0;
        if *port == DEFAULT_PORT {
            host_ip_without_port = host_ip.len().saturating_sub(3);
            host_ns_without_port = host_ns.len().saturating_sub(3);
        }

        Self {
            handlebars,
            host_ip: host_ip.into(),
            host_ip_without_port,
            host_ns: host_ns.into(),
            host_ns_without_port,
        }
    }

    fn validate_host(&self, host: &[u8]) -> bool {
        (!self.host_ns_without_port != 0 && host == &self.host_ns[..self.host_ns_without_port])
            || (!self.host_ip_without_port != 0
                && host == &self.host_ip[..self.host_ip_without_port])
            || (!self.host_ns.is_empty() && host == self.host_ns)
            || host == self.host_ip
    }

    fn route(&self, req: &Request) -> Response {
        if !req
            .headers
            .get_single(b"Host")
            .is_some_and(|h| self.validate_host(h))
        {
            return Response::new(Code::MisdirectedRequest);
        }

        let mut res = self.get_resource_for_path(req);
        if req.method == Method::Head {
            res.body = Bytes::new();
        }

        res
    }

    fn get_resource_for_path(&self, req: &Request) -> Response {
        if !matches!(req.method, Method::Get | Method::Head) {
            return Response::new(Code::MethodNotAllowed);
        }

        if !req.path.first().is_some_and(|&b| b == b'/') {
            return Response::builder(Code::NotFound)
                .body("Not found".to_string())
                .finish();
        }

        if slice_contains(&req.path, b"..") {
            return Response::new(Code::BadRequest);
        }

        let Ok(path) = std::str::from_utf8(&req.path) else {
            return Response::new(Code::BadRequest);
        };

        if req.path.last().is_some_and(|&b| b == b'/') {
            let contents = match fs::read_dir(format!(".{path}")) {
                Ok(read_dir) => read_file_names(read_dir),
                Err(err) => {
                    warn!("Failed to read dir: {err}");
                    return Response::builder(Code::NotFound)
                        .body("Not found".to_string())
                        .finish();
                }
            };

            let data = DirTemplateData { path, contents };
            match self.handlebars.render("dir", &data) {
                Ok(body) => Response::builder(Code::Ok)
                    .body_of_type(body.into(), "text/html".into())
                    .finish(),
                Err(err) => {
                    error!("Failed to render template: {err}");
                    Response::new(Code::InternalServerError)
                }
            }
        } else {
            match fs::read(format!(".{path}")) {
                Ok(body) => {
                    let mime_type = mime_guess::from_path(path).first_or_octet_stream();
                    Response::builder(Code::Ok)
                        .body_of_type(body.into(), mime_type.to_string().into())
                        .finish()
                }
                Err(_) => Response::builder(Code::NotFound)
                    .body("Not found".to_string())
                    .finish(),
            }
        }
    }

    pub fn handle(&self, req: &Request) -> Response {
        let method = req.method;
        let path = req.path.clone();
        let path = std::str::from_utf8(&path).unwrap();
        let res = self.route(req);
        let code = res.code;
        info!("{method} {path} {code}");
        res
    }
}

fn read_file_names(read_dir: fs::ReadDir) -> Vec<String> {
    let mut file_names = Vec::with_capacity(read_dir.size_hint().0);
    for file in read_dir {
        let Ok(file) = file else {
            continue;
        };

        let mut file_name = file.file_name().to_string_lossy().to_string();
        if file_name.is_empty() {
            // this should never happen, but just in case it does, prevent a crash
            continue;
        }
        if file.metadata().map_or(false, |m| m.is_dir()) {
            file_name.push('/');
        }
        file_names.push(file_name);
    }

    file_names.sort_by(|a, b| {
        use std::cmp::Ordering;
        let a_is_dir = a.as_bytes()[a.len() - 1] == b'/';
        let b_is_dir = b.as_bytes()[b.len() - 1] == b'/';
        if a_is_dir == b_is_dir {
            a.cmp(b)
        } else if a_is_dir {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });

    file_names
}

// Taken from https://stackoverflow.com/a/47044053/11967372
fn slice_contains<T: PartialEq>(mut haystack: &[T], needle: &[T]) -> bool {
    if needle.is_empty() {
        return true;
    }

    while !haystack.is_empty() {
        if haystack.starts_with(needle) {
            return true;
        }

        haystack = &haystack[1..];
    }

    false
}
