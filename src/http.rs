use std::borrow::Cow;
use std::collections::hash_map::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::io::{self, prelude::*};
use std::ops::Deref;

const HTTP_PROTOCOL: &str = "HTTP/1.1";

pub type StatusCode = u16;

lazy_static! {
    static ref HTTP_STATUS_MESSAGES: HashMap<StatusCode, &'static str> = {
        let mut m = HashMap::new();
        m.insert(200, "OK");
        m.insert(204, "NO CONTENT");
        m.insert(400, "BAD REQUEST");
        m.insert(403, "FORBIDDEN");
        m.insert(404, "NOT FOUND");
        m.insert(500, "INTERNAL SERVER ERROR");
        m
    };
}
const HTTP_DEFAULT_STATUS_MESSAGE: &str = "UNKNOWN";

pub fn get_status_msg(status_code: StatusCode) -> &'static str {
    match HTTP_STATUS_MESSAGES.get(&status_code) {
        Some(e) => e,
        None => HTTP_DEFAULT_STATUS_MESSAGE,
    }
}

#[derive(PartialEq)]
pub enum Method {
    Unknown,
    GET,
    POST,
    DELETE,
}

impl From<&str> for Method {
    fn from(name: &str) -> Self {
        match name {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "DELETE" => Method::DELETE,
            _ => Method::Unknown,
        }
    }
}

impl From<Method> for &str {
    fn from(method: Method) -> Self {
        match method {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::DELETE => "DELETE",
            Method::Unknown => "Unknown",
        }
    }
}

#[derive(PartialEq)]
pub struct URL(String);

impl From<&str> for URL {
    fn from(s: &str) -> Self {
        URL(String::from(s))
    }
}

impl From<String> for URL {
    fn from(s: String) -> Self {
        URL(s)
    }
}

impl Deref for URL {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Request {
    pub method: Method,
    pub url: URL,
}

impl Request {
    pub fn new(method: Method, url: URL) -> Request {
        Request { method, url }
    }
}

pub struct Response<P> 
    where P: AsRef<[u8]>,
{
    pub status_code: StatusCode,
    pub payload: P,
}

#[allow(dead_code)]
impl<P> Response<P> 
    where P: AsRef<[u8]>,
{
    pub fn new(status_code: StatusCode, payload: P) -> Response<P> {
        Response {
            status_code,
            payload,
        }
    }
    pub fn ok(payload: P) -> Response<P> {
        Response::new(200, payload)
    }
    pub fn forbidden(payload: P) -> Response<P> {
        Response::new(403, payload)
    }
    pub fn not_found(payload: P) -> Response<P> {
        Response::new(404, payload)
    }
    pub fn server_error(payload: P) -> Response<P> {
        Response::new(500, payload)
    }
}

pub struct RequestParseError;

impl Debug for RequestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error parsing HTTP request")
    }
}

impl Display for RequestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for RequestParseError {}

pub enum RequestStatus {
    Ok(Request),
    IOError(io::Error),
    ParseError(RequestParseError),
}

pub fn read_request(stream: &mut impl Read) -> RequestStatus {
    let mut buffer = [0; 512];
    match stream.read(&mut buffer) {
        Err(ioerror) => return RequestStatus::IOError(ioerror),
        _ => {}
    };

    let mut request = buffer
        .split(|&b| b == '\n' as u8)
        .next()
        .unwrap()
        .split(|&b| b == ' ' as u8);
    let method = if let Some(text) = request.next() {
        text
    } else {
        return RequestStatus::ParseError(RequestParseError);
    };
    let url = if let Some(text) = request.next() {
        text
    } else {
        return RequestStatus::ParseError(RequestParseError);
    };
    let protocol = request
        .next()
        .unwrap()
        .split(|&b| b == '\r' as u8)
        .next()
        .unwrap();

    if protocol != HTTP_PROTOCOL.as_bytes() {
        RequestStatus::ParseError(RequestParseError)
    } else {
        let method = Method::from(String::from_utf8_lossy(method).as_ref());
        let url = match String::from_utf8_lossy(url) {
            Cow::Owned(u) => URL::from(u),
            Cow::Borrowed(u) => URL::from(u),
        };
        RequestStatus::Ok(Request::new(method, url))
    }
}

pub fn write_response<P>(stream: &mut impl Write, response: Response<P>) -> io::Result<()> 
    where P: AsRef<[u8]>,
{
    let status_msg = get_status_msg(response.status_code);
    let status_line = format!("{} {} {}", HTTP_PROTOCOL, response.status_code, status_msg);
    stream.write(status_line.as_bytes())?;
    stream.write(b"\r\n\r\n")?;
    stream.write(response.payload.as_ref())?;
    stream.flush()
}
