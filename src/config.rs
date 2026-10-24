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
    /// Serial interface index: 0..3 = COM1..COM4, 4 = demo.
    pub interface: u8,
    /// Baud selector: 0 = 9600 via DOS, 1 = 19200, 2 = 38400.
    pub baud: u8,
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
        let unit = CoordUnit::from_index(self.u(22)?)?;
        let sorting = match self.u(23)? {
            0 => SortMode::None,
            1 => SortMode::XFirst,
            2 => SortMode::YFirst,
            3 => SortMode::Special,
            _ => return None,
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
            interface: self.u(25)? as u8,
            baud: self.u(26)? as u8,
            atc,
        })
    }
}
