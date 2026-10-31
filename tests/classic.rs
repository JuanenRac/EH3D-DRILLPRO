//! The classic front end: texts in eight languages, layout limits and the keyboard flow.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eh3d_drillpro::app::Input;
use eh3d_drillpro::classic::texts::{ctr, CText, ALL_CTEXTS};
use eh3d_drillpro::classic::{hot_letter, ClassicApp};
use eh3d_drillpro::i18n::{str_width, Lang, CORE_LANGS};
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

/// The controls that share a screen: no two of them may need the same key, in any of the 37 languages.
fn screens() -> Vec<(&'static str, Vec<CText>)> {
    let parameter = |first: CText| {
        vec![
            CText::PFile,
            CText::PCoordinates,
            first,
            CText::PSpeed,
            CText::PRamp,
            CText::PStroke,
            CText::PCorrect,
            CText::PSpindle,
            CText::PLanguage,
            CText::BLoad,
            CText::BOk,
            CText::BAbort,
        ]
    };
    vec![
        (
            "main",
            vec![
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
                CText::BtnAbout,
            ],
        ),
        ("parameter", parameter(CText::PSorting)),
        ("parameter of the routing program", parameter(CText::PChord)),
        ("tools", vec![CText::BLoad, CText::BOk, CText::BAbort]),
        ("panel", vec![CText::PzCalc, CText::BOk, CText::BAbort]),
        (
            "files",
            vec![CText::FName, CText::FFiles, CText::BOkUpper, CText::BCancel],
        ),
        ("quit", vec![CText::BYes, CText::BNo]),
        ("interrupted", vec![CText::BCont, CText::BAbort]),
        (
            "teach",
            vec![CText::BTeach, CText::BAbort, CText::BOk, CText::BMod],
        ),
    ]
}

/// Controls that share a screen must have different hot letters in every language (the letter a key types, so a Latin O and a Cyrillic O
/// of a Russian screen count as the keys they are).
#[test]
fn hot_letters_are_unique_on_every_screen() {
    for lang in Lang::ALL {
        for (screen, keys) in screens() {
            let mut seen = HashSet::new();
            for k in keys {
                let label = ctr(lang, k);
                let hot = hot_letter(label)
                    .unwrap_or_else(|| panic!("{lang:?} {screen}: no hot letter in {label:?}"));
                let typed = lang.hot_key(hot);
                assert!(
                    seen.insert(typed),
                    "{lang:?} {screen}: the hot letter {hot:?} of {label:?} is used twice"
                );
            }
        }
    }
}

/// Every text of every language has no mark or one pair, and has the pair when the English text has it.
#[test]
fn every_classic_text_has_its_hot_letter_in_every_language() {
    for &k in ALL_CTEXTS {
        let en = ctr(Lang::En, k).contains('~');
        for lang in Lang::ALL {
            let t = ctr(lang, k);
            let n = t.matches('~').count();
            assert!(n == 0 || n == 2, "{lang:?} {k:?}: {t:?}");
            // the Chinese and Japanese texts of the original have no mark on a few labels
            if lang as usize >= CORE_LANGS {
                assert_eq!(en, n > 0, "{lang:?} {k:?}: {t:?}");
            }
        }
    }
}

/// The teach window keeps six keys for the arrows: no hot letter of its buttons may be one of them (in the layout of the language).
#[test]
fn teach_buttons_leave_the_arrow_keys_free() {
    for lang in Lang::ALL {
        for k in [CText::BTeach, CText::BAbort, CText::BOk, CText::BMod] {
            let hot = lang.hot_key(hot_letter(ctr(lang, k)).unwrap());
            for fixed in "esdxrc".chars() {
                assert_ne!(
                    hot,
                    lang.hot_key(fixed),
                    "{lang:?} {k:?} takes the key of an arrow"
                );
            }
        }
    }
}

/// A letter of the Cyrillic or Greek alphabet works when it is typed, in either case, and so does the Latin key that sits where it is.
#[test]
fn a_cyrillic_or_greek_hot_letter_works_when_typed() {
    // Russian: Quit is the "В" of "Выход"; "d" is the key of "в" in the Russian layout
    for typed in ['в', 'В', 'd', 'D'] {
        let mut a = app(Lang::Ru);
        a.handle(Input::Char(typed));
        let q = ctr(Lang::Ru, CText::QAsk);
        let first: String = q.chars().take(8).collect();
        assert!(text(&a).contains(&first), "{typed:?}");
    }
    // Greek: Quit is the "Έ" of "Έξοδος"; the accent does not have to be typed and "e" is the key of "ε"
    for typed in ['Έ', 'έ', 'ε', 'e'] {
        let mut a = app(Lang::El);
        a.handle(Input::Char(typed));
        let q = ctr(Lang::El, CText::QAsk);
        let first: String = q.chars().take(8).collect();
        assert!(text(&a).contains(&first), "{typed:?}");
    }
}

/// The labels must fit the room the original layout gives them.
#[test]
fn labels_fit_their_slots() {
    let limits: [(CText, usize); 22] = [
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
        // the signs of the questions have a frame of ten cells
        (CText::QStop, 10),
        (CText::QAttention, 10),
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
        // (a long question of another language may be cut at the edge of the screen: its first words are there)
        let q = ctr(lang, CText::QStop);
        let first: String = q.chars().take(12).collect();
        assert!(
            text(&a).contains(&first),
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
        "Start does not run without a file"
    );
    assert!(text(&a).contains("Load a file first"));
    a.handle(Input::Enter);
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
