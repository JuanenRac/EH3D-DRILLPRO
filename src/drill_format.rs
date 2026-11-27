//! The format of the drill files that do not declare one. A file whose scale would have to be guessed is refused until the person chooses how its numbers are
//! written; the choice is kept in `DRILLPRO.FMT` of the data folder, with the machine file. What a file declares always wins over the choice.

use std::path::Path;

pub use eh3d_bungard_formats::ncd::{how_to_choose, NumberFormat};

use crate::i18n::Lang;
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
