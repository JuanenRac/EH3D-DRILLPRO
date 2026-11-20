//! The way to the EH3D server: the requests of a job and the reading of what the server answers. The link itself is the client of the suite
//! (`eh3d-serverclient` of EH3D-COMMON, reached through [`crate::server_wire`]): a time limit and retries with growing pauses on each request (a `POST` is
//! never sent twice), the state of the link, and the refusals told by their code in the language of the person. A job sent to the server is planned and run
//! there, on the machine's node; the program only sends it, follows it and answers its dialogs.

use serde_json::Value;

use eh3d_fleet::Fleet;
use eh3d_serverclient as sc;

use crate::protocol::connection::ConnectionConfig;
use crate::server_wire::{self, Link};

pub use sc::transport::check_request_part;
pub use sc::{path_segment, plain_token_warning};

/// The server a program talks to: its address (`host:port`), the token of this program, the machine the jobs are for and the limits of the link (the time
/// of one try, how many more tries after a failure of the network, whether to look for the server on the network, the most MiB read from an answer).
pub struct Client {
    pub address: String,
    pub token: String,
    pub machine: String,
    pub timeout_s: u64,
    pub retries: u32,
    pub mdns: bool,
    pub max_answer_mib: u64,
}

impl Client {
    pub fn new(config: &ConnectionConfig) -> Client {
        Client {
            address: config.server_address.clone(),
            token: config.server_token.clone(),
            machine: config.server_machine.clone(),
            timeout_s: config.server_timeout_s,
            retries: config.server_retries,
            mdns: config.server_mdns,
            max_answer_mib: config.server_max_answer_mib,
        }
    }

    /// The values of the link, for the client of the suite.
    pub fn link(&self) -> Link {
        Link {
            address: self.address.clone(),
            token: self.token.clone(),
            machine: self.machine.clone(),
            timeout_s: self.timeout_s,
            retries: self.retries,
            mdns: self.mdns,
            max_answer_mib: self.max_answer_mib,
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

    /// One request; the answer is the status and the JSON body (`null` when the body is not JSON). A refusal of the server is an answer, with its status; a
    /// request that never got one is the error, told in the language of the person. The request goes in plain HTTP, so the token of the program travels in the
    /// clear: use it on a network that can be trusted or through a tunnel (SSH, VPN). What is written into the lines of the request (the path, the address and
    /// the token) must not hold a space, a control character or a line break, which would change the request: such a request is refused before it is sent.
    pub fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Value), String> {
        check_request_part("the path", path)?;
        let mut client = sc::Client::connect(self.link().settings()?);
        self.call_with(&mut client, method, path, body)
    }

    /// The same request over a client already made (a server in memory for the tests, a tunnel).
    pub fn call_with(
        &self,
        client: &mut sc::Client,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Value), String> {
        check_request_part("the path", path)?;
        let text = body.map(Value::to_string);
        let (status, raw) = server_wire::exchange(
            client,
            method,
            path,
            text.as_deref(),
            server_wire::language(),
        )?;
        Ok((status, serde_json::from_str(&raw).unwrap_or(Value::Null)))
    }

    /// What the server answers at `/health` and the metrics of the requests (what `--server-status` prints).
    pub fn status(&self) -> Result<String, String> {
        let mut client = sc::Client::connect(self.link().settings()?);
        server_wire::status_text(&mut client, server_wire::language())
    }
}

/// Version of the job request that carries the chaining of the tool changer (`rack`); a request without it is the first version and has no `request_version`.
pub const REQUEST_VERSION_CHAINED: u64 = 2;

/// Puts the chaining of the tool changer into a job request: where the rack of this job starts, whether the chuck comes holding the last tool of the job before
/// and whether this job leaves its last tool in the chuck for the next. When the person said where each tool is, the places go too: `rack.slots` is a list
/// of `{"tool": n, "place": p}` and `rack.carry_slot` the place of the tool the chuck comes holding. A job with the defaults (no chaining, no places) is left as
/// it was, so a server that does not know the field gets exactly the request it always got.
pub fn with_chain(mut request: Value, options: &crate::job::JobOptions) -> Value {
    if options.rack_first == 0
        && !options.rack_carry_in
        && !options.rack_carry_out
        && options.rack_slots.is_empty()
        && options.rack_carry_slot.is_none()
    {
        return request;
    }
    request["request_version"] = Value::from(REQUEST_VERSION_CHAINED);
    request["rack"] = serde_json::json!({
        "first": options.rack_first,
        "carry_in": options.rack_carry_in,
        "carry_out": options.rack_carry_out,
    });
    if !options.rack_slots.is_empty() {
        request["rack"]["slots"] = options
            .rack_slots
            .iter()
            .map(|(tool, place)| serde_json::json!({"tool": tool, "place": place}))
            .collect();
    }
    if let Some(place) = options.rack_carry_slot {
        request["rack"]["carry_slot"] = Value::from(place);
    }
    request
}

/// The same request without the chaining (what an older server takes), or `None` when it had none.
pub fn without_chain(request: &Value) -> Option<Value> {
    let mut plain = request.clone();
    let map = plain.as_object_mut()?;
    map.remove("rack")?;
    map.remove("request_version");
    Some(plain)
}

/// The server turned a job down for a field it does not know: an old server answers 400 or 422 to the chaining of the request.
pub fn chain_refused(status: u16) -> bool {
    status == 400 || status == 422
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
    fn the_chaining_goes_into_the_request_only_when_there_is_one() {
        use crate::job::JobOptions;
        let base = serde_json::json!({"kind": "drill", "name": "a"});
        // no chaining: the request is the one of always
        let plain = with_chain(base.clone(), &JobOptions::default());
        assert_eq!(plain, base);
        assert!(without_chain(&plain).is_none());
        // with chaining: a versioned request with the three values
        let chained = with_chain(
            base.clone(),
            &JobOptions {
                rack_first: 3,
                rack_carry_in: true,
                rack_carry_out: false,
                ..JobOptions::default()
            },
        );
        assert_eq!(chained["request_version"], 2);
        assert_eq!(chained["rack"]["first"], 3);
        assert_eq!(chained["rack"]["carry_in"], true);
        assert_eq!(chained["rack"]["carry_out"], false);
        // taken off again, it is the request of an older server
        assert_eq!(without_chain(&chained), Some(base.clone()));
        // the places the person gave to the tools go in the rack too, and only when there are some
        assert!(chained["rack"].get("slots").is_none());
        let placed = with_chain(
            base.clone(),
            &JobOptions {
                rack_slots: vec![(5, 9), (2, 1)],
                rack_carry_slot: Some(4),
                ..JobOptions::default()
            },
        );
        assert_eq!(placed["request_version"], 2);
        assert_eq!(placed["rack"]["first"], 0);
        assert_eq!(placed["rack"]["slots"][0]["tool"], 5);
        assert_eq!(placed["rack"]["slots"][0]["place"], 9);
        assert_eq!(placed["rack"]["slots"][1]["tool"], 2);
        assert_eq!(placed["rack"]["slots"][1]["place"], 1);
        assert_eq!(placed["rack"]["carry_slot"], 4);
        assert_eq!(without_chain(&placed), Some(base));
        assert!(chain_refused(400) && chain_refused(422));
        assert!(!chain_refused(200) && !chain_refused(401) && !chain_refused(500));
    }

    fn client_at(address: String, token: &str) -> Client {
        Client {
            address,
            token: token.into(),
            machine: String::new(),
            timeout_s: 15,
            retries: 0,
            mdns: false,
            max_answer_mib: 8,
        }
    }

    #[test]
    fn names_become_one_safe_segment_and_a_request_cannot_be_split() {
        assert_eq!(path_segment("bench-1_a.b~c"), "bench-1_a.b~c");
        assert_eq!(
            path_segment("a b/c?d\r\nX: y"),
            "a%20b%2Fc%3Fd%0D%0AX%3A%20y"
        );
        assert_eq!(path_segment("ñ"), "%C3%B1");
        assert!(check_request_part("the path", "/api/v1/fleet").is_ok());
        for bad in ["/a b", "/a\r\nHost: x", "/a\n", "/ñ", "/a\0"] {
            assert!(check_request_part("the path", bad).is_err(), "{bad:?}");
        }
        let client = client_at("127.0.0.1:1".into(), "t\r\nX: y");
        let error = client.call("GET", "/api/v1/fleet", None).unwrap_err();
        assert!(error.contains("server_token"), "{error}");
        let client = client_at("127.0.0.1:1".into(), "t");
        let error = client
            .call("GET", "/api/v1/machines/a b/jobs", None)
            .unwrap_err();
        assert!(error.contains("the path"), "{error}");
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
    fn an_answer_past_the_limit_is_refused_and_a_big_one_below_it_is_read() {
        let big = format!("\"{}\"", "a".repeat(9 * 1_048_576));
        let why = client_at(one_answer("200 OK", &big), "secret")
            .call("GET", "/api/v1/fleet", None)
            .unwrap_err();
        assert!(why.contains("too large"), "{why}");
        let mut wide = client_at(one_answer("200 OK", &big), "secret");
        wide.max_answer_mib = 12;
        assert!(wide.call("GET", "/api/v1/fleet", None).is_ok());
        let fine = format!("\"{}\"", "a".repeat(1_048_576));
        let (status, body) = client_at(one_answer("200 OK", &fine), "secret")
            .call("GET", "/api/v1/fleet", None)
            .unwrap();
        assert_eq!(
            (status, body.as_str().map(str::len)),
            (200, Some(1_048_576))
        );
    }

    #[test]
    fn the_whole_fleet_is_asked_in_one_request_and_a_refusal_is_one_line() {
        let body = r#"{"stations":[{"id":"bench","machines":["m1","m2"]}],"machines":[
            {"id":"m1","name":"M1","station":"bench","connection":"demo","state":"busy","job":{"id":"j","name":"board","kind":"drill","items_done":1,"items_total":4,"percent":25},"ready":0},
            {"id":"m2","name":"M2","station":"bench","connection":"demo","state":"idle","job":null,"ready":2}]}"#;
        let client = client_at(one_answer("200 OK", body), "secret");
        let fleet = client.fleet().unwrap();
        assert_eq!(fleet.machines.len(), 2);
        assert_eq!(fleet.pick_free("bench").unwrap().id, "m2");
        assert_eq!(fleet.find("m1").unwrap().job.as_ref().unwrap().percent, 25);
        let client = client_at(
            one_answer(
                "401 Unauthorized",
                r#"{"error":"unauthorized","detail":"no token"}"#,
            ),
            "secret",
        );
        assert_eq!(client.fleet().unwrap_err(), "unauthorized: no token");
    }

    #[test]
    fn a_refusal_keeps_its_status_and_its_code_over_the_client_of_the_suite() {
        use sc::sim::{Fault, SimServer};
        let sim = SimServer::new();
        let client = client_at("127.0.0.1:18200".into(), "program-token");
        let mut link = sc::Client::new(client.link().settings().unwrap(), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()));
        // a route the server in memory does not have: the 404 and its code come back as an answer
        let (status, body) = client
            .call_with(&mut link, "GET", "/api/v1/nothing", None)
            .unwrap();
        assert_eq!((status, body["error"].as_str()), (404, Some("not_found")));
        // a server that is busy is asked again for a GET
        sim.script(vec![Fault::Status(503, "busy")]);
        let (status, _) = client
            .call_with(&mut link, "GET", "/api/v1/echo", None)
            .unwrap();
        assert_eq!(status, 200);
        assert_eq!(sim.pauses().len(), 1);
        // a POST is sent once: a time-out is the error of that request
        sim.script(vec![Fault::Timeout]);
        let before = sim.pauses().len();
        let why = client
            .call_with(
                &mut link,
                "POST",
                "/api/v1/echo",
                Some(&serde_json::json!({"a": 1})),
            )
            .unwrap_err();
        assert!(!why.is_empty());
        assert_eq!(sim.pauses().len(), before);
    }

    #[test]
    fn the_limits_of_the_link_come_from_the_machine_file() {
        let config = ConnectionConfig {
            server_address: "10.0.0.5:18200".into(),
            server_timeout_s: 40,
            server_retries: 4,
            server_mdns: true,
            ..ConnectionConfig::default()
        };
        let link = Client::new(&config).link();
        let settings = link.settings().unwrap();
        assert_eq!(
            (
                settings.server_timeout_s,
                settings.server_retries,
                settings.server_mdns
            ),
            (40, 4, true)
        );
        assert_eq!(settings.server_address, "10.0.0.5:18200");
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
