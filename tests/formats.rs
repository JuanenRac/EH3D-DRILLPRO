use eh3d_drillpro::{config, ncd, tools};

fn read(name: &str) -> String {
    let b = std::fs::read(format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    b.iter().map(|&c| c as char).collect()
}

#[test]
fn atc_test_has_fourteen_tools() {
    let f = ncd::parse(&read("14TOOLS.NCD")).unwrap();
    assert_eq!(f.tools.len(), 14);
    assert!(f.holes.iter().all(|h| h.x_mil == 10000 && h.y_mil == 6000));
}

#[test]
fn modal_coordinates_keep_previous_axis() {
    let f = ncd::parse(&read("speedtst.ncd")).unwrap();
    // "X00000Y00300" then "Y0" keeps X=0; then "X13300" keeps Y=0.
    assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (0, 300));
    assert_eq!((f.holes[1].x_mil, f.holes[1].y_mil), (0, 0));
    assert_eq!((f.holes[2].x_mil, f.holes[2].y_mil), (13300, 0));
}

#[test]
fn board_sample_and_reference_holes() {
    let f = ncd::parse(&read("I8d.ncd")).unwrap();
    assert!(f.holes.len() > 50);
    let r = ncd::parse(&read("refholes.ncd")).unwrap();
    assert_eq!(r.holes.len(), 2);
    assert_eq!(r.tools, vec![20]);
}

#[test]
fn mm_conversion() {
    let h = ncd::Hole {
        tool: 1,
        x_mil: 1000,
        y_mil: 0,
    };
    assert!((h.x_mm() - 25.4).abs() < 1e-9);
}

#[test]
fn tool_table() {
    let t = tools::parse(&read("DRILLPRO.DTD"));
    assert_eq!(
        t[0],
        tools::Tool {
            number: 4,
            extra_stroke_mm: 0.5,
            feed_mm_min: 1200,
            spindle_rpm: 60000
        }
    );
    assert_eq!(t.len(), 15);
}

#[test]
fn machine_config() {
    let c = config::parse(&read("DRILLPRO.DCF")).unwrap();
    assert_eq!(c.serial_number, "0000001");
    assert_eq!(c.version, "TEST.000000A");
    assert_eq!(c.directory, ".\\RACK3000");
    assert_eq!(c.default_extension, "*.NCD");
    // last two fields: serial interface index and baud selector
    assert_eq!(c.values[c.values.len() - 2], "0");
    assert_eq!(c.values[c.values.len() - 1], "2");
}

/// Parses every `.NCD` under the folder named by `EH3D_LOCAL_DATA`, when that variable is set.
#[test]
fn all_real_world_ncd_files_parse() {
    let Some(root) = std::env::var_os("EH3D_LOCAL_DATA").map(std::path::PathBuf::from) else {
        return;
    };
    if !root.exists() {
        return;
    }
    let mut stack = vec![root];
    let (mut ok, mut bad) = (0, Vec::new());
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ncd")) {
                let text: String = std::fs::read(&p)
                    .unwrap()
                    .iter()
                    .map(|&c| c as char)
                    .collect();
                match ncd::parse(&text) {
                    Ok(_) => ok += 1,
                    Err(e) => bad.push(format!("{}: {e}", p.display())),
                }
            }
        }
    }
    eprintln!("parsed {ok} files, {} failures", bad.len());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn typed_settings_of_the_main_machine() {
    use eh3d_drillpro::config::{CoordUnit, SortMode};
    let s = config::parse(&read("DRILLPRO.DCF"))
        .unwrap()
        .settings()
        .unwrap();
    assert_eq!(s.serial_number, "0000001");
    assert_eq!(s.clip_mm, (555.0, 405.0));
    assert_eq!(s.offset_mm, (2.5, 2.5));
    assert_eq!(s.correction, (0.9992, 0.9998, 0.8988));
    assert_eq!(s.z_max_depth_mm, 37.30);
    assert_eq!(s.distance_tool_board_mm, 16.0);
    assert_eq!(s.unit, CoordUnit::Mil);
    assert_eq!(s.sorting, SortMode::Special);
    assert_eq!((s.interface, s.baud), (0, 2));
    assert!(s.atc.is_empty());
}

#[test]
fn typed_settings_of_the_second_machine_with_a_tool_changer() {
    let s = config::parse(&read("DRILLPRO_CNCA.DCF"))
        .unwrap()
        .settings()
        .unwrap();
    assert_eq!(s.serial_number, "0000002");
    assert_eq!(s.clip_mm, (495.0, 325.0));
    assert_eq!(s.correction, (0.9993, 0.9990, 0.8988));
    assert_eq!(s.z_max_depth_mm, 37.5);
    assert_eq!(s.atc.len(), 16);
    assert_eq!(
        (s.atc[0].x_mm, s.atc[0].y_mm, s.atc[0].z_mm),
        (540.0, 20.0, 37.4)
    );
    assert_eq!(s.atc[15].index, 15);
}
