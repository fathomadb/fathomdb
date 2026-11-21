use std::time::{Duration, Instant};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};
struct DeadlineSocket<'a> {
    stream: &'a mut std::net::TcpStream,
    deadline: Instant,
}
impl<'a> DeadlineSocket<'a> {
    fn new(stream: &'a mut std::net::TcpStream) -> Self {
        Self { stream, deadline: Instant::now() + Duration::from_secs(5) }
    }
    fn remaining(&self) -> std::io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::TimedOut, "socket deadline"))
    }
}
impl Read for DeadlineSocket<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(buffer)
    }
}
impl Write for DeadlineSocket<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(buffer)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}
pub(crate) fn parse_socket(stream: &mut std::net::TcpStream) -> Result<Request, &'static str> {
    parse(DeadlineSocket::new(stream))
}
#[derive(Debug)]
pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
}
pub(crate) fn parse(mut read: impl Read) -> Result<Request, &'static str> {
    let started = Instant::now();
    let mut bytes = vec![];
    loop {
        if bytes.len() >= 16384 || started.elapsed() > Duration::from_secs(5) {
            return Err("invalid_request");
        }
        let mut b = [0];
        read.read_exact(&mut b).map_err(|_| "invalid_request")?;
        bytes.push(b[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let head = std::str::from_utf8(&bytes).map_err(|_| "invalid_request")?;
    let mut lines = head[..head.len() - 4].split("\r\n");
    let first = lines.next().ok_or("invalid_request")?.split(' ').collect::<Vec<_>>();
    if first.len() != 3
        || first[2] != "HTTP/1.1"
        || !matches!(first[0], "GET" | "POST")
        || !first[1].starts_with('/')
        || first[1].contains(['?', '#', '%', '\\', '\r', '\n'])
        || first[1].len() > 256
    {
        return Err("invalid_request");
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        if headers.len() >= 32 || line.starts_with([' ', '\t']) {
            return Err("invalid_request");
        }
        let (key, value) = line.split_once(':').ok_or("invalid_request")?;
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.bytes().any(|b| b < 32 && b != b'\t')
        {
            return Err("invalid_request");
        }
        if headers.insert(key.to_ascii_lowercase(), value.trim().to_string()).is_some() {
            return Err("invalid_request");
        }
    }
    if headers.contains_key("transfer-encoding") || headers.contains_key("expect") {
        return Err("invalid_request");
    }
    let length = match headers.get("content-length") {
        Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
            v.parse::<usize>().map_err(|_| "invalid_request")?
        }
        None if first[0] == "GET" => 0,
        _ => return Err("invalid_request"),
    };
    if length > 2 * 1024 * 1024 || (first[0] == "GET" && length != 0) {
        return Err("invalid_request");
    }
    let mut body = vec![0; length];
    read.read_exact(&mut body).map_err(|_| "invalid_request")?;
    Ok(Request { method: first[0].into(), path: first[1].into(), headers, body })
}
pub(crate) fn respond(
    mut stream: std::net::TcpStream,
    code: u16,
    mime: &str,
    body: &[u8],
    extra: &str,
) -> std::io::Result<()> {
    let mut write = DeadlineSocket::new(&mut stream);
    let status = match code {
        200 => "OK",
        303 => "See Other",
        400 => "Bad Request",
        403 => "Forbidden",
        409 => "Conflict",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Error",
    };
    write!(write,"HTTP/1.1 {code} {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'\r\n{extra}\r\n",body.len())?;
    write.write_all(body)
}
