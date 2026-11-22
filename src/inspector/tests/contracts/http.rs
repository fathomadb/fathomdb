use crate::http::parse;
#[test]
fn http_framing_is_bounded_and_rejects_ambiguous_requests() {
    let r=parse(&b"POST /api HTTP/1.1\r\nHost: 127.0.0.1:1234\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}"[..]).unwrap();
    assert_eq!(r.method, "POST");
    assert_eq!(r.path, "/api");
    assert_eq!(r.body, b"{}");
    assert_eq!(r.headers["host"], "127.0.0.1:1234");
    for raw in [
        "POST /api HTTP/1.1\r\nContent-Length: 0\r\nContent-Length: 1\r\n\r\nx",
        "POST /api HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
        "POST /api HTTP/1.1\r\nContent-Length: 2097153\r\n\r\n",
        "GET /?secret=x HTTP/1.1\r\nHost: x\r\n\r\n",
        "GET / HTTP/1.0\r\n\r\n",
        "GET / HTTP/1.1\n\n",
    ] {
        assert!(parse(raw.as_bytes()).is_err());
    }
    assert!(
        parse(format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "x".repeat(17000)).as_bytes()).is_err()
    );
}
