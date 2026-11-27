//! The format of the drill files that do not declare one. A file whose scale would have to be guessed is refused until the person chooses how its numbers are
//! written; the choice is kept in `DRILLPRO.FMT` of the data folder, with the machine file. What a file declares always wins over the choice.

use std::path::Path;

pub use eh3d_bungard_formats::ncd::{how_to_choose, NumberFormat};

use crate::i18n::{str_width, Lang};
use crate::texts::{tr, Text};

/// The file of the data folder that keeps the choice.
pub const FILE: &str = "DRILLPRO.FMT";

/// The choice kept in `data_dir`: the first line that is not empty and not a `#` comment. No file is no choice; a line that is no format gives it as the error.
pub fn load(data_dir: &Path) -> Result<Option<NumberFormat>, String> {
    let Ok(text) = std::fs::read_to_string(data_dir.join(FILE)) else {
        return Ok(None);
    };
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("");
    NumberFormat::from_setting(line)
}

/// Keeps `chosen` in `data_dir` (`None`: ask, that is, refuse a file that does not declare its format).
pub fn save(data_dir: &Path, chosen: Option<NumberFormat>) -> std::io::Result<()> {
    std::fs::write(
        data_dir.join(FILE),
        format!(
            "# Format of the drill files that do not declare one: ask, mils, inch-2.4-tz, inch-2.4-lz, metric-3.3-tz, metric-3.3-lz\n{}\n",
            NumberFormat::setting(chosen)
        ),
    )
}

/// What is said after a file is refused for its format: how to choose one, in the language of the screen.
pub fn help(lang: Lang) -> String {
    format!(
        "{}. {}",
        how_to_choose(lang.code()),
        tr(lang, Text::HowFormat)
    )
}

/// What a chooser offers: no format (the file stays refused) first, then the formats of the list, and the one in use when it is none of those (one written by name).
pub fn choices(current: Option<NumberFormat>) -> Vec<Option<NumberFormat>> {
    let mut list: Vec<Option<NumberFormat>> = vec![None];
    list.extend(NumberFormat::PRESETS.map(Some));
    if !list.contains(&current) {
        list.push(current);
    }
    list
}

/// The words of a choice: the name of the format, or that there is none.
pub fn label(lang: Lang, chosen: Option<NumberFormat>) -> String {
    chosen.map_or_else(
        || tr(lang, Text::FormatNone).to_string(),
        NumberFormat::name,
    )
}

/// `text` cut into lines of at most `width` cells of the screen, at the spaces where there are any.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for c in text.chars() {
        if str_width(&line) + str_width(c.encode_utf8(&mut [0; 4])) > width {
            // the word that does not fit goes whole to the next line, when the line has a space to cut at
            let rest = match line.rfind(' ') {
                Some(at) if c != ' ' => line.split_off(at + 1),
                _ => String::new(),
            };
            lines.push(line.trim_end().to_string());
            line = rest;
        }
        if !(line.is_empty() && c == ' ') {
            line.push(c);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_help_fits_the_chooser_in_every_language_and_nothing_is_chosen_first() {
        for lang in Lang::ALL {
            let lines = wrap(how_to_choose(lang.code()), 64);
            assert!(lines.len() <= 7, "{lang:?}: {}", lines.len());
            assert!(lines.iter().all(|l| str_width(l) <= 64), "{lang:?}");
            let said: String = lines.concat().split_whitespace().collect();
            let whole: String = how_to_choose(lang.code()).split_whitespace().collect();
            assert_eq!(said, whole, "{lang:?}");
        }
        assert_eq!(choices(None)[0], None);
        assert_eq!(choices(None).len(), 6);
        let own = NumberFormat::from_name("inch-2.5-tz");
        assert_eq!(choices(own).len(), 7);
        assert_eq!(label(Lang::En, own), "inch-2.5-tz");
    }
}
