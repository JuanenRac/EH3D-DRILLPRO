//! The documents that say what the program is, what it must do and where it stands, and the register of requirements: every requirement has an identifier, a state that agrees with its
//! progress and its evidence, and names a test that exists (or, when it is not written yet, the test that will show it). The register is the one source of the state of each capability, so
//! `STATUS.md` and the README files must agree with it, and the documents are public: they carry no date and no reference to anything private.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn lf(text: &str) -> String {
    text.replace("\r\n", "\n")
}

const DOCUMENTS: [&str; 5] = [
    "docs/SCOPE.md",
    "docs/REQUIREMENTS.md",
    "docs/STATUS.md",
    "docs/NOT_YET.md",
    "AGENTS.md",
];

const READMES: [&str; 8] = [
    "README.md",
    "README_spa.md",
    "README_deu.md",
    "README_fra.md",
    "README_ita.md",
    "README_jpn.md",
    "README_zho.md",
    "README_por.md",
];

#[derive(Debug)]
struct Row {
    id: String,
    state: String,
    progress: String,
    evidence: String,
    test: String,
    limits: String,
}

fn rows() -> Vec<Row> {
    lf(&read("docs/REQUIREMENTS.md"))
        .lines()
        .filter(|l| l.starts_with("| DRPR-"))
        .map(|l| {
            let cells: Vec<&str> = l
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            assert_eq!(cells.len(), 7, "a row has seven cells: {l}");
            Row {
                id: cells[0].to_string(),
                state: cells[2].to_string(),
                progress: cells[3].to_string(),
                evidence: cells[4].to_string(),
                test: cells[5].trim_matches('`').to_string(),
                limits: cells[6].to_string(),
            }
        })
        .collect()
}

/// Whether the file that `stem` names has a test called `name`: `tests/<stem>.rs`, or the path from the root when the stem has a slash (`src/file`).
fn test_exists(stem: &str, name: &str) -> bool {
    let path = if stem.contains('/') {
        root().join(format!("{stem}.rs"))
    } else {
        root().join("tests").join(format!("{stem}.rs"))
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let lines: Vec<&str> = text.lines().collect();
    let marker = format!("#[{}]", "test");
    lines.iter().enumerate().any(|(i, l)| {
        let declared = l.trim_start().starts_with(&format!("fn {name}("))
            || l.trim_start().starts_with(&format!("fn {name} ("));
        declared
            && lines[i.saturating_sub(4)..i]
                .iter()
                .any(|before| before.trim() == marker)
    })
}

#[test]
fn the_five_documents_exist_and_say_something() {
    for d in DOCUMENTS {
        let text = read(d);
        assert!(text.trim().len() > 400, "{d} is nearly empty");
        assert!(text.starts_with('#'), "{d} does not start with a title");
    }
    assert!(
        lf(&read("docs/SCOPE.md")).contains("\nStatus:"),
        "SCOPE.md says its status in a line that begins with Status:"
    );
}

#[test]
fn every_requirement_has_an_identifier_that_is_used_once_and_in_order() {
    let rows = rows();
    assert!(rows.len() >= 35, "{} requirements", rows.len());
    for (i, r) in rows.iter().enumerate() {
        assert_eq!(
            r.id,
            format!("DRPR-{:03}", i + 1),
            "the identifiers run on without a gap"
        );
    }
}

#[test]
fn every_state_agrees_with_its_progress_and_its_evidence() {
    for r in rows() {
        let expected = match r.progress.as_str() {
            "done" => "implemented",
            "partial" => "prototype",
            "pending" | "blocked" => "planned",
            other => panic!("{}: progress {other}", r.id),
        };
        assert_eq!(r.state, expected, "{}", r.id);
        match r.evidence.as_str() {
            "simulator" | "host-test" => assert_ne!(r.state, "planned", "{}", r.id),
            "none" => assert_eq!(r.state, "planned", "{}", r.id),
            other => panic!("{}: evidence {other}", r.id),
        }
        // nothing has run on a machine, so nothing is validated
        assert_ne!(r.state, "validated", "{}", r.id);
        if r.progress == "partial" || r.progress == "blocked" {
            assert!(
                r.limits.len() > 3 && r.limits != "-",
                "{} says no limit",
                r.id
            );
        }
    }
}

#[test]
fn every_requirement_names_a_test_that_exists() {
    for r in rows() {
        if r.state == "planned" {
            // not written yet: the cell names the test that will show it, and that test must not exist yet (when it does, the row is out of date)
            let spec = r
                .test
                .strip_prefix("planned:")
                .unwrap_or_else(|| panic!("{}: a planned row names `planned:file::test`", r.id));
            let (stem, name) = spec
                .split_once("::")
                .unwrap_or_else(|| panic!("{}: {spec}", r.id));
            assert!(
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{}: {name}",
                r.id
            );
            assert!(
                !test_exists(stem, name),
                "{}: the test {spec} exists, so the row must say more than planned",
                r.id
            );
        } else {
            let (stem, name) = r
                .test
                .split_once("::")
                .unwrap_or_else(|| panic!("{}: {}", r.id, r.test));
            assert!(
                test_exists(stem, name),
                "{}: {stem}.rs has no test {name}",
                r.id
            );
        }
    }
}

#[test]
fn the_register_and_the_status_page_say_the_same_numbers() {
    let rows = rows();
    let count = |state: &str| rows.iter().filter(|r| r.state == state).count();
    let sentence = format!(
        "The register has {} requirements: {} implemented, {} prototype, {} planned and 0 validated.",
        rows.len(),
        count("implemented"),
        count("prototype"),
        count("planned")
    );
    let progress = |p: &str| rows.iter().filter(|r| r.progress == p).count();
    let by_progress = format!(
        "By progress: {} done, {} partial, {} pending and {} blocked.",
        progress("done"),
        progress("partial"),
        progress("pending"),
        progress("blocked")
    );
    for doc in ["docs/REQUIREMENTS.md", "docs/STATUS.md"] {
        let text = lf(&read(doc));
        assert!(text.contains(&sentence), "{doc} does not say: {sentence}");
        assert!(
            text.contains(&by_progress),
            "{doc} does not say: {by_progress}"
        );
    }
}

#[test]
fn the_readmes_link_the_scope_and_the_status() {
    for name in READMES {
        let text = read(name);
        assert!(
            text.contains("docs/SCOPE.md"),
            "{name} does not link docs/SCOPE.md"
        );
        assert!(
            text.contains("docs/STATUS.md"),
            "{name} does not link docs/STATUS.md"
        );
        assert!(
            text.contains("docs/REQUIREMENTS.md"),
            "{name} does not link docs/REQUIREMENTS.md"
        );
    }
}

#[test]
fn the_documents_are_public_without_a_date_or_a_private_reference() {
    // the words are built here so that this file does not hold them itself
    let forbidden = [
        ["son", "net"].concat(),
        ["aud", "it"].concat(),
        ["cla", "ude"].concat(),
        ["anthro", "pic"].concat(),
        ["C:", "\\Users"].concat(),
        ["Docu", "ments\\GitHub"].concat(),
        ["Docu", "ments/GitHub"].concat(),
    ];
    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    for d in DOCUMENTS {
        let text = read(d);
        let lower = text.to_lowercase();
        for word in &forbidden {
            assert!(!lower.contains(&word.to_lowercase()), "{d} holds `{word}`");
        }
        // a date as 2026-11-20, 20/11/2026 or a month with a year
        let bytes: Vec<char> = text.chars().collect();
        for w in bytes.windows(10) {
            let s: String = w.iter().collect();
            let iso = s.len() == 10
                && s.chars().enumerate().all(|(i, c)| {
                    if i == 4 || i == 7 {
                        c == '-'
                    } else {
                        c.is_ascii_digit()
                    }
                })
                && s.starts_with("20");
            assert!(!iso, "{d} holds a date: {s}");
        }
        for m in months {
            for year in 2024..=2030 {
                assert!(
                    !text.contains(&format!("{m} {year}")),
                    "{d} holds a date: {m} {year}"
                );
            }
        }
    }
}

#[test]
fn the_not_yet_page_says_why_and_when_for_every_idea() {
    let text = lf(&read("docs/NOT_YET.md"));
    let rows: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Idea") && !l.starts_with("|---"))
        .collect();
    assert!(rows.len() >= 7, "{} ideas", rows.len());
    for l in rows {
        let cells: Vec<&str> = l
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        assert_eq!(cells.len(), 3, "{l}");
        assert!(cells.iter().all(|c| c.len() > 3), "{l}");
    }
}

#[test]
fn the_scope_says_what_the_program_never_decides() {
    let text = lf(&read("docs/SCOPE.md")).to_lowercase();
    for want in [
        "## purpose",
        "## what it does",
        "## what it will not do",
        "## where it sits",
        "## what it never decides",
    ] {
        assert!(text.contains(want), "SCOPE.md has no section {want}");
    }
    assert!(text.contains("emergency stop"));
    assert!(text.contains("limits"));
}
