use eh3d_drillpro_protocol::connection::{parse, ConnectionConfig, Kind};
use eh3d_drillpro_protocol::Pacing;

#[test]
fn an_empty_file_means_demo() {
    let c = parse("").unwrap();
    assert_eq!(c.kind, Kind::Demo);
    assert!(!c.is_real());
    assert_eq!(c, ConnectionConfig::default());
}

#[test]
fn the_three_ways_and_their_pacing() {
    let text = "# EH3D machine connection\n\nconnection = usb\nrs232_port = COM1\nusb_port = COM8\nnode_address = 192.168.0.50:5800\nnode_token = a#b c\n";
    let mut c = parse(text).unwrap();
    assert_eq!(c.kind, Kind::Usb);
    assert_eq!(
        (c.rs232_port.as_str(), c.usb_port.as_str()),
        ("COM1", "COM8")
    );
    assert_eq!(c.node_address, "192.168.0.50:5800");
    assert_eq!(c.node_token, "a#b c"); // a '#' inside a value is kept
    assert_eq!(c.pacing(), Pacing::StreamKeepDir);
    assert_eq!(c.describe(), "usb COM8");
    c.kind = Kind::Rs232;
    assert_eq!(c.pacing(), Pacing::Bytes);
    assert_eq!(c.describe(), "rs232 COM1");
    c.kind = Kind::Node;
    assert_eq!(c.pacing(), Pacing::StreamKeepDir);
    assert_eq!(c.describe(), "node 192.168.0.50:5800");
    assert!(c.is_real());
}

#[test]
fn mistakes_are_errors_not_silent_defaults() {
    assert!(parse("connection = serial\n")
        .unwrap_err()
        .contains("line 1"));
    assert!(parse("connexion = usb\n")
        .unwrap_err()
        .contains("unknown setting 'connexion'"));
    assert!(parse("just some text\n")
        .unwrap_err()
        .contains("expected key = value"));
    assert!(parse("CONNECTION = NODE\n").is_ok()); // case does not matter
    assert!(parse(";comment\nusb_port=COM9\n").is_ok());
}

#[test]
fn a_named_file_that_is_missing_is_an_error() {
    let missing = std::path::Path::new("definitely-not-here.cfg");
    assert!(eh3d_drillpro_protocol::connection::load(Some(missing)).is_err());
}

#[test]
fn demo_opens_the_simulator_and_a_real_kind_without_its_address_fails() {
    let c = ConnectionConfig::default();
    assert!(c.open().is_ok());
    let node = ConnectionConfig {
        kind: Kind::Node,
        ..ConnectionConfig::default()
    };
    assert!(node.open().is_err()); // no node_address
}

#[test]
fn the_speed_limits_are_read_and_checked() {
    let c = parse("xy_hz_limit = 1800\nz_hz_limit = 400\n").unwrap();
    assert_eq!((c.xy_hz_limit, c.z_hz_limit), (Some(1800.0), Some(400.0)));
    assert_eq!(parse("").unwrap().z_hz_limit, None);
    assert!(parse("z_hz_limit = fast\n").is_err());
    assert!(parse("z_hz_limit = 9000\n").unwrap_err().contains("3840"));
    assert!(parse("xy_hz_limit = 10\n").is_err());
}

#[test]
fn a_capped_geometry_never_exceeds_the_limits() {
    use eh3d_drillpro_protocol::motion::Geometry;
    let g = Geometry::with_correction(1.0, 1.0, 1.0)
        .with_speeds((4000.0, 1000.0, 1000.0), (5000.0, 1000.0, 1000.0))
        .capped(Some(1800.0), Some(400.0));
    assert!(g.max_hz <= 1800.0 && g.start_hz <= g.max_hz && g.ref_hz <= g.max_hz);
    assert!(g.z_max_hz <= 400.0 && g.z_start_hz <= g.z_max_hz && g.z_ref_hz <= g.z_max_hz);
    let free = Geometry::with_correction(1.0, 1.0, 1.0)
        .with_speeds((4000.0, 1000.0, 1000.0), (5000.0, 1000.0, 1000.0));
    assert_eq!(free.capped(None, None), free);
}

// ---- the tuning values of the real machine ----

#[test]
fn every_tuning_key_is_read_and_checked() {
    use eh3d_drillpro_protocol::tuning::Tuning;
    let text = "x_search_hz = 2800\ny_search_hz = 2400\nsearch_ramp_steps = 900\nsearch_margin_percent = 15\n\
                z_search_hz = 1000\nz_start_hz = 500\nz_ramp_steps = 250\njump_peak_hz = 1800\njump_start_fraction = 0.5\n\
                jump_ramp_parts = 4\njump_backoff_steps = 180\njump_approach_steps = 220\nz_backoff_steps = 40\nz_approach_steps = 50\n";
    let t = parse(text).unwrap().tuning;
    assert_eq!(t.x_search_hz, 2800.0);
    assert_eq!(t.y_search_hz, 2400.0);
    assert_eq!(t.search_ramp_steps, 900);
    assert_eq!(t.search_margin_percent, 15.0);
    assert_eq!(
        (t.z_search_hz, t.z_start_hz, t.z_ramp_steps),
        (1000.0, 500.0, 250)
    );
    assert_eq!(
        (t.jump_peak_hz, t.jump_start_fraction, t.jump_ramp_parts),
        (1800.0, 0.5, 4)
    );
    assert_eq!((t.jump_backoff_steps, t.jump_approach_steps), (180, 220));
    assert_eq!((t.z_backoff_steps, t.z_approach_steps), (40, 50));
    // a file with none of them keeps today's values
    assert_eq!(parse("").unwrap().tuning, Tuning::default());
    // out of range or not a number is an error that names the line
    assert!(parse("y_search_hz = 9000\n")
        .unwrap_err()
        .contains("line 1"));
    assert!(parse("jump_start_fraction = 3\n").is_err());
    assert!(parse("z_ramp_steps = lots\n").is_err());
    assert!(parse("jump_ramp_parts = 0\n").is_err());
}

#[test]
fn the_example_file_parses_and_says_what_the_defaults_are() {
    let text = std::fs::read_to_string("../docs/machine.cfg.example").unwrap();
    let c = parse(&text).unwrap();
    assert_eq!(c.tuning, eh3d_drillpro_protocol::tuning::Tuning::default());
    assert!(c.home_signal);
}

#[test]
fn the_modes_and_the_waits_are_keys_of_the_file_too() {
    let t = parse(
        "z_pacing = stream
stream_filler = zero
cancel_piece_steps = 800
home_check_piece_steps = 50
spindle_start_wait_ms = 2000
spindle_stop_wait_ms = 1500
",
    )
    .unwrap()
    .tuning;
    assert!(!t.z_pacing_bytes && !t.stream_keep_direction);
    assert_eq!((t.cancel_piece_steps, t.home_check_piece_steps), (800, 50));
    assert_eq!(
        (t.spindle_start_wait_ms, t.spindle_stop_wait_ms),
        (2000, 1500)
    );
    assert!(parse(
        "z_pacing = fast
"
    )
    .unwrap_err()
    .contains("bytes or stream"));
    assert!(parse(
        "stream_filler = both
"
    )
    .is_err());
    assert!(parse(
        "spindle_start_wait_ms = 99999
"
    )
    .is_err());
    // the filler choice reaches the way a usb connection sends its moves
    use eh3d_drillpro_protocol::Pacing;
    let usb = |text: &str| {
        parse(&format!(
            "connection = usb
{text}"
        ))
        .unwrap()
        .pacing()
    };
    assert_eq!(usb(""), Pacing::StreamKeepDir);
    assert_eq!(
        usb("stream_filler = zero
"),
        Pacing::Stream
    );
}

#[test]
fn the_unstick_keys_are_read_and_checked() {
    let t = parse(
        "unstick_mm = 15
unstick_always = on
search_hint_margin_steps = 500
home_watch_seek = on
home_watch_chunk_bytes = 1200
",
    )
    .unwrap()
    .tuning;
    assert_eq!(
        (t.unstick_mm, t.unstick_always, t.search_hint_margin_steps),
        (15.0, true, 500)
    );
    assert!(t.home_watch_seek);
    assert_eq!(t.home_watch_chunk_bytes, 1200);
    assert!(parse(
        "unstick_mm = 500
"
    )
    .is_err());
    assert!(parse(
        "unstick_always = maybe
"
    )
    .is_err());
    assert_eq!(
        parse(
            "unstick_mm = 0
"
        )
        .unwrap()
        .tuning
        .unstick_mm,
        0.0
    );
}

#[test]
fn the_travel_ramp_is_a_key_of_the_file() {
    assert_eq!(parse("").unwrap().tuning.travel_ramp_steps, 400);
    assert_eq!(
        parse("travel_ramp_steps = 600\n")
            .unwrap()
            .tuning
            .travel_ramp_steps,
        600
    );
    assert!(parse("travel_ramp_steps = 0\n").is_err());
}

#[test]
fn the_spindle_can_be_the_nodes() {
    assert!(!parse("").unwrap().tuning.spindle_node);
    assert!(parse("spindle = node\n").unwrap().tuning.spindle_node);
    assert!(!parse("spindle = central\n").unwrap().tuning.spindle_node);
    assert!(parse("spindle = laser\n").is_err());
}

#[test]
fn a_job_can_go_to_the_eh3d_server() {
    let c = parse("connection = server\nserver_address = 192.168.0.200:18200\nserver_token = eh3d_abc\nserver_machine = bungard\n").unwrap();
    assert_eq!(c.kind, Kind::Server);
    assert!(c.is_real());
    assert_eq!(c.describe(), "server 192.168.0.200:18200 machine bungard");
    assert_eq!(
        (
            c.server_address.as_str(),
            c.server_token.as_str(),
            c.server_machine.as_str()
        ),
        ("192.168.0.200:18200", "eh3d_abc", "bungard")
    );
    // the link of this program is not used: the server runs the job through the machine's node
    assert!(c.open().is_err());
    assert!(parse("connection = cloud\n").is_err());
    // nothing of the server in a file that does not ask for it
    assert_eq!(parse("").unwrap().server_address, "");
}

#[test]
fn the_vacuum_cleaner_can_use_the_auxiliary_output() {
    assert!(!parse("").unwrap().tuning.vacuum_aux);
    assert!(
        parse(
            "vacuum = aux
"
        )
        .unwrap()
        .tuning
        .vacuum_aux
    );
    assert!(
        !parse(
            "vacuum = central
"
        )
        .unwrap()
        .tuning
        .vacuum_aux
    );
    assert!(parse(
        "vacuum = maybe
"
    )
    .is_err());
}
