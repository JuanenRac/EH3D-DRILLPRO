//! The sounds of a job: one when it ends and one when the machine asks for a tool, each one switched on or off and each one with the tone
//! the program makes itself or a sound file of the operator's (a WAV file). It is set in the machine file, which DRILLPRO, ROUTEPRO and
//! the modern interface share:
//!
//! ```text
//! sound_finish = default                  # off | default | the path of a WAV file
//! sound_tool_change = C:\sounds\ding.wav
//! ```
//!
//! The sound is played by the system (a short PowerShell command on Windows, `afplay` on macOS, `paplay`, `aplay` or `play` on Linux) in
//! a process of its own: it never waits for it and it never fails the job if there is no sound card or no player.

use std::path::{Path, PathBuf};

/// What a sound is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// The job has ended.
    Finished,
    /// The machine asks for a tool.
    ToolChange,
}

/// A sound as the machine file sets it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SoundSetting {
    /// No sound.
    #[default]
    Off,
    /// The tone that the program makes.
    Default,
    /// A sound file of the operator's (WAV).
    File(PathBuf),
    /// A sound of the catalogue of the program (see [`TONES`]): a pitch and a pattern.
    Tone(u8),
}

impl SoundSetting {
    /// The text of the machine file: `off`, `default` or the path.
    pub fn to_text(&self) -> String {
        match self {
            SoundSetting::Off => "off".into(),
            SoundSetting::Default => "default".into(),
            SoundSetting::File(p) => p.display().to_string(),
            SoundSetting::Tone(i) => format!("tone:{i}"),
        }
    }

    /// The other way round; an empty text is `off`.
    pub fn parse(text: &str) -> SoundSetting {
        let t = text.trim().trim_matches('"');
        match t.to_ascii_lowercase().as_str() {
            "" | "off" | "no" | "false" | "0" => SoundSetting::Off,
            "default" | "on" | "yes" | "true" | "1" => SoundSetting::Default,
            other => match other
                .strip_prefix("tone:")
                .and_then(|n| n.trim().parse::<usize>().ok())
            {
                Some(i) if i < TONES => SoundSetting::Tone(i as u8),
                _ => SoundSetting::File(PathBuf::from(t)),
            },
        }
    }

    pub fn is_on(&self) -> bool {
        *self != SoundSetting::Off
    }
}

/// The notes of a cue as (frequency in Hz, length in ms): a rising arpeggio when the job ends, two bells for a tool.
fn notes(cue: Cue) -> &'static [(f32, u32)] {
    match cue {
        Cue::Finished => &[(523.25, 140), (659.25, 140), (783.99, 140), (1046.5, 420)],
        Cue::ToolChange => &[(880.0, 160), (0.0, 60), (880.0, 160), (1174.66, 300)],
    }
}

/// The tone of a cue as a WAV file (16 bits, mono, 22050 Hz).
pub fn tone_wav(cue: Cue) -> Vec<u8> {
    wav_of(notes(cue))
}

// ---- the catalogue: sixty sounds for each cue, from the high ones to the low ones

/// How many sounds each cue has in the catalogue: ten pitches, from high to low, in six patterns each.
pub const TONES: usize = 60;
/// How many patterns each pitch has.
pub const PATTERNS: usize = 6;

/// The ten pitches of the catalogue, from the highest to the lowest, in Hz: C7, G6, D6, A5, E5, C5, G4, D4, G3 and C3.
pub const PITCHES: [f32; 10] = [
    2093.00, 1567.98, 1174.66, 880.00, 659.25, 523.25, 392.00, 293.66, 196.00, 130.81,
];

/// The note names of [`PITCHES`], for the lists.
pub const PITCH_NAMES: [&str; 10] = ["C7", "G6", "D6", "A5", "E5", "C5", "G4", "D4", "G3", "C3"];

/// The pitch (0 the highest) and the pattern (0 to 5) of a sound of the catalogue. The number goes from 0 to [`TONES`] - 1: the sounds are ordered from the
/// highest pitch to the lowest and, inside a pitch, by pattern.
pub fn tone_parts(index: usize) -> (usize, usize) {
    let i = index.min(TONES - 1);
    (i / PATTERNS, i % PATTERNS)
}

/// The notes of a sound of the catalogue as (frequency in Hz, length in ms; frequency 0 is a silence). The patterns of the sound for the end of the job are
/// the chime, the rise, the fall, the fanfare, the triple and the bell; the ones for a tool are the beep, the double, the triple beep, the alarm, the knock and
/// the siren.
pub fn tone_notes(cue: Cue, index: usize) -> Vec<(f32, u32)> {
    let (p, pattern) = tone_parts(index);
    let f = PITCHES[p];
    match (cue, pattern) {
        (Cue::Finished, 0) => vec![(f, 700)],
        (Cue::Finished, 1) => vec![(f, 150), (f * 1.26, 150), (f * 1.5, 380)],
        (Cue::Finished, 2) => vec![(f * 1.5, 150), (f * 1.26, 150), (f, 380)],
        (Cue::Finished, 3) => vec![(f, 120), (0.0, 30), (f, 120), (0.0, 30), (f * 1.5, 420)],
        (Cue::Finished, 4) => vec![(f, 160), (0.0, 80), (f, 160), (0.0, 80), (f, 160)],
        (Cue::Finished, _) => vec![(f, 900), (f * 2.0, 500)],
        (Cue::ToolChange, 0) => vec![(f, 220)],
        (Cue::ToolChange, 1) => vec![(f, 140), (0.0, 80), (f, 140)],
        (Cue::ToolChange, 2) => vec![(f, 110), (0.0, 70), (f, 110), (0.0, 70), (f, 110)],
        (Cue::ToolChange, 3) => (0..4)
            .flat_map(|k| [(if k % 2 == 0 { f } else { f * 1.5 }, 160), (0.0, 20)])
            .collect(),
        (Cue::ToolChange, 4) => vec![
            (f * 0.5, 50),
            (0.0, 90),
            (f * 0.5, 50),
            (0.0, 90),
            (f * 0.5, 50),
        ],
        (Cue::ToolChange, _) => (0..14)
            .map(|k| {
                let up = if k < 7 { k } else { 13 - k } as f32;
                (f * (1.0 + up * 0.06), 45)
            })
            .collect(),
    }
}

/// The WAV file (16 bits, mono, 22050 Hz) of some notes.
fn wav_of(notes: &[(f32, u32)]) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let mut samples: Vec<i16> = Vec::new();
    for &(freq, ms) in notes {
        let n = (RATE as u64 * ms as u64 / 1000) as usize;
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            // a soft start and a long fade, so that it does not click
            let env = (i as f32 / 200.0).min(1.0) * (1.0 - i as f32 / n as f32).powf(0.6);
            let v = if freq == 0.0 || freq > 10_000.0 {
                0.0
            } else {
                (t * freq * std::f32::consts::TAU).sin() * 0.55
                    + (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.12
            };
            samples.push((v * env * 22_000.0) as i16);
        }
    }
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// The WAV file of a sound of the catalogue.
pub fn catalogue_wav(cue: Cue, index: usize) -> Vec<u8> {
    wav_of(&tone_notes(cue, index))
}

/// The file to play for a setting: the operator's, or the tone written to the temporary folder. `None` when there is no sound.
pub fn file_for(setting: &SoundSetting, cue: Cue) -> Option<PathBuf> {
    match setting {
        SoundSetting::Off => None,
        SoundSetting::File(p) if p.is_file() => Some(p.clone()),
        SoundSetting::Tone(i) => {
            let tag = match cue {
                Cue::Finished => "finished",
                Cue::ToolChange => "tool",
            };
            let path = std::env::temp_dir().join(format!("eh3d-sound-{tag}-{i}.wav"));
            if !path.is_file() {
                std::fs::write(&path, catalogue_wav(cue, *i as usize)).ok()?;
            }
            Some(path)
        }
        // a file that is not there plays the tone instead of nothing
        SoundSetting::File(_) | SoundSetting::Default => {
            let name = match cue {
                Cue::Finished => "eh3d-sound-finished.wav",
                Cue::ToolChange => "eh3d-sound-tool-change.wav",
            };
            let path = std::env::temp_dir().join(name);
            if !path.is_file() {
                std::fs::write(&path, tone_wav(cue)).ok()?;
            }
            Some(path)
        }
    }
}

/// The commands that play a file on this system, in the order they are tried.
fn players(path: &Path) -> Vec<(String, Vec<String>)> {
    let p = path.display().to_string();
    if cfg!(windows) {
        let quoted = p.replace('\'', "''");
        vec![(
            "powershell".into(),
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-WindowStyle".into(),
                "Hidden".into(),
                "-Command".into(),
                format!("(New-Object Media.SoundPlayer '{quoted}').PlaySync()"),
            ],
        )]
    } else if cfg!(target_os = "macos") {
        vec![("afplay".into(), vec![p])]
    } else {
        vec![
            ("paplay".into(), vec![p.clone()]),
            ("aplay".into(), vec!["-q".into(), p.clone()]),
            ("play".into(), vec!["-q".into(), p]),
        ]
    }
}

/// Plays the sound of a cue if the setting asks for one. It returns at once.
pub fn play(setting: &SoundSetting, cue: Cue) {
    let Some(path) = file_for(setting, cue) else {
        return;
    };
    std::thread::spawn(move || {
        for (cmd, args) in players(&path) {
            let mut c = std::process::Command::new(&cmd);
            c.args(&args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                c.creation_flags(0x0800_0000); // no console window
            }
            if let Ok(mut child) = c.spawn() {
                let _ = child.wait();
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_comes_back_from_its_text() {
        for s in [
            SoundSetting::Off,
            SoundSetting::Default,
            SoundSetting::File(PathBuf::from("C:\\sounds\\ding.wav")),
        ] {
            assert_eq!(SoundSetting::parse(&s.to_text()), s);
        }
        assert_eq!(SoundSetting::parse(""), SoundSetting::Off);
        assert_eq!(SoundSetting::parse(" ON "), SoundSetting::Default);
        assert_eq!(
            SoundSetting::parse("\"a b.wav\""),
            SoundSetting::File(PathBuf::from("a b.wav"))
        );
        assert!(SoundSetting::Default.is_on() && !SoundSetting::Off.is_on());
    }

    #[test]
    fn the_catalogue_has_sixty_different_sounds_for_each_cue_from_high_to_low() {
        assert_eq!(PITCHES.len() * PATTERNS, TONES);
        // the pitches go down
        assert!(PITCHES.windows(2).all(|w| w[0] > w[1]));
        for cue in [Cue::Finished, Cue::ToolChange] {
            let mut seen = std::collections::HashSet::new();
            for i in 0..TONES {
                let notes = tone_notes(cue, i);
                assert!(!notes.is_empty() && notes.iter().any(|n| n.0 > 0.0), "{i}");
                let wav = catalogue_wav(cue, i);
                assert_eq!(&wav[0..4], b"RIFF");
                assert!(wav.len() > 8_000, "the sound {i} is too short");
                assert!(seen.insert(wav), "the sound {i} repeats another one");
            }
            assert_eq!(seen.len(), TONES);
            // the first sound of the catalogue is higher than the last
            let top = tone_notes(cue, 0).iter().map(|n| n.0).fold(0.0, f32::max);
            let bottom = tone_notes(cue, TONES - 1)
                .iter()
                .map(|n| n.0)
                .fold(f32::MAX, |a, b| if b > 0.0 { a.min(b) } else { a });
            assert!(top > bottom * 4.0, "{top} against {bottom}");
        }
        // the sounds for a tool are not the ones for the end of the job
        assert_ne!(tone_notes(Cue::Finished, 7), tone_notes(Cue::ToolChange, 7));
    }

    #[test]
    fn a_sound_of_the_catalogue_is_kept_in_the_machine_file_and_played_from_a_file() {
        let s = SoundSetting::Tone(42);
        assert_eq!(s.to_text(), "tone:42");
        assert_eq!(SoundSetting::parse("tone:42"), s);
        assert!(s.is_on());
        // a number that is not in the catalogue is a file name, as any other text
        assert_eq!(
            SoundSetting::parse("tone:99"),
            SoundSetting::File(PathBuf::from("tone:99"))
        );
        let c = crate::connection::parse(
            "connection = demo\nsound_finish = tone:7\nsound_tool_change = tone:59\n",
        )
        .unwrap();
        assert_eq!(
            (c.sound_finish, c.sound_tool_change),
            (SoundSetting::Tone(7), SoundSetting::Tone(59))
        );
        let path = file_for(&SoundSetting::Tone(3), Cue::ToolChange).unwrap();
        assert!(path.is_file());
    }

    #[test]
    fn the_tones_are_valid_wav_files_and_differ() {
        for cue in [Cue::Finished, Cue::ToolChange] {
            let w = tone_wav(cue);
            assert_eq!(&w[0..4], b"RIFF");
            assert_eq!(&w[8..16], b"WAVEfmt ");
            let data = u32::from_le_bytes([w[40], w[41], w[42], w[43]]) as usize;
            assert_eq!(w.len(), 44 + data);
            assert!(data > 20_000, "the tone is too short: {data} bytes");
            assert!(w[44..].iter().any(|&b| b != 0));
        }
        assert_ne!(tone_wav(Cue::Finished), tone_wav(Cue::ToolChange));
    }

    #[test]
    fn nothing_is_played_when_the_sound_is_off_and_the_tone_when_the_file_is_missing() {
        assert_eq!(file_for(&SoundSetting::Off, Cue::Finished), None);
        let f = file_for(
            &SoundSetting::File(PathBuf::from("/nowhere/none.wav")),
            Cue::ToolChange,
        )
        .unwrap();
        assert!(f.is_file());
        // the player of the system is built with the path in it, quoted
        let c = players(Path::new("a'b.wav"));
        assert!(!c.is_empty());
    }

    #[test]
    fn the_machine_file_sets_the_two_sounds() {
        let text = r"connection = demo
sound_finish = default
sound_tool_change = C:\sounds\ding.wav
";
        let c = crate::connection::parse(text).unwrap();
        assert_eq!(c.sound_finish, SoundSetting::Default);
        assert_eq!(
            c.sound_tool_change,
            SoundSetting::File(PathBuf::from(r"C:\sounds\ding.wav"))
        );
        // without the lines there is no sound
        let d = crate::connection::parse("connection = demo").unwrap();
        assert_eq!(
            (d.sound_finish, d.sound_tool_change),
            (SoundSetting::Off, SoundSetting::Off)
        );
    }
}
