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
}

impl SoundSetting {
    /// The text of the machine file: `off`, `default` or the path.
    pub fn to_text(&self) -> String {
        match self {
            SoundSetting::Off => "off".into(),
            SoundSetting::Default => "default".into(),
            SoundSetting::File(p) => p.display().to_string(),
        }
    }

    /// The other way round; an empty text is `off`.
    pub fn parse(text: &str) -> SoundSetting {
        let t = text.trim().trim_matches('"');
        match t.to_ascii_lowercase().as_str() {
            "" | "off" | "no" | "false" | "0" => SoundSetting::Off,
            "default" | "on" | "yes" | "true" | "1" => SoundSetting::Default,
            _ => SoundSetting::File(PathBuf::from(t)),
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
    const RATE: u32 = 22_050;
    let mut samples: Vec<i16> = Vec::new();
    for &(freq, ms) in notes(cue) {
        let n = (RATE as u64 * ms as u64 / 1000) as usize;
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            // a soft start and a long fade, so that it does not click
            let env = (i as f32 / 200.0).min(1.0) * (1.0 - i as f32 / n as f32).powf(0.6);
            let v = if freq == 0.0 {
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

/// The file to play for a setting: the operator's, or the tone written to the temporary folder. `None` when there is no sound.
pub fn file_for(setting: &SoundSetting, cue: Cue) -> Option<PathBuf> {
    match setting {
        SoundSetting::Off => None,
        SoundSetting::File(p) if p.is_file() => Some(p.clone()),
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
