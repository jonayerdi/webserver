#[macro_use]
extern crate lazy_static;

mod http;
mod server;
mod threadpool;

const STATIC_ROOT: &str = "static";

const PAGE_INDEX: &str = "index.html";
const PAGE_404: &str = "404.html";

fn read_file(filename: &str) -> std::io::Result<Vec<u8>> {
    let filepath: std::path::PathBuf = [STATIC_ROOT, filename].iter().collect();
    std::fs::read(filepath)
}

fn response_not_found() -> server::Response {
    http::Response::not_found(read_file(PAGE_404).unwrap_or(vec![]))
}

fn response_file_contents(filename: &str) -> server::Response {
    match read_file(filename) {
        Ok(data) => http::Response::ok(data),
        Err(_ioerr) => response_not_found(),
    }
}

fn main() {
    let _server = server::Server::new("127.0.0.1:8080")
        .unwrap()
        .register_error_handler(|error| {
            eprintln!("{}", error);
        })
        .register_default_handler(|_request| {
            Some(response_not_found())
        })
        .register_handler(r"^/$", |_request, _captures| {
            Some(response_file_contents(PAGE_INDEX))
        })
        .register_handler(r"^/sleep/(\d+)$", |_request, captures| {
            match captures.get(1).unwrap().as_str().parse::<u64>() {
                Ok(seconds) => {
                    std::thread::sleep(std::time::Duration::from_secs(seconds));
                    Some(http::Response::ok(vec![]))
                },
                Err(_) => Some(http::Response::new(400, vec![])),
            }
        })
        .register_handler(r"^/([\w\.]+)$", |_request, captures| { 
            let path = std::path::PathBuf::from(captures.get(1).unwrap().as_str());
            if let Some(last) = path.components().last() {
                let mut filename = String::from(last.as_os_str().to_str()?);
                if std::path::PathBuf::from(&filename).extension().is_none() {
                    filename.push_str(".html");
                }
                return Some(response_file_contents(&filename));
            }
            Some(response_not_found())
        })
        .run();
}
