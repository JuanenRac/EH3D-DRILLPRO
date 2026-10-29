//! The files that travel with every release: the basic configuration and the examples, from one hole to many thousands.

use eh3d_drillpro::job::{select_holes, JobOptions, Sorting};
use eh3d_drillpro::{config, ncd, tools};

fn root(rel: &str) -> String {
    format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn latin1(path: &str) -> String {
    std::fs::read(path)
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect()
}

/// The examples with the holes and tools each one has (the list in `examples/README.txt` is written by the same script).
const examples: [(&str, usize, usize); 9] = [
    ("01_one_hole.ncd", 1, 1),
    ("02_three_holes.ncd", 3, 1),
    ("03_two_tools.ncd", 6, 2),
    ("04_grid_10x10.ncd", 100, 1),
    ("05_small_board.ncd", 58, 3),
    ("06_four_tools_80_holes.ncd", 80, 4),
    ("07_dense_1000_holes.ncd", 1000, 2),
    ("08_large_10000_holes.ncd", 10_000, 3),
    ("09_huge_20000_holes.ncd", 20_000, 4),
];

#[test]
fn every_example_is_read_with_its_holes_and_tools() {
    for (name, holes, ntools) in examples {
        let f = ncd::parse(&latin1(&root(&format!("examples/{name}")))).unwrap();
        assert_eq!(f.holes.len(), holes, "{name}");
        assert_eq!(f.tools.len(), ntools, "{name}");
        // everything is inside the table of the basic configuration (495 x 325 mm)
        assert!(
            f.holes
                .iter()
                .all(|h| (0..=19_488).contains(&h.x_mil) && (0..=12_795).contains(&h.y_mil)),
            "{name}"
        );
    }
}

#[test]
fn no_example_is_missing_from_the_list() {
    let mut on_disk: Vec<String> = std::fs::read_dir(root("examples"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".ncd"))
        .collect();
    on_disk.sort();
    let listed: Vec<String> = examples.iter().map(|e| e.0.to_string()).collect();
    assert_eq!(on_disk, listed);
}

#[test]
fn the_examples_are_planned_whole_in_the_order_of_the_machine() {
    let f = ncd::parse(&latin1(&root("examples/07_dense_1000_holes.ncd"))).unwrap();
    let sel = select_holes(
        &f,
        Sorting::Special,
        &JobOptions::default(),
        0.0254,
        (1.0, 1.0),
        (2.5, 2.5),
    );
    assert_eq!(sel.holes.len() + sel.skipped, 1000);
}

#[test]
fn the_basic_configuration_is_read_whole() {
    let c = config::parse(&latin1(&root("config/DRILLPRO.DCF"))).expect("the machine file");
    let s = c.settings().expect("typed values");
    assert_eq!(s.clip_mm, (495.0, 325.0));
    assert_eq!(s.unit, config::CoordUnit::Mil);
    assert_eq!(s.interface, 4, "no machine: the simulator");
    assert_eq!(s.baud, 2);
    assert!(!s.auto_change && s.atc.is_empty());
    assert_eq!(c.directory, r".\examples");
    assert_eq!(c.default_extension, "*.NCD");
    let t = tools::parse(&latin1(&root("config/DRILLPRO.DTD")));
    assert_eq!(t.len(), 15);
    assert!(t.iter().any(|t| t.number == 1));
}

#[test]
fn the_basic_configuration_is_rewritten_without_losing_its_explanations() {
    let text = latin1(&root("config/DRILLPRO.DCF"));
    let c = config::parse(&text).unwrap();
    let s = c.settings().unwrap();
    let out = config::rewrite(&text, &s, r".\examples", "*.NCD", false);
    // every explanation survives and the values read the same
    assert_eq!(
        out.matches('*').count(),
        text.matches('*').count(),
        "the comments are kept"
    );
    assert_eq!(config::parse(&out).unwrap().settings().unwrap(), s);
}

#[test]
fn the_configuration_guide_exists_in_the_eight_languages() {
    for lang in ["eng", "spa", "deu", "fra", "ita", "por", "zho", "jpn"] {
        let p = root(&format!("config/GUIDE_{lang}.txt"));
        let text = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("{p}"));
        // each one names the three files and the example to start with
        for needle in [
            "DRILLPRO.DCF",
            "DRILLPRO.DTD",
            "machine.cfg",
            "01_one_hole.ncd",
        ] {
            assert!(text.contains(needle), "{lang}: {needle}");
        }
    }
}
