use std::ops::Deref;
use std::io;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;

use crate::http;
use crate::threadpool::ThreadPool;

use num_cpus;
use regex;

pub type Response = http::Response<Vec<u8>>;

pub type ErrorHandler = Box<dyn Fn(Box<dyn std::error::Error>) + Send + Sync + 'static>;
pub type DefaultHandler = Box<dyn Fn(&http::Request) -> Option<Response> + Send + Sync + 'static>;
pub type RequestHandler = (
    regex::Regex,
    Box<dyn Fn(&http::Request, &regex::Captures) -> Option<Response> + Send + Sync + 'static>,
);

pub struct Handlers {
    error_handler: Option<ErrorHandler>,
    default_handler: Option<DefaultHandler>,
    request_handlers: Vec<RequestHandler>,
}

pub struct Server {
    listener: TcpListener,
    threadpool: ThreadPool,
    handlers: Handlers,
}

impl Server {
    pub fn new<A: ToSocketAddrs>(address: A) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        
        Ok(Server {
            listener,
            threadpool: ThreadPool::new(num_cpus::get()),
            handlers: Handlers {
                error_handler: None,
                default_handler: None,
                request_handlers: Vec::new(),
            },
        })
    }
    pub fn register_error_handler<H>(self, handler: H) -> Self
    where
        H: Fn(Box<dyn std::error::Error>) + Send + Sync + 'static,
    {
        let mut server = self;
        server.handlers.error_handler = Some(Box::new(handler));
        server
    }

    pub fn register_default_handler<H>(self, handler: H) -> Self
    where
        H: Fn(&http::Request) -> Option<Response> + Send + Sync + 'static,
    {
        let mut server = self;
        server.handlers.default_handler = Some(Box::new(handler));
        server
    }
    pub fn register_handler<H>(self, pattern: &str, handler: H) -> Self
    where
        H: Fn(&http::Request, &regex::Captures) -> Option<Response> + Send + Sync + 'static,
    {
        let mut server = self;
        let pattern = regex::Regex::new(pattern).unwrap();
        server.handlers
            .request_handlers
            .push((pattern, Box::new(handler)));
        server
    }
    pub fn run(self) {
        let handlers = Arc::new(self.handlers);
        for stream in self.listener.incoming() {
            let stream = stream.unwrap();
            let handlers = handlers.clone();
            self.threadpool
                .execute(move |_wid| Self::handle_request(stream, handlers))
                .unwrap();
        }
    }
    fn handle_request(stream: TcpStream, handlers: impl Deref<Target=Handlers>) {
        let mut stream = stream;
        match http::read_request(&mut stream) {
            http::RequestStatus::Ok(request) => {
                // Find first matching request handler
                let request_handler =
                    handlers
                        .request_handlers
                        .iter()
                        .fold(None, |current, (pattern, handler)| match current {
                            Some(_) => current,
                            None => {
                                if let Some(captures) = pattern.captures(&request.url) {
                                    Some((handler, captures))
                                } else {
                                    None
                                }
                            }
                        });
                // Get response from appropriate handler
                let response = if let Some((request_handler, captures)) = request_handler {
                    request_handler(&request, &captures)
                } else if let Some(default_handler) = &handlers.default_handler {
                    default_handler(&request)
                } else {
                    None
                };
                // Send response
                if let Some(response) = response {
                    http::write_response(&mut stream, response).unwrap();
                }
            }
            http::RequestStatus::IOError(io_error) => {
                if let Some(error_handler) = &handlers.error_handler {
                    error_handler(Box::new(io_error));
                }
            }
            http::RequestStatus::ParseError(parse_error) => {
                if let Some(error_handler) = &handlers.error_handler {
                    error_handler(Box::new(parse_error));
                }
            }
        }
    }
}
