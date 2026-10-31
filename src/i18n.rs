//! Language support shared by the DRILLPRO and ROUTEPRO front ends.
//!
//! English is the default; the others follow in a fixed order that matches [`Lang::ALL`].
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
    Bg = 8,
    Hr = 9,
    Cs = 10,
    Da = 11,
    Nl = 12,
    Et = 13,
    Fi = 14,
    El = 15,
    Hu = 16,
    Ga = 17,
    Lv = 18,
    Lt = 19,
    Mt = 20,
    Pl = 21,
    Ro = 22,
    Sk = 23,
    Sl = 24,
    Sv = 25,
    No = 26,
    Is = 27,
    Uk = 28,
    Ru = 29,
    Sr = 30,
    Sq = 31,
    Ca = 32,
    Gl = 33,
    Eu = 34,
    PtBr = 35,
    FrCa = 36,
}

/// How many languages have their words inside the program (the first of [`Lang::ALL`]); the rest are read from the files of `locales/`.
pub const CORE_LANGS: usize = 8;

impl Lang {
    pub const ALL: [Lang; 37] = [
        Lang::En,
        Lang::Es,
        Lang::It,
        Lang::De,
        Lang::Fr,
        Lang::Pt,
        Lang::Zh,
        Lang::Ja,
        Lang::Bg,
        Lang::Hr,
        Lang::Cs,
        Lang::Da,
        Lang::Nl,
        Lang::Et,
        Lang::Fi,
        Lang::El,
        Lang::Hu,
        Lang::Ga,
        Lang::Lv,
        Lang::Lt,
        Lang::Mt,
        Lang::Pl,
        Lang::Ro,
        Lang::Sk,
        Lang::Sl,
        Lang::Sv,
        Lang::No,
        Lang::Is,
        Lang::Uk,
        Lang::Ru,
        Lang::Sr,
        Lang::Sq,
        Lang::Ca,
        Lang::Gl,
        Lang::Eu,
        Lang::PtBr,
        Lang::FrCa,
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
            Lang::Bg => "Български",
            Lang::Hr => "Hrvatski",
            Lang::Cs => "Čeština",
            Lang::Da => "Dansk",
            Lang::Nl => "Nederlands",
            Lang::Et => "Eesti",
            Lang::Fi => "Suomi",
            Lang::El => "Ελληνικά",
            Lang::Hu => "Magyar",
            Lang::Ga => "Gaeilge",
            Lang::Lv => "Latviešu",
            Lang::Lt => "Lietuvių",
            Lang::Mt => "Malti",
            Lang::Pl => "Polski",
            Lang::Ro => "Română",
            Lang::Sk => "Slovenčina",
            Lang::Sl => "Slovenščina",
            Lang::Sv => "Svenska",
            Lang::No => "Norsk",
            Lang::Is => "Íslenska",
            Lang::Uk => "Українська",
            Lang::Ru => "Русский",
            Lang::Sr => "Српски",
            Lang::Sq => "Shqip",
            Lang::Ca => "Català",
            Lang::Gl => "Galego",
            Lang::Eu => "Euskara",
            Lang::PtBr => "Português (Brasil)",
            Lang::FrCa => "Français (Canada)",
        }
    }

    /// Short code used on the command line and in settings (`pt-br` and `fr-ca` for the variants).
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
            Lang::Bg => "bg",
            Lang::Hr => "hr",
            Lang::Cs => "cs",
            Lang::Da => "da",
            Lang::Nl => "nl",
            Lang::Et => "et",
            Lang::Fi => "fi",
            Lang::El => "el",
            Lang::Hu => "hu",
            Lang::Ga => "ga",
            Lang::Lv => "lv",
            Lang::Lt => "lt",
            Lang::Mt => "mt",
            Lang::Pl => "pl",
            Lang::Ro => "ro",
            Lang::Sk => "sk",
            Lang::Sl => "sl",
            Lang::Sv => "sv",
            Lang::No => "no",
            Lang::Is => "is",
            Lang::Uk => "uk",
            Lang::Ru => "ru",
            Lang::Sr => "sr",
            Lang::Sq => "sq",
            Lang::Ca => "ca",
            Lang::Gl => "gl",
            Lang::Eu => "eu",
            Lang::PtBr => "pt-br",
            Lang::FrCa => "fr-ca",
        }
    }

    /// A variant takes the words of its main language where it does not change them.
    pub fn fallback(self) -> Option<Lang> {
        match self {
            Lang::PtBr => Some(Lang::Pt),
            Lang::FrCa => Some(Lang::Fr),
            _ => None,
        }
    }

    /// The language of a code (`es`, `pt_BR`, `fr-ca`); a region that is not a variant of the project gives its main language.
    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_ascii_lowercase().replace('_', "-");
        let c = c.split('.').next().unwrap_or("").to_string();
        if let Some(l) = Lang::ALL.into_iter().find(|l| l.code() == c) {
            return Some(l);
        }
        let main = c.split('-').next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.code() == main)
    }

    /// Best guess from a locale string such as `es_ES.UTF-8`; English when unknown.
    pub fn from_locale(locale: &str) -> Lang {
        Lang::from_code(locale).unwrap_or_default()
    }
}

/// The words of the languages that are read from files: one map per language of [`Lang::ALL`], from `Table.Name` to the text (`Name<TAB>text` per line in the file, `#` for comments, `\n` for a line break).
pub type Locales = Vec<std::collections::HashMap<&'static str, &'static str>>;

pub fn load_locales(source: fn(Lang) -> &'static str) -> Locales {
    Lang::ALL
        .iter()
        .map(|l| {
            let mut m = std::collections::HashMap::new();
            for line in source(*l).lines() {
                let line = line.trim_end_matches('\r');
                if line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('\t') {
                    if !v.trim().is_empty() {
                        let v: &'static str = Box::leak(v.replace("\\n", "\n").into_boxed_str());
                        m.insert(
                            Box::leak(k.trim().to_string().into_boxed_str()) as &'static str,
                            v,
                        );
                    }
                }
            }
            m
        })
        .collect()
}

/// The text of a language: its own file, then the main language of a variant, then English.
pub fn pick(
    lang: Lang,
    row: [&'static str; CORE_LANGS],
    key: &str,
    locales: &Locales,
) -> &'static str {
    let mut l = lang;
    loop {
        let i = l as usize;
        if i < CORE_LANGS {
            return row[i];
        }
        if let Some(s) = locales.get(i).and_then(|m| m.get(key)) {
            return s;
        }
        match l.fallback() {
            Some(base) => l = base,
            None => return row[0],
        }
    }
}

/// The source of the file of a language of a crate (the `locales/` folder of the crate that defines the texts).
#[macro_export]
macro_rules! locale_source {
    ($l:expr) => {
        match $l {
            $crate::i18n::Lang::Bg => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/bg.tsv"))
            }
            $crate::i18n::Lang::Hr => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/hr.tsv"))
            }
            $crate::i18n::Lang::Cs => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/cs.tsv"))
            }
            $crate::i18n::Lang::Da => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/da.tsv"))
            }
            $crate::i18n::Lang::Nl => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/nl.tsv"))
            }
            $crate::i18n::Lang::Et => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/et.tsv"))
            }
            $crate::i18n::Lang::Fi => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/fi.tsv"))
            }
            $crate::i18n::Lang::El => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/el.tsv"))
            }
            $crate::i18n::Lang::Hu => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/hu.tsv"))
            }
            $crate::i18n::Lang::Ga => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/ga.tsv"))
            }
            $crate::i18n::Lang::Lv => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/lv.tsv"))
            }
            $crate::i18n::Lang::Lt => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/lt.tsv"))
            }
            $crate::i18n::Lang::Mt => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/mt.tsv"))
            }
            $crate::i18n::Lang::Pl => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/pl.tsv"))
            }
            $crate::i18n::Lang::Ro => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/ro.tsv"))
            }
            $crate::i18n::Lang::Sk => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/sk.tsv"))
            }
            $crate::i18n::Lang::Sl => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/sl.tsv"))
            }
            $crate::i18n::Lang::Sv => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/sv.tsv"))
            }
            $crate::i18n::Lang::No => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/no.tsv"))
            }
            $crate::i18n::Lang::Is => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/is.tsv"))
            }
            $crate::i18n::Lang::Uk => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/uk.tsv"))
            }
            $crate::i18n::Lang::Ru => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/ru.tsv"))
            }
            $crate::i18n::Lang::Sr => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/sr.tsv"))
            }
            $crate::i18n::Lang::Sq => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/sq.tsv"))
            }
            $crate::i18n::Lang::Ca => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/ca.tsv"))
            }
            $crate::i18n::Lang::Gl => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/gl.tsv"))
            }
            $crate::i18n::Lang::Eu => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/eu.tsv"))
            }
            $crate::i18n::Lang::PtBr => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/pt-br.tsv"))
            }
            $crate::i18n::Lang::FrCa => {
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/fr-ca.tsv"))
            }
            _ => "",
        }
    };
}

/// Defines a key enum plus a translation function from a table of eight strings per key (the other languages come from the files of `locales/`, keyed by `Table.Name`).
#[macro_export]
macro_rules! translations {
    ($key:ident, $tr:ident, $all:ident; $($name:ident => [$en:expr, $es:expr, $it:expr, $de:expr, $fr:expr, $pt:expr, $zh:expr, $ja:expr]),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $key { $($name),* }
        pub const $all: &[$key] = &[$($key::$name),*];
        impl $key {
            /// The name of the text in the files of the languages: `Table.Name`.
            pub fn locale_key(self) -> &'static str {
                match self { $($key::$name => concat!(stringify!($key), ".", stringify!($name))),* }
            }
        }
        fn locales() -> &'static $crate::i18n::Locales {
            static MAPS: std::sync::OnceLock<$crate::i18n::Locales> = std::sync::OnceLock::new();
            MAPS.get_or_init(|| $crate::i18n::load_locales(|l| $crate::locale_source!(l)))
        }
        pub fn $tr(lang: $crate::i18n::Lang, key: $key) -> &'static str {
            let row = match key { $($key::$name => [$en, $es, $it, $de, $fr, $pt, $zh, $ja]),* };
            $crate::i18n::pick(lang, row, key.locale_key(), locales())
        }
        /// How many of the texts of this table a language has in its file, and how many there are (for the tests).
        pub fn locale_coverage(lang: $crate::i18n::Lang) -> (usize, usize) {
            let table = concat!(stringify!($key), ".");
            let have = locales()
                .get(lang as usize)
                .map_or(0, |m| m.keys().filter(|k| k.starts_with(table)).count());
            (have, $all.len())
        }
        /// The problems of the files of the languages: a text that does not exist, an empty one (for the tests).
        pub fn locale_problems() -> Vec<String> {
            let names: std::collections::HashSet<&str> = $all.iter().map(|k| k.locale_key()).collect();
            let table = concat!(stringify!($key), ".");
            let mut out = Vec::new();
            for (i, m) in locales().iter().enumerate() {
                for k in m.keys().filter(|k| k.starts_with(table)) {
                    if !names.contains(k) {
                        out.push(format!("{:?}: unknown text {k}", $crate::i18n::Lang::ALL[i]));
                    }
                }
            }
            out
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

/// Display width of a string in cells, ignoring `~` hot-key markers.
pub fn str_width(s: &str) -> usize {
    s.chars().filter(|&c| c != '~').map(cell_width).sum()
}
