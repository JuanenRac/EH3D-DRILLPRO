use eh3d_drillpro::i18n::*;
use eh3d_drillpro::texts::{tr, ALL_TEXTS};

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
        for l in Lang::ALL.into_iter().take(CORE_LANGS) {
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
    for l in Lang::ALL.into_iter().take(CORE_LANGS) {
        let mut seen: Vec<String> = Vec::new();
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
            let hot = t.split('~').nth(1).unwrap().to_ascii_lowercase();
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
fn every_new_language_has_all_the_texts_of_the_program() {
    use eh3d_drillpro::classic::texts::locale_coverage as classic;
    use eh3d_drillpro::texts::locale_coverage;
    for l in Lang::ALL.into_iter().skip(CORE_LANGS) {
        let variant = l.fallback().is_some();
        for (have, total) in [locale_coverage(l), classic(l)] {
            if variant {
                assert!(have < total, "{l:?}");
            } else {
                assert_eq!(have, total, "{l:?}");
            }
        }
    }
}

#[test]
fn the_files_of_the_languages_name_only_texts_that_exist() {
    assert!(eh3d_drillpro::texts::locale_problems().is_empty());
    assert!(eh3d_drillpro::classic::texts::locale_problems().is_empty());
}
