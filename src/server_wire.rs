//! One request to the EH3D server over the client of the suite (`eh3d-serverclient` of EH3D-COMMON), as this program asks it: the answer is a status and the
//! text of the body, and a refusal of the server (a 4xx or a 5xx) is an answer too, so the callers read the status and the code as they always did. What the
//! client adds: a time limit and retries with growing pauses on each request (a `POST` is never sent twice), the state of the link, the metrics of the
//! requests and the refusals told by their code in the language of the person. Nothing here depends on the rest of the program.

use eh3d_serverclient as sc;
use sc::json::quote;
use sc::{Client, ClientError, Lang, Settings};

/// The values of the link as the machine file holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub address: String,
    pub token: String,
    pub machine: String,
    pub timeout_s: u64,
    pub retries: u32,
    pub mdns: bool,
    pub max_answer_mib: u64,
}

impl Link {
    /// The settings of the client for these values; a value out of its range or an address with a space is an error that names the key.
    pub fn settings(&self) -> Result<Settings, String> {
        let mut s = Settings::default();
        let mut set =
            |key: &str, value: &str| s.set(key, value).map(|_| ()).map_err(|e| e.to_string());
        set("server_address", &self.address)?;
        set("server_token", &self.token)?;
        set("server_machine", &self.machine)?;
        set("server_timeout_s", &self.timeout_s.to_string())?;
        set("server_retries", &self.retries.to_string())?;
        set("server_mdns", if self.mdns { "on" } else { "off" })?;
        set("server_max_answer_mib", &self.max_answer_mib.to_string())?;
        Ok(s)
    }
}

/// The language of the person for the refusals of the server, from the environment (`LC_ALL`, `LC_MESSAGES`, `LANG`); English when it is none of the eight
/// the catalog of the server is written in.
pub fn language() -> Lang {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find_map(|v| Lang::from_tag(&v))
        .unwrap_or(Lang::En)
}

/// What a failure of the network or a refusal says to the person: the text of its code and, when the text does not hold it, the detail the server gave.
pub fn explain(e: &ClientError, lang: Lang) -> String {
    let text = e.text(lang);
    match e {
        ClientError::Server { detail, .. } if !detail.is_empty() && !text.contains(detail) => {
            format!("{text}: {detail}")
        }
        _ => text,
    }
}

/// A refusal of the server as the status and the body it had (`{"error": code, "detail": detail}`), or `None` when the request never got an answer.
pub fn refusal(e: &ClientError) -> Option<(u16, String)> {
    let body = |code: &Option<String>, detail: &str| match code {
        Some(c) => format!("{{\"error\":{},\"detail\":{}}}", quote(c), quote(detail)),
        None => format!("{{\"detail\":{}}}", quote(detail)),
    };
    match e {
        ClientError::Server {
            status,
            code,
            detail,
        } => Some((*status, body(code, detail))),
        ClientError::Unauthorized { code } => Some((401, body(code, ""))),
        ClientError::Forbidden { code } => Some((403, body(code, ""))),
        _ => None,
    }
}

/// One request: the status and the text of the body. A refusal of the server is an answer (`Ok`); a request that never got one is the error, told in `lang`.
pub fn exchange(
    client: &mut Client,
    method: &str,
    path: &str,
    body: Option<&str>,
    lang: Lang,
) -> Result<(u16, String), String> {
    match client.request(method, path, body) {
        Ok(reply) => Ok((reply.status, reply.body)),
        Err(e) => refusal(&e).ok_or_else(|| explain(&e, lang)),
    }
}

/// What `--server-status` prints: what the server answers at `/health` and the metrics of the link.
pub fn status_text(client: &mut Client, lang: Lang) -> Result<String, String> {
    let health = client.health().map_err(|e| explain(&e, lang))?;
    Ok(format!(
        "{} {} ({}, api {})\n{}",
        health.name,
        health.version,
        health.status,
        health.api_revision,
        client.metrics().to_text()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc::sim::{Fault, SimServer};

    fn link() -> Link {
        Link {
            address: "127.0.0.1:18200".into(),
            token: "program-token".into(),
            machine: "bench".into(),
            timeout_s: 15,
            retries: 2,
            mdns: false,
            max_answer_mib: 8,
        }
    }

    fn client(sim: &SimServer, l: &Link) -> Client {
        Client::new(l.settings().unwrap(), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()))
    }

    #[test]
    fn the_values_of_the_file_become_the_settings_of_the_link() {
        let s = link().settings().unwrap();
        assert_eq!(s.server_address, "127.0.0.1:18200");
        assert_eq!((s.server_timeout_s, s.server_retries), (15, 2));
        assert!(!s.server_mdns);
        assert_eq!(s.server_max_answer_mib, 8);
        let mut bad = link();
        bad.timeout_s = 0;
        assert!(bad.settings().unwrap_err().contains("server_timeout_s"));
        let mut bad = link();
        bad.retries = 9;
        assert!(bad.settings().unwrap_err().contains("server_retries"));
        let mut bad = link();
        bad.address = "a b".into();
        assert!(bad.settings().unwrap_err().contains("server_address"));
    }

    #[test]
    fn a_refusal_is_an_answer_with_its_status_and_code() {
        let sim = SimServer::new();
        let mut c = client(&sim, &link());
        // the server in memory has no such route: a 404 with a code
        let (status, body) = exchange(&mut c, "GET", "/api/v1/nothing", None, Lang::En).unwrap();
        assert_eq!(status, 404);
        assert!(body.contains("\"error\":\"not_found\""), "{body}");
        // a token the server does not know is a 401
        let mut other = link();
        other.token = "wrong".into();
        let mut c = client(&sim, &other);
        let (status, body) = exchange(&mut c, "GET", "/api/v1/echo", None, Lang::En).unwrap();
        assert_eq!(status, 401);
        assert!(body.contains("error"), "{body}");
    }

    #[test]
    fn a_request_that_never_got_an_answer_is_an_error_in_words_after_its_retries() {
        let sim = SimServer::new();
        sim.script(vec![Fault::NoConnection; 3]);
        let mut c = client(&sim, &link());
        let e = exchange(&mut c, "GET", "/api/v1/echo", None, Lang::En).unwrap_err();
        assert!(!e.is_empty());
        // two retries, with a pause before each
        assert_eq!(sim.pauses().len(), 2);
        assert_eq!(sim.request_count(), 0);
    }

    #[test]
    fn a_post_that_timed_out_is_not_sent_twice_but_a_get_is_tried_again() {
        let sim = SimServer::new();
        sim.script(vec![Fault::Timeout, Fault::Timeout]);
        let mut c = client(&sim, &link());
        assert!(exchange(&mut c, "POST", "/api/v1/echo", Some("{}"), Lang::En).is_err());
        // the second time-out is still waiting: the POST used one only
        let (status, _) =
            exchange(&mut c, "GET", "/api/v1/echo", None, Lang::En).unwrap_or((0, String::new()));
        assert_eq!(status, 200);
        assert_eq!(
            sim.seen().iter().filter(|s| s.method == "POST").count(),
            0,
            "a timed-out POST never reached the server in the double, and it was not repeated"
        );
        assert_eq!(sim.pauses().len(), 1);
    }

    #[test]
    fn a_busy_server_is_tried_again_for_a_get_and_the_answer_is_kept() {
        let sim = SimServer::new();
        sim.script(vec![Fault::Status(503, "busy")]);
        let mut c = client(&sim, &link());
        let (status, body) = exchange(&mut c, "GET", "/api/v1/echo", None, Lang::En).unwrap();
        assert_eq!(status, 200, "{body}");
        assert_eq!(sim.pauses().len(), 1);
    }

    #[test]
    fn the_refusals_are_told_in_the_language_asked_for() {
        let e = ClientError::Server {
            status: 422,
            code: Some("bad_credentials".into()),
            detail: String::new(),
        };
        let en = explain(&e, Lang::En);
        let es = explain(&e, Lang::Es);
        assert_ne!(en, es);
        let detail = ClientError::Server {
            status: 422,
            code: Some("bad_gcode".into()),
            detail: "line 3: no".into(),
        };
        assert!(explain(&detail, Lang::En).contains("line 3: no"));
        assert_eq!(
            refusal(&ClientError::Unauthorized { code: None })
                .unwrap()
                .0,
            401
        );
        assert_eq!(
            refusal(&ClientError::Forbidden {
                code: Some("forbidden".into())
            })
            .unwrap()
            .0,
            403
        );
        assert!(refusal(&ClientError::Config("x".into())).is_none());
    }

    #[test]
    fn the_status_names_the_server_and_counts_the_requests() {
        let sim = SimServer::new();
        let mut c = client(&sim, &link());
        let text = status_text(&mut c, Lang::En).unwrap();
        assert!(text.starts_with("eh3d-server "), "{text}");
        assert!(text.to_lowercase().contains("request"), "{text}");
    }
}
