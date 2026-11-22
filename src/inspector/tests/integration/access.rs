use crate::{policy::Policy, server::Server};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
fn request(address: &str, method: &str, path: &str, extra: &str, body: &str) -> String {
    let mut s = TcpStream::connect(address).unwrap();
    s.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n{extra}\r\n{body}",body.len()).as_bytes()).unwrap();
    let mut response = String::new();
    s.read_to_string(&mut response).unwrap();
    response
}
#[test]
fn loopback_one_use_bootstrap_and_exact_origin_cookie_pair_are_required() {
    let scratch = tempfile::tempdir().unwrap();
    let server = Server::start(scratch.path(), None, Policy::default()).unwrap();
    assert!(server.address.starts_with("127.0.0.1:"));
    assert_eq!(std::fs::metadata(&server.bootstrap).unwrap().permissions().mode() & 0o777, 0o600);
    let html = std::fs::read_to_string(&server.bootstrap).unwrap();
    let secret = html.split(r#"name="secret" value=""#).nth(1).unwrap().split('"').next().unwrap();
    let body = format!("secret={secret}");
    assert!(request(&server.address, "GET", "/handshake", "", "").starts_with("HTTP/1.1 403"));
    assert!(request(
        &server.address,
        "POST",
        "/bootstrap",
        "Origin: http://evil.invalid\r\nContent-Type: application/x-www-form-urlencoded\r\n",
        &body
    )
    .starts_with("HTTP/1.1 403"));
    let good = request(
        &server.address,
        "POST",
        "/bootstrap",
        "Origin: null\r\nContent-Type: application/x-www-form-urlencoded\r\n",
        &body,
    );
    assert!(good.starts_with("HTTP/1.1 303"));
    assert!(good.contains("HttpOnly; SameSite=Strict"));
    let cookie = good
        .lines()
        .find_map(|s| s.strip_prefix("Set-Cookie: "))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    assert!(!server.bootstrap.exists());
    assert!(request(
        &server.address,
        "POST",
        "/bootstrap",
        "Origin: null\r\nContent-Type: application/x-www-form-urlencoded\r\n",
        &body
    )
    .starts_with("HTTP/1.1 403"));
    let handshake =
        request(&server.address, "GET", "/handshake", &format!("Cookie: {cookie}\r\n"), "");
    assert!(handshake.starts_with("HTTP/1.1 200"));
    let data: serde_json::Value =
        serde_json::from_str(handshake.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(data["build"], crate::process::BUILD);
    let payload=serde_json::json!({"build":data["build"],"protocol":1,"launch":data["launch"],"attachment":null,"request":{"op":"catalog"}}).to_string();
    for origin in ["null", "http://evil.invalid"] {
        assert!(request(
            &server.address,
            "POST",
            "/api",
            &format!("Cookie: {cookie}\r\nOrigin: {origin}\r\nContent-Type: application/json\r\n"),
            &payload
        )
        .starts_with("HTTP/1.1 403"));
    }
    let wrong = payload.replace(crate::process::BUILD, "stale-build");
    assert!(request(
        &server.address,
        "POST",
        "/api",
        &format!(
            "Cookie: {cookie}\r\nOrigin: http://{}\r\nContent-Type: application/json\r\n",
            server.address
        ),
        &wrong
    )
    .starts_with("HTTP/1.1 409"));
}
