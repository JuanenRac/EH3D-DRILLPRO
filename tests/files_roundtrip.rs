//! Writing the configuration and the tool table back keeps what the programs do not edit.

use std::path::Path;

use eh3d_drillpro::{config, tools};

fn read(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(name);
    std::fs::read(p)
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect()
}

#[test]
fn an_unchanged_configuration_is_rewritten_without_a_difference_in_its_values() {
    let text = read("DRILLPRO.DCF");
    let cfg = config::parse(&text).unwrap();
    let settings = cfg.settings().unwrap();
    let out = config::rewrite(
        &text,
        &settings,
        &cfg.directory,
        &cfg.default_extension,
        false,
    );
    assert_eq!(config::parse(&out).unwrap().settings().unwrap(), settings);
    // the comments are untouched: the same number of lines and the same comment text
    assert_eq!(out.lines().count(), text.lines().count());
    assert!(out.contains("* Scale X (0.0000 ... 10.000)"));
}

#[test]
fn edited_values_come_back_through_the_file() {
    let text = read("DRILLPRO.DCF");
    let cfg = config::parse(&text).unwrap();
    let mut s = cfg.settings().unwrap();
    s.scale = (1.5, 0.75);
    s.offset_mm = (3.25, 4.0);
    s.speed_xy = (3500, 900, 800);
    s.correction = (0.9991, 1.0002, 0.9);
    s.sorting = config::SortMode::XFirst;
    s.unit = config::CoordUnit::PlotterStep;
    let out = config::rewrite(&text, &s, r".\NEWDIR", "*.DRL", false);
    let back = config::parse(&out).unwrap();
    assert_eq!(back.directory, r".\NEWDIR");
    assert_eq!(back.default_extension, "*.DRL");
    assert_eq!(back.settings().unwrap(), s);
}

#[test]
fn a_tool_table_keeps_its_comment_lines() {
    let text = read("DRILLPRO.DTD");
    let table = tools::parse(&text);
    assert!(!table.is_empty());
    let out = tools::rewrite(&text, &table);
    assert_eq!(tools::parse(&out), table);
    assert_eq!(
        out.lines().take(19).collect::<Vec<_>>(),
        text.lines().take(19).collect::<Vec<_>>()
    );
}
