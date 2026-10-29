//! Machine configuration (`DRILLPRO.DCF`, `ROUTEPRO.RCF`). Each data line is `value   * comment`;
//! a line whose first non-blank character is `*` followed by blank is a comment
//! (`*.NCD` is a value, not a comment).

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub serial_number: String,
    pub version: String,
    pub directory: String,
    pub default_extension: String,
    /// Remaining fields in file order (scale, offsets, speeds, ..., interface, baud selector,
    /// then the tool changer table when the machine has one).
    pub values: Vec<String>,
}

/// Removes the trailing `* comment`, which starts at a `*` preceded by whitespace.
fn strip_comment(line: &str) -> &str {
    let mut prev_blank = true;
    for (i, c) in line.char_indices() {
        if c == '*' && prev_blank {
            let next_blank = line[i + 1..].chars().next().is_none_or(char::is_whitespace);
            if next_blank {
                return &line[..i];
            }
        }
        prev_blank = c.is_whitespace();
    }
    line
}

pub fn parse(text: &str) -> Option<Config> {
    let data: Vec<String> = text
        .lines()
        .map(|l| strip_comment(l).trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let mut it = data.into_iter();
    Some(Config {
        serial_number: it.next()?,
        version: it.next()?,
        directory: it.next()?,
        default_extension: it.next()?,
        values: it.collect(),
    })
}

/// Unit of the coordinates of the drill or plot files (the `Coordinates unit` line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordUnit {
    TenthMil,
    Mil,
    /// 0.025 mm, the HP-GL plotter step.
    PlotterStep,
    HundredthMm,
}

impl CoordUnit {
    pub fn from_index(i: u32) -> Option<Self> {
        Some(match i {
            0 => Self::TenthMil,
            1 => Self::Mil,
            2 => Self::PlotterStep,
            3 => Self::HundredthMm,
            _ => return None,
        })
    }

    pub fn index(self) -> u32 {
        match self {
            Self::TenthMil => 0,
            Self::Mil => 1,
            Self::PlotterStep => 2,
            Self::HundredthMm => 3,
        }
    }

    pub fn mm(self) -> f64 {
        match self {
            Self::TenthMil => 0.00254,
            Self::Mil => 0.0254,
            Self::PlotterStep => 0.025,
            Self::HundredthMm => 0.01,
        }
    }
}

/// How the drill holes are ordered before drilling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    None,
    XFirst,
    YFirst,
    Special,
}

impl SortMode {
    pub fn index(self) -> u32 {
        match self {
            Self::None => 0,
            Self::XFirst => 1,
            Self::YFirst => 2,
            Self::Special => 3,
        }
    }
}

/// One position of the automatic tool changer (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtcSlot {
    pub index: u8,
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

/// Typed view of the configuration file.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineSettings {
    pub serial_number: String,
    pub scale: (f64, f64),
    pub offset_mm: (f64, f64),
    /// Largest XY travel in mm.
    pub clip_mm: (f64, f64),
    pub board_mm: f64,
    pub base_mm: f64,
    /// Speeds in mm/min: (normal, reference, ramp start) for XY and for Z.
    pub speed_xy: (u32, u32, u32),
    pub speed_z: (u32, u32, u32),
    pub accel: (u32, u32),
    /// Calibration coefficients of X, Y and Z.
    pub correction: (f64, f64, f64),
    pub z_max_depth_mm: f64,
    pub distance_tool_board_mm: f64,
    pub spindle_start_time: f64,
    pub unit: CoordUnit,
    pub sorting: SortMode,
    /// Chord angle of the routing program in degrees (the line where the drilling program keeps its sorting); 0 for drilling.
    pub chord_angle: i32,
    /// Serial interface index: 0..3 = COM1..COM4, 4 = demo.
    pub interface: u8,
    /// Baud selector: 0 = 9600 via DOS, 1 = 19200, 2 = 38400.
    pub baud: u8,
    /// The automatic tool changer is on (line "Tool change 1=Auto, 0=Man." of the ATC models' files; the manual models do not have it).
    pub auto_change: bool,
    /// The rack: position 0 is the test pin, 1 to 14 the tools and 15 the drill break detector.
    pub atc: Vec<AtcSlot>,
}

impl Config {
    fn f(&self, i: usize) -> Option<f64> {
        self.values.get(i)?.split_whitespace().next()?.parse().ok()
    }

    fn u(&self, i: usize) -> Option<u32> {
        self.values.get(i)?.split_whitespace().next()?.parse().ok()
    }

    /// Parses the numeric fields. `None` when the file does not have the expected layout.
    pub fn settings(&self) -> Option<MachineSettings> {
        self.parse_settings(false)
    }

    /// The same for the routing program, whose line 23 is the HP-GL chord angle instead of the sorting.
    pub fn route_settings(&self) -> Option<MachineSettings> {
        self.parse_settings(true)
    }

    fn parse_settings(&self, route: bool) -> Option<MachineSettings> {
        let unit = CoordUnit::from_index(self.u(22)?)?;
        let (sorting, chord_angle) = if route {
            let angle: i32 = self
                .values
                .get(23)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()?;
            (SortMode::None, angle)
        } else {
            let sorting = match self.u(23)? {
                0 => SortMode::None,
                1 => SortMode::XFirst,
                2 => SortMode::YFirst,
                3 => SortMode::Special,
                _ => return None,
            };
            (sorting, 0)
        };
        let atc = self
            .values
            .iter()
            .skip(27)
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                Some(AtcSlot {
                    index: it.next()?.parse().ok()?,
                    x_mm: it.next()?.parse().ok()?,
                    y_mm: it.next()?.parse().ok()?,
                    z_mm: it.next()?.parse().ok()?,
                })
            })
            .collect();
        Some(MachineSettings {
            serial_number: self.serial_number.clone(),
            scale: (self.f(0)?, self.f(1)?),
            offset_mm: (self.f(2)?, self.f(3)?),
            clip_mm: (self.f(4)?, self.f(5)?),
            board_mm: self.f(6)?,
            base_mm: self.f(7)?,
            speed_xy: (self.u(8)?, self.u(9)?, self.u(10)?),
            speed_z: (self.u(11)?, self.u(12)?, self.u(13)?),
            accel: (self.u(14)?, self.u(15)?),
            correction: (self.f(16)?, self.f(17)?, self.f(18)?),
            z_max_depth_mm: self.f(19)?,
            distance_tool_board_mm: self.f(20)?,
            spindle_start_time: self.f(21)?,
            unit,
            sorting,
            chord_angle,
            interface: self.u(25)? as u8,
            baud: self.u(26)? as u8,
            auto_change: self.u(27) == Some(1),
            atc,
        })
    }
}

/// Writes `settings` back into the text of a configuration file, keeping every comment, the blank lines, the reserved
/// lines and the layout: only the first value of each data line that the program edits is replaced (and padded to the
/// width of the old one, so the comments stay in their column). `route` selects the routing layout (chord angle on the
/// line where the drilling program keeps its sorting).
pub fn rewrite(
    original: &str,
    settings: &MachineSettings,
    directory: &str,
    pattern: &str,
    route: bool,
) -> String {
    let chord = if settings.chord_angle >= 0 {
        format!("{:02}", settings.chord_angle)
    } else {
        format!("{:03}", settings.chord_angle)
    };
    let value = |j: usize| -> Option<String> {
        let s = settings;
        Some(match j {
            0 => format!("{:.4}", s.scale.0),
            1 => format!("{:.4}", s.scale.1),
            2 => format!("{:05.2}", s.offset_mm.0),
            3 => format!("{:05.2}", s.offset_mm.1),
            4 => format!("{:06.2}", s.clip_mm.0),
            5 => format!("{:06.2}", s.clip_mm.1),
            6 => format!("{:.2}", s.board_mm),
            7 => format!("{:.2}", s.base_mm),
            8 => s.speed_xy.0.to_string(),
            9 => s.speed_xy.1.to_string(),
            10 => s.speed_xy.2.to_string(),
            11 => s.speed_z.0.to_string(),
            12 => s.speed_z.1.to_string(),
            13 => s.speed_z.2.to_string(),
            14 => s.accel.0.to_string(),
            15 => s.accel.1.to_string(),
            16 => format!("{:.4}", s.correction.0),
            17 => format!("{:.4}", s.correction.1),
            18 => format!("{:.4}", s.correction.2),
            19 => format!("{:.2}", s.z_max_depth_mm),
            20 => format!("{:.2}", s.distance_tool_board_mm),
            21 => format!("{:.2}", s.spindle_start_time),
            22 => s.unit.index().to_string(),
            23 => {
                if route {
                    chord.clone()
                } else {
                    s.sorting.index().to_string()
                }
            }
            // only the ATC models' files have this line; the rewrite only replaces lines that exist
            27 => if s.auto_change { "1" } else { "0" }.to_string(),
            _ => return None,
        })
    };
    let mut out = String::with_capacity(original.len() + 64);
    let mut data_index = 0usize;
    for line in original.split_inclusive('\n') {
        let is_data = !strip_comment(line).trim().is_empty();
        if !is_data {
            out.push_str(line);
            continue;
        }
        let new = match data_index {
            2 => Some(directory.to_string()),
            3 => Some(pattern.to_string()),
            k if k >= 4 => value(k - 4),
            _ => None,
        };
        data_index += 1;
        let Some(new) = new else {
            out.push_str(line);
            continue;
        };
        let start = line.len() - line.trim_start().len();
        let token_len = line[start..]
            .find(char::is_whitespace)
            .unwrap_or(line.len() - start);
        let pad = " ".repeat(token_len.saturating_sub(new.len()));
        out.push_str(&line[..start]);
        out.push_str(&new);
        out.push_str(&pad);
        out.push_str(&line[start + token_len..]);
    }
    out
}

/// The folder the files of the jobs live in: the `Directory` line of the configuration (DOS style, `.\RACK3000`)
/// under `data_dir`, when that folder exists; `data_dir` itself otherwise.
pub fn work_dir(data_dir: &std::path::Path, cfg: Option<&Config>) -> std::path::PathBuf {
    let Some(c) = cfg else {
        return data_dir.to_path_buf();
    };
    let rel = c.directory.trim().replace('\\', "/");
    let rel = rel.trim_start_matches("./").trim_start_matches('/');
    if rel.is_empty() || rel == "." {
        return data_dir.to_path_buf();
    }
    let dir = data_dir.join(rel);
    if dir.is_dir() {
        dir
    } else {
        data_dir.to_path_buf()
    }
}

/// Bytes of a file read as text (the files are DOS text: one character per byte).
pub fn bytes_to_text(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// The text written back as bytes (characters above 255 become `?`).
pub fn text_to_bytes(t: &str) -> Vec<u8> {
    t.chars()
        .map(|c| u8::try_from(c as u32).unwrap_or(b'?'))
        .collect()
}

/// Writes `text` to `path`, keeping the old file as `<name>.BAK` next to it.
pub fn write_with_backup(path: &std::path::Path, text: &str) -> Result<(), String> {
    if path.exists() {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".BAK");
        std::fs::copy(path, std::path::PathBuf::from(bak)).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text_to_bytes(text)).map_err(|e| e.to_string())
}

/// Where the program looks for its files when no folder is given: the current folder, the folder of the program, and
/// the user's configuration folder (`%APPDATA%\EH3D`, `~/.config/eh3d`); the first one that holds `file` (for example
/// `DRILLPRO.DCF`) wins, and the current folder is the answer when none does.
pub fn find_data_dir(file: &str) -> std::path::PathBuf {
    let mut candidates = vec![std::path::PathBuf::from(".")];
    if let Some(exe) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
    {
        candidates.push(exe);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        candidates.push(std::path::PathBuf::from(appdata).join("EH3D"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(std::path::PathBuf::from(home).join(".config").join("eh3d"));
    }
    candidates
        .into_iter()
        .find(|d| d.join(file).is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
