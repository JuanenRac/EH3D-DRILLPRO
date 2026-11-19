//! DRILLPRO plan: works out, on paper, what a job would do and prints it. No machine, no connection and no window are needed, and nothing is sent anywhere.
//!
//! Usage: `drillpro-plan FILE.NCD [--data DIR] [--lang en|es|it|de|fr|pt|zh|ja] [--sort none|x|y|special] [--from N] [--to N] [--skip-tool T]... [--skip-duplicates] [--per-tool] [--check]`
//!
//! `DIR` is the folder of `DRILLPRO.DCF` and `DRILLPRO.DTD` (the folder of the program by default). The lines are the same for the same files: the holes, the tools, the steps of the
//! plan, how far each axis goes, the time, the checks of the work and the verdict of the dry run against the course of the machine file. `--sort` orders the holes of each tool,
//! `--from` and `--to` take the vectors of the file from one to the other (counted from 1), `--skip-tool` switches a tool off (it may be given several times), `--skip-duplicates` leaves
//! out a hole that repeats the place of an earlier hole of the same tool, `--per-tool` adds a line for each tool (its holes and the travel of the drill) and `--check` ends with the
//! code 3 when the plan leaves the course of the machine, so that a script can tell without reading the lines.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use eh3d_drillpro::app::App;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::job::{JobOptions, Sorting};

const USAGE: &str = "usage: drillpro-plan FILE.NCD [--data DIR] [--lang CODE] [--sort none|x|y|special] [--from N] [--to N] [--skip-tool T] [--skip-duplicates] [--per-tool] [--check]";

/// Takes `name VALUE` out of the arguments (the first one) and gives the value.
fn take_value(args: &mut Vec<String>, name: &str) -> Result<Option<String>, String> {
    match args.iter().position(|a| a == name) {
        Some(i) => {
            let v = args
                .get(i + 1)
                .ok_or_else(|| format!("{name} needs a value"))?
                .clone();
            args.drain(i..=i + 1);
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

/// Takes a flag (an option with no value) out of the arguments and says whether it was there.
fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|a| a == name) {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

/// A whole number for an option, or the reason it is not one.
fn number<T: std::str::FromStr>(name: &str, text: &str) -> Result<T, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("{name} '{text}' is not a whole number in the range of the option"))
}

/// The job options and the sorting that the arguments ask for.
fn job_asked(args: &mut Vec<String>) -> Result<(JobOptions, Option<Sorting>), String> {
    let sorting = match take_value(args, "--sort")? {
        Some(name) => Some(
            Sorting::parse(&name)
                .ok_or_else(|| format!("--sort '{name}' is not none, x, y or special"))?,
        ),
        None => None,
    };
    let mut options = JobOptions::default();
    if let Some(v) = take_value(args, "--from")? {
        options.from = number("--from", &v)?;
    }
    if let Some(v) = take_value(args, "--to")? {
        options.to = number("--to", &v)?;
    }
    while let Some(v) = take_value(args, "--skip-tool")? {
        options.skip_tools.push(number("--skip-tool", &v)?);
    }
    options.skip_duplicates = take_flag(args, "--skip-duplicates");
    Ok((options, sorting))
}

fn run(mut args: Vec<String>) -> Result<bool, String> {
    let lang = match take_value(&mut args, "--lang")? {
        Some(c) => Lang::from_code(&c)
            .ok_or_else(|| format!("--lang '{c}' is not a language of the program"))?,
        None => Lang::En,
    };
    let data = take_value(&mut args, "--data")?.map_or_else(|| PathBuf::from("."), PathBuf::from);
    let (options, sorting) = job_asked(&mut args)?;
    let per_tool = take_flag(&mut args, "--per-tool");
    let check = take_flag(&mut args, "--check");
    if let Some(o) = args.iter().find(|a| a.starts_with("--")) {
        return Err(format!("{o}: unknown option"));
    }
    if args.len() != 1 {
        return Err(USAGE.into());
    }
    let mut app = App::new(lang, &data);
    if let Some(s) = sorting {
        app.set_sorting(s);
    }
    app.set_job_options(options);
    app.load_file(Path::new(&args[0]));
    let (lines, fits) = app.plan_report_with(per_tool)?;
    for line in lines {
        println!("{line}");
    }
    Ok(fits || !check)
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(3),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
