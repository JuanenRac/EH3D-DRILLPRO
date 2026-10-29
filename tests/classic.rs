//! The classic front end: texts in eight languages, layout limits and the keyboard flow.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::texts::{ctr, CText};
use eh3d_drillpro::classic::{hot_letter, ClassicApp};
use eh3d_drillpro::i18n::{str_width, Lang};
use eh3d_drillpro::screen::Screen;
use eh3d_drillpro::textapp::TextApp;

fn testdata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

fn app(lang: Lang) -> ClassicApp {
    ClassicApp::new(lang, &testdata())
}

fn press(app: &mut ClassicApp, keys: &str) {
    for c in keys.chars() {
        app.handle(Input::Char(c));
    }
}

fn text(app: &ClassicApp) -> String {
    let mut s = Screen::default();
    app.render(&mut s);
    s.to_text()
}

fn plain(label: &str) -> String {
    label.chars().filter(|&c| c != '~').collect()
}

/// Controls that share a screen must have different hot letters in every language.
#[test]
fn hot_letters_are_unique_on_every_screen() {
    let screens: [(&str, &[CText]); 5] = [
        (
            "main",
            &[
                CText::LblScale,
                CText::LblOffset,
                CText::LblClip,
                CText::LblVectors,
                CText::LblBoard,
                CText::LblBase,
                CText::BtnConfig,
                CText::BtnTools,
                CText::BtnStart,
                CText::BtnLoad,
                CText::BtnPanelz,
                CText::BtnOffset,
                CText::BtnQuit,
            ],
        ),
        (
            "parameter",
            &[
                CText::PFile,
                CText::PCoordinates,
                CText::PSorting,
                CText::PSpeed,
                CText::PRamp,
                CText::PStroke,
                CText::PCorrect,
                CText::PSpindle,
                CText::BLoad,
                CText::BOk,
                CText::BAbort,
            ],
        ),
        ("tools", &[CText::BLoad, CText::BOk, CText::BAbort]),
        (
            "files",
            &[CText::FName, CText::FFiles, CText::BOkUpper, CText::BCancel],
        ),
        ("quit", &[CText::BYes, CText::BNo]),
    ];
    for lang in Lang::ALL {
        for (screen, keys) in screens {
            let mut seen = HashSet::new();
            for k in keys {
                let label = ctr(lang, *k);
                let hot = hot_letter(label)
                    .unwrap_or_else(|| panic!("{lang:?} {screen}: no hot letter in {label:?}"));
                assert!(
                    seen.insert(hot),
                    "{lang:?} {screen}: the hot letter {hot:?} of {label:?} is used twice"
                );
            }
        }
    }
}

/// The labels must fit the room the original layout gives them.
#[test]
fn labels_fit_their_slots() {
    let limits: [(CText, usize); 20] = [
        (CText::LblScale, 9),
        (CText::LblOffset, 10),
        (CText::LblClip, 9),
        (CText::LblVectors, 13),
        (CText::LblBoard, 10),
        (CText::LblBase, 10),
        (CText::LblFrm, 4),
        (CText::LblTo, 4),
        (CText::LblCount, 12),
        (CText::LblTotal, 6),
        (CText::LblDone, 6),
        (CText::BtnConfig, 12),
        (CText::BtnTools, 12),
        (CText::BtnStart, 12),
        (CText::BtnLoad, 12),
        (CText::BtnPanelz, 12),
        (CText::BtnOffset, 12),
        (CText::BtnQuit, 12),
        (CText::Title, 19),
        (CText::MachineNr, 14),
    ];
    for lang in Lang::ALL {
        for (k, max) in limits {
            let w = str_width(&plain(ctr(lang, k)));
            assert!(w <= max, "{lang:?} {k:?}: {w} cells > {max}");
        }
    }
}

#[test]
fn every_language_draws_every_screen() {
    for lang in Lang::ALL {
        let mut a = app(lang);
        assert!(!text(&a).trim().is_empty());
        for keys in ["c", "t", "l", "q"] {
            let mut b = app(lang);
            // the first letter of the language's own label for the button
            let c = match keys {
                "c" => hot_letter(ctr(lang, CText::BtnConfig)),
                "t" => hot_letter(ctr(lang, CText::BtnTools)),
                "l" => hot_letter(ctr(lang, CText::BtnLoad)),
                _ => hot_letter(ctr(lang, CText::BtnQuit)),
            }
            .unwrap();
            b.handle(Input::Char(c));
            let t = text(&b);
            assert!(t.lines().count() > 10, "{lang:?} {keys}");
        }
        a.handle(Input::Esc);
        assert!(
            text(&a).contains(ctr(lang, CText::QStop)),
            "{lang:?}: Esc asks whether to quit"
        );
    }
}

#[test]
fn start_and_panelz_need_a_file() {
    let mut a = app(Lang::En);
    press(&mut a, "s");
    assert!(
        !text(&a).contains("Homing"),
        "Start does nothing without a file"
    );
    a.load_file(&testdata().join("T1.NCD"));
    let t = text(&a);
    assert!(t.contains("00003"), "three holes counted:\n{t}");
    assert!(t.contains("T1.NCD"));
    assert!(
        t.contains("025.40") && t.contains("050.80"),
        "bounds of the holes:\n{t}"
    );
}

#[test]
fn loading_a_file_through_the_dialog() {
    let mut a = app(Lang::En);
    press(&mut a, "l");
    assert!(text(&a).contains("Select File"));
    press(&mut a, "t1.ncd");
    a.handle(Input::Enter);
    // the original shows the tool data of the new file, and Enter closes it
    assert!(text(&a).contains("Tool Data"));
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("T1.NCD") && t.contains("00003"), "{t}");
}

#[test]
fn editing_a_field_checks_its_range() {
    let mut a = app(Lang::En);
    press(&mut a, "a");
    press(&mut a, "2");
    a.handle(Input::Enter);
    assert!(text(&a).contains("2.0000"), "scale taken:\n{}", text(&a));
    // 99 is above the 10.000 limit of the scale
    press(&mut a, "a");
    press(&mut a, "99");
    a.handle(Input::Enter);
    let t = text(&a);
    assert!(t.contains("Input error!") && t.contains("Max:"), "{t}");
    a.handle(Input::Enter);
    assert!(text(&a).contains("2.0000"), "the old value stays");
}

#[test]
fn quit_asks_first() {
    let mut a = app(Lang::En);
    press(&mut a, "q");
    assert!(text(&a).contains("Do you really want"));
    assert!(!a.wants_quit());
    press(&mut a, "n");
    assert!(!a.wants_quit());
    assert!(text(&a).contains("Scale") || text(&a).contains("DrillPro"));
    press(&mut a, "q");
    press(&mut a, "y");
    assert!(a.wants_quit());
}

#[test]
fn form_values_are_shown_like_the_original() {
    let a = app(Lang::En);
    let t = text(&a);
    for want in [
        "1.0000",
        "+002.50",
        "555.00",
        "405.00",
        "01.00",
        "12.00",
        "--------.---",
    ] {
        assert!(t.contains(want), "{want} missing:\n{t}");
    }
}
