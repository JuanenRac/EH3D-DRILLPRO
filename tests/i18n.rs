use eh3d_drillpro::classic::texts::{ctr, ALL_CTEXTS};
use eh3d_drillpro::cli::parse_prefs;
use eh3d_drillpro::i18n::*;
use eh3d_drillpro::texts::{failure_text, tr, Text, ALL_TEXTS};

#[test]
fn every_text_exists_in_all_eight_languages() {
    for &k in ALL_TEXTS {
        for l in Lang::ALL {
            assert!(!tr(l, k).trim().is_empty(), "{k:?} empty in {l:?}");
        }
    }
}

#[test]
fn hotkey_markers_are_balanced_and_consistent() {
    for &k in ALL_TEXTS {
        let en_has = tr(Lang::En, k).contains('~');
        for l in Lang::ALL {
            let t = tr(l, k);
            let n = t.matches('~').count();
            assert!(n == 0 || n == 2, "{k:?} {l:?}: {t}");
            assert_eq!(
                en_has,
                n > 0,
                "{k:?} {l:?} hotkey presence differs from English"
            );
        }
    }
}

#[test]
fn chinese_and_japanese_use_cjk() {
    for &k in ALL_TEXTS {
        for l in [Lang::Zh, Lang::Ja] {
            let t = tr(l, k);
            assert!(
                t.chars().any(|c| cell_width(c) == 2),
                "{k:?} {l:?} has no CJK: {t}"
            );
        }
    }
}

#[test]
fn menu_hotkeys_are_unique_within_a_language() {
    use eh3d_drillpro::texts::Text::*;
    for l in Lang::ALL {
        let mut seen: Vec<char> = Vec::new();
        for k in [
            MenuLoad,
            MenuVectors,
            MenuTools,
            MenuConfig,
            MenuSort,
            MenuStart,
            MenuLanguage,
            MenuQuit,
        ] {
            let t = tr(l, k);
            let hot = l.hot_key(t.split('~').nth(1).unwrap().chars().next().unwrap());
            assert!(!seen.contains(&hot), "{l:?}: hotkey {hot} repeated");
            seen.push(hot);
        }
    }
}

#[test]
fn language_codes_round_trip() {
    for l in Lang::ALL {
        assert_eq!(Lang::from_code(l.code()), Some(l));
    }
    assert_eq!(Lang::from_locale("es_ES.UTF-8"), Lang::Es);
    assert_eq!(Lang::from_locale("pt-PT"), Lang::Pt);
    assert_eq!(Lang::from_locale("xx"), Lang::En);
    assert_eq!(Lang::default(), Lang::En);
}

#[test]
fn the_project_has_eight_languages() {
    let codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
    assert_eq!(codes, ["en", "es", "it", "de", "fr", "pt", "zh", "ja"]);
}

#[test]
fn a_settings_file_with_a_code_of_an_earlier_version_is_still_read() {
    for (written, read) in [
        ("pt-br", Some(Lang::Pt)),
        ("fr-ca\nfullscreen=1\n", Some(Lang::Fr)),
        ("pt", Some(Lang::Pt)),
        // a language the program has no more: the default is used, nothing fails
        ("ru", None),
        ("eu\n", None),
    ] {
        assert_eq!(parse_prefs(written).lang, read, "{written:?}");
    }
    assert!(parse_prefs("fr-ca\nfullscreen=1\n").fullscreen);
}

/// The same holes in every language: `{}` is filled by the program.
#[test]
fn every_text_has_the_same_holes_in_every_language() {
    for &k in ALL_TEXTS {
        let en = tr(Lang::En, k).matches("{}").count();
        for l in Lang::ALL {
            assert_eq!(tr(l, k).matches("{}").count(), en, "{k:?} {l:?}");
        }
    }
    for &k in ALL_CTEXTS {
        let en = ctr(Lang::En, k).matches("{}").count();
        for l in Lang::ALL {
            assert!(!ctr(l, k).trim().is_empty(), "{k:?} empty in {l:?}");
            assert_eq!(ctr(l, k).matches("{}").count(), en, "{k:?} {l:?}");
        }
    }
}

#[test]
fn the_failures_of_the_program_are_said_in_the_language_of_the_screen() {
    // English, and a text the program did not word, stay as they are
    for why in ["arming the machine: no reply", "the server answered 503"] {
        assert_eq!(failure_text(Lang::En, why), why);
    }
    assert_eq!(
        failure_text(Lang::Es, "port COM9 not found"),
        "port COM9 not found"
    );
    assert_eq!(
        failure_text(Lang::Es, "arming the machine: no reply"),
        "al armar la máquina: no reply"
    );
    assert_eq!(
        failure_text(Lang::De, "the job failed on the server"),
        "der Auftrag ist auf dem Server fehlgeschlagen"
    );
    assert_eq!(
        failure_text(Lang::Pt, "internal error: the table of tools is damaged"),
        "erro interno: the table of tools is damaged"
    );
    assert_eq!(
        failure_text(
            Lang::Fr,
            "lost the server (time out); the job may still be running on it"
        ),
        "serveur perdu (time out) ; le travail est peut-être encore en cours"
    );
    assert_eq!(
        failure_text(Lang::It, "a.zip: no file of this kind in the archive"),
        "a.zip: nessun file di questo tipo nell'archivio"
    );
    for l in Lang::ALL {
        for k in [
            Text::FailNoConfig,
            Text::FailNotSupported,
            Text::FailInternal,
        ] {
            assert_eq!(failure_text(l, tr(Lang::En, k)), tr(l, k), "{l:?}");
        }
    }
}

/// A screen of text has 80 columns: the message of a terminal that is too small and a failure the program words fit.
#[test]
fn the_messages_fit_the_screen() {
    use Text::*;
    for l in Lang::ALL {
        let small = tr(l, SmallTerminal)
            .replacen("{}", "999", 1)
            .replacen("{}", "999", 1);
        assert!(str_width(&small) <= 80, "{l:?}: {small}");
        for k in [
            FailInternal,
            FailServerJob,
            FailNoJobId,
            FailNotSupported,
            FailNoConfig,
            FailBadConfig,
            FailNoArchiveFile,
        ] {
            assert!(str_width(tr(l, k)) <= 74, "{l:?} {k:?}: {}", tr(l, k));
        }
    }
}

#[test]
fn typed_letters_are_folded_to_the_hot_letter() {
    assert_eq!(fold_key('Q'), 'q');
    assert_eq!(fold_key('É'), 'é');
    assert_eq!(Lang::Pt.hot_key('S'), 's');
}
