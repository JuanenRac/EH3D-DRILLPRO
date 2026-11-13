//! The warning about a token that would travel in the clear: which addresses are the user's own and which are not.

use eh3d_drillpro::server_link::plain_token_warning;

#[test]
fn this_computer_and_private_networks_give_no_warning() {
    for a in [
        "127.0.0.1:18200",
        "127.9.9.9:1",
        "localhost:18200",
        "LOCALHOST:18200",
        "[::1]:18200",
        "::1",
        "10.0.0.2:18200",
        "172.16.0.1:18200",
        "172.31.255.255:18200",
        "192.168.0.180:18200",
        "169.254.3.4:18200",
        "[fe80::1]:18200",
        "[fd12:3456::1]:18200",
        "cm5.local:18200",
        "cm5.local.:18200",
        "http://192.168.0.180:18200/api",
        "user@192.168.1.1:80",
    ] {
        assert!(!plain_token_warning(a), "{a}");
    }
}

#[test]
fn any_other_address_gives_a_warning() {
    for a in [
        "8.8.8.8:18200",
        "172.15.0.1:18200",
        "172.32.0.1:18200",
        "192.169.0.1:80",
        "11.0.0.1:80",
        "example.com:18200",
        "cm5:18200",
        "[2001:db8::1]:18200",
        "http://example.com:80/",
        "evil.local.example.com:80",
        "user@8.8.8.8:80",
    ] {
        assert!(plain_token_warning(a), "{a}");
    }
}

#[test]
fn an_https_address_or_an_empty_one_gives_no_warning() {
    for a in [
        "",
        "   ",
        "https://example.com",
        "HTTPS://8.8.8.8:443",
        "http://",
        ":80",
    ] {
        assert!(!plain_token_warning(a), "{a:?}");
    }
}

#[test]
fn the_verdict_never_panics_on_odd_text_and_ignores_surrounding_spaces() {
    let mut state: u64 = 0xD15EA5E;
    let alphabet: Vec<char> = "0123456789abcdef:.[]@/?#% \u{e9}\u{4e2d}-_"
        .chars()
        .collect();
    for _ in 0..5000 {
        let mut text = String::new();
        for _ in 0..(state % 24) {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            text.push(alphabet[((state >> 33) as usize) % alphabet.len()]);
        }
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        assert_eq!(
            plain_token_warning(&text),
            plain_token_warning(&format!("  {text}  ")),
            "{text:?}"
        );
    }
}
