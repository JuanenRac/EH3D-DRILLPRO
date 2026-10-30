//! The way to the EH3D server: a small HTTP client (plain HTTP/1.1, one request per connection) and the reading of what the server answers.
//! A job sent to the server is planned and run there, on the machine's node; the program only sends it, follows it and answers its dialogs.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::Value;

use eh3d_fleet::Fleet;

use crate::protocol::connection::ConnectionConfig;

/// The server a program talks to: its address (`host:port`), the token of this program and the machine the jobs are for.
pub struct Client {
    pub address: String,
    pub token: String,
    pub machine: String,
}

impl Client {
    pub fn new(config: &ConnectionConfig) -> Client {
        Client {
            address: config.server_address.clone(),
            token: config.server_token.clone(),
            machine: config.server_machine.clone(),
        }
    }

    /// Every machine of the server at once, in its station, with what it is doing: what a central screen shows.
    pub fn fleet(&self) -> Result<Fleet, String> {
        let (status, body) = self.call("GET", "/api/v1/fleet", None)?;
        if status != 200 {
            return Err(error_text(status, &body));
        }
        Fleet::from_json(&body)
    }

    /// One request; the answer is the status and the JSON body (`null` when the body is not JSON).
    pub fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Value), String> {
        let target = self
            .address
            .to_socket_addrs()
            .map_err(|e| format!("{}: {e}", self.address))?
            .next()
            .ok_or_else(|| format!("{}: no address", self.address))?;
        let mut stream = TcpStream::connect_timeout(&target, Duration::from_secs(5))
            .map_err(|e| format!("{}: {e}", self.address))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .and_then(|_| stream.set_write_timeout(Some(Duration::from_secs(15))))
            .map_err(|e| e.to_string())?;
        let payload = body.map(|b| b.to_string()).unwrap_or_default();
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\nAccept: application/json\r\n",
            self.address, self.token
        );
        if body.is_some() {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                payload.len()
            ));
        } else if method == "POST" || method == "PUT" {
            request.push_str("Content-Length: 0\r\n");
        }
        request.push_str("\r\n");
        request.push_str(&payload);
        stream
            .write_all(request.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).map_err(|e| e.to_string())?;
        parse_response(&raw)
    }
}

/// The status and the JSON body of an HTTP response (a body sent in chunks is put together).
pub fn parse_response(raw: &[u8]) -> Result<(u16, Value), String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("the answer of the server is incomplete")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or("the answer of the server has no status")?;
    let body = &raw[split + 4..];
    let chunked = head.lines().any(|l| {
        l.to_ascii_lowercase().starts_with("transfer-encoding:")
            && l.to_ascii_lowercase().contains("chunked")
    });
    let body = if chunked {
        unchunk(body)?
    } else {
        body.to_vec()
    };
    Ok((status, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

fn unchunk(mut rest: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let eol = rest
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("a chunk of the answer is cut")?;
        let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..eol]).trim(), 16)
            .map_err(|_| "a chunk of the answer has a bad size")?;
        rest = &rest[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < size + 2 {
            return Err("a chunk of the answer is cut".into());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

/// What the server says went wrong, as one line.
pub fn error_text(status: u16, body: &Value) -> String {
    match (body["error"].as_str(), body["detail"].as_str()) {
        (Some(code), Some(detail)) if !detail.is_empty() => format!("{code}: {detail}"),
        (Some(code), _) => code.to_string(),
        _ => format!("the server answered {status}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_is_split_into_status_and_json() {
        let raw = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 14\r\n\r\n{\"id\":\"j1234\"}";
        let (status, body) = parse_response(raw).unwrap();
        assert_eq!((status, body["id"].as_str()), (200, Some("j1234")));
    }

    #[test]
    fn a_chunked_body_is_put_together() {
        // the body {"error":"x"} in two chunks of 6 and 7 bytes
        let raw = b"HTTP/1.1 422 Unprocessable\r\ntransfer-encoding: chunked\r\n\r\n6\r\n{\"erro\r\n7\r\nr\":\"x\"}\r\n0\r\n\r\n";
        let (status, body) = parse_response(raw).unwrap();
        assert_eq!((status, body["error"].as_str()), (422, Some("x")));
    }

    #[test]
    fn a_cut_answer_is_an_error_and_a_body_that_is_not_json_is_null() {
        assert!(parse_response(b"HTTP/1.1 200 OK\r\ncontent-le").is_err());
        let (status, body) = parse_response(b"HTTP/1.1 502 Bad Gateway\r\n\r\nnot json").unwrap();
        assert_eq!((status, body), (502, Value::Null));
    }

    /// A server that answers one request with `body` and goes away.
    fn one_answer(status: &str, body: &str) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let reply = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let n = s.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                // the request carries the path and the token of the program
                assert!(
                    request.starts_with("GET /api/v1/fleet HTTP/1.1"),
                    "{request}"
                );
                assert!(request.contains("Authorization: Bearer secret"));
                let _ = s.write_all(reply.as_bytes());
            }
        });
        address
    }

    #[test]
    fn the_whole_fleet_is_asked_in_one_request_and_a_refusal_is_one_line() {
        let body = r#"{"stations":[{"id":"bench","machines":["m1","m2"]}],"machines":[
            {"id":"m1","name":"M1","station":"bench","connection":"demo","state":"busy","job":{"id":"j","name":"board","kind":"drill","items_done":1,"items_total":4,"percent":25},"ready":0},
            {"id":"m2","name":"M2","station":"bench","connection":"demo","state":"idle","job":null,"ready":2}]}"#;
        let client = Client {
            address: one_answer("200 OK", body),
            token: "secret".into(),
            machine: String::new(),
        };
        let fleet = client.fleet().unwrap();
        assert_eq!(fleet.machines.len(), 2);
        assert_eq!(fleet.pick_free("bench").unwrap().id, "m2");
        assert_eq!(fleet.find("m1").unwrap().job.as_ref().unwrap().percent, 25);
        let client = Client {
            address: one_answer(
                "401 Unauthorized",
                r#"{"error":"unauthorized","detail":"no token"}"#,
            ),
            token: "secret".into(),
            machine: String::new(),
        };
        assert_eq!(client.fleet().unwrap_err(), "unauthorized: no token");
    }

    #[test]
    fn an_error_is_one_line() {
        assert_eq!(
            error_text(
                422,
                &serde_json::json!({"error": "out_of_table", "detail": "the hole is outside"})
            ),
            "out_of_table: the hole is outside"
        );
        assert_eq!(
            error_text(
                401,
                &serde_json::json!({"error": "unauthorized", "detail": ""})
            ),
            "unauthorized"
        );
        assert_eq!(error_text(500, &Value::Null), "the server answered 500");
    }
}
