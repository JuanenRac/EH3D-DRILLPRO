//! Language support shared by the DRILLPRO and ROUTEPRO front ends.
//!
//! The eight languages of the project. English is the default; the others follow in a fixed order that matches [`Lang::ALL`].
//! A `~X~` pair marks the hot key (alt+X) of a menu item, as in the original programs; for
//! Chinese and Japanese the hot key is appended in Latin letters, e.g. `载入(~L~)`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[repr(usize)]
pub enum Lang {
    #[default]
    En = 0,
    Es = 1,
    It = 2,
    De = 3,
    Fr = 4,
    Pt = 5,
    Zh = 6,
    Ja = 7,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::It,
        Lang::De,
        Lang::Fr,
        Lang::Pt,
        Lang::Zh,
        Lang::Ja,
    ];

    /// Name of the language in that language.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::It => "Italiano",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Pt => "Português",
            Lang::Zh => "中文",
            Lang::Ja => "日本語",
        }
    }

    /// Short code used on the command line and in settings (`pt` is the Portuguese of Portugal).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::It => "it",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Pt => "pt",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
        }
    }

    /// The language of a code (`es`, `pt_BR`, `fr-ca`): a region gives its main language, so `pt-br` is read as `pt` and `fr-ca` as `fr` (the codes of settings
    /// written by earlier versions); a language the project does not have gives `None`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_ascii_lowercase().replace('_', "-");
        let c = c.split('.').next().unwrap_or("");
        let main = c.split('-').next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.code() == main)
    }

    /// Best guess from a locale string such as `es_ES.UTF-8`; English when unknown.
    pub fn from_locale(locale: &str) -> Lang {
        Lang::from_code(locale).unwrap_or_default()
    }

    /// The hot letter a typed character means: the letter in lower case.
    pub fn hot_key(self, c: char) -> char {
        fold_key(c)
    }
}

/// Defines a key enum plus a translation function from a table of eight strings per key, in the order of [`Lang::ALL`].
#[macro_export]
macro_rules! translations {
    ($key:ident, $tr:ident, $all:ident; $($name:ident => [$en:expr, $es:expr, $it:expr, $de:expr, $fr:expr, $pt:expr, $zh:expr, $ja:expr]),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $key { $($name),* }
        pub const $all: &[$key] = &[$($key::$name),*];
        pub fn $tr(lang: $crate::i18n::Lang, key: $key) -> &'static str {
            let row: [&'static str; 8] = match key { $($key::$name => [$en, $es, $it, $de, $fr, $pt, $zh, $ja]),* };
            row[lang as usize]
        }
    };
}

/// Display width of a character in text-mode cells: wide (CJK) characters take two.
pub fn cell_width(c: char) -> usize {
    let u = c as u32;
    let wide = (0x1100..=0x115F).contains(&u)
        || (0x2E80..=0xA4CF).contains(&u)
        || (0xAC00..=0xD7A3).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
        || (0xFE30..=0xFE6F).contains(&u)
        || (0xFF00..=0xFF60).contains(&u)
        || (0xFFE0..=0xFFE6).contains(&u);
    if wide {
        2
    } else {
        1
    }
}

/// The letter a typed character stands for in a hot key: lower case.
pub fn fold_key(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Display width of a string in cells, ignoring `~` hot-key markers.
pub fn str_width(s: &str) -> usize {
    s.chars().filter(|&c| c != '~').map(cell_width).sum()
}

/// The words of the summary live in the shared crates of the suite, which name the languages with their own type: the same language, by its code.
impl From<Lang> for eh3d_view::texts::Lang {
    fn from(l: Lang) -> Self {
        eh3d_view::texts::Lang::from_code(l.code()).unwrap_or(eh3d_view::texts::Lang::En)
    }
}
