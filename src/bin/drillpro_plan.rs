//! DRILLPRO plan: works out, on paper, what a job would do and prints it. No machine, no connection and no window are needed, and nothing is sent anywhere.
//!
//! Usage: `drillpro-plan FILE.NCD [--data DIR] [--lang en|es|it|de|fr|pt|zh|ja] [--sort none|x|y|special] [--from N] [--to N] [--skip-tool T]... [--skip-duplicates] [--per-tool] [--check]`
//!
//! `DIR` is the folder of `DRILLPRO.DCF` and `DRILLPRO.DTD` (the folder of the program by default). The lines are the same for the same files: the holes, the tools, the steps of the
//! plan, how far each axis goes, the time, the checks of the work and the verdict of the dry run against the course of the machine file. `--sort` orders the holes of each tool,
//! `--from` and `--to` take the vectors of the file from one to the other (counted from 1), `--skip-tool` switches a tool off (it may be given several times), `--skip-duplicates` leaves
//! out a hole that repeats the place of an earlier hole of the same tool, `--per-tool` adds a line for each tool (its holes and the travel of the drill) and `--check` ends with the
//! code 3 when the plan leaves the course of the machine, so that a script can tell without reading the lines.
//!
//! The `.eh3d` project of the suite: `drillpro-plan project info FILE.eh3d` shows the layers of the job and the plans kept in the project (it ends with the code 2 when they are out
//! of date), and `drillpro-plan project plan FILE.eh3d [--layer NAME] [options]` plans each drill layer of the job (or the one named) and keeps each plan in the branch
//! `drillpro/` only. With `--check`, a plan that leaves the course of the machine keeps nothing.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use eh3d_drillpro::app::App;
use eh3d_drillpro::eh3dfile;
use eh3d_drillpro::i18n::Lang;
use eh3d_drillpro::job::{JobOptions, Sorting};
use eh3d_project::manufacturing as suite;

const USAGE: &str = "usage: drillpro-plan FILE.NCD [--data DIR] [--lang CODE] [--sort none|x|y|special] [--from N] [--to N] [--skip-tool T] [--skip-duplicates] [--per-tool] [--check]\n       drillpro-plan project info FILE.eh3d\n       drillpro-plan project plan FILE.eh3d [--layer NAME] [the options above]";

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

/// `project info FILE` and `project plan FILE`: the code the program ends with (0 clean, 2 out of date, 3 outside the course with `--check`).
fn project(
    args: &[String],
    layer: Option<&str>,
    per_tool: bool,
    check: bool,
    new_app: &dyn Fn() -> App,
) -> Result<u8, String> {
    let (what, file) = match args {
        [what, file] => (what.as_str(), Path::new(file)),
        _ => return Err(USAGE.into()),
    };
    match what {
        "info" if layer.is_none() => {
            let (lines, state) = suite::describe(&eh3dfile::KIT, file)?;
            for line in lines {
                println!("{line}");
            }
            if state.out_of_date {
                eprintln!("{}", suite::out_of_date_message(&eh3dfile::KIT));
                return Ok(2);
            }
            Ok(0)
        }
        "plan" => {
            // the mark is taken before the layers are read: what another program saves after it is not written over
            let seen = eh3dfile::mark_of(&eh3dfile::KIT, file)?;
            let layers = eh3dfile::drill_layers(file)?;
            let chosen = match layer {
                Some(name) => vec![eh3dfile::choose(&layers, name)?.clone()],
                None => layers.clone(),
            };
            // every plan is made before one is kept, so a refusal leaves the project as it was
            let mut made = Vec::new();
            let mut fits_all = true;
            for layer in chosen {
                let mut app = new_app();
                app.load_layer(file, &layer)?;
                let (lines, fits) = app.plan_report_with(per_tool)?;
                for line in &lines {
                    println!("{line}");
                }
                fits_all &= fits;
                made.push((layer, lines));
            }
            if check && !fits_all {
                return Ok(3);
            }
            for (layer, lines) in made {
                for notice in eh3dfile::save_plan_since(file, &seen, &layer, &lines)? {
                    eprintln!("note: {notice}");
                }
                println!(
                    "{} kept in the branch {} of {}",
                    eh3dfile::result_name(&layers, &layer),
                    eh3dfile::KIT.branch,
                    file.display()
                );
            }
            Ok(0)
        }
        _ => Err(USAGE.into()),
    }
}

fn run(mut args: Vec<String>) -> Result<u8, String> {
    let lang = match take_value(&mut args, "--lang")? {
        Some(c) => Lang::from_code(&c)
            .ok_or_else(|| format!("--lang '{c}' is not a language of the program"))?,
        None => Lang::En,
    };
    let data = take_value(&mut args, "--data")?.map_or_else(|| PathBuf::from("."), PathBuf::from);
    let (options, sorting) = job_asked(&mut args)?;
    let per_tool = take_flag(&mut args, "--per-tool");
    let check = take_flag(&mut args, "--check");
    let layer = take_value(&mut args, "--layer")?;
    if let Some(o) = args.iter().find(|a| a.starts_with("--")) {
        return Err(format!("{o}: unknown option"));
    }
    let new_app = || {
        let mut app = App::new(lang, &data);
        if let Some(s) = sorting {
            app.set_sorting(s);
        }
        app.set_job_options(options.clone());
        app
    };
    if args.len() > 1 && args[0] == "project" {
        return project(&args[1..], layer.as_deref(), per_tool, check, &new_app);
    }
    if args.len() != 1 || layer.is_some() {
        return Err(USAGE.into());
    }
    let mut app = new_app();
    app.load_file(Path::new(&args[0]));
    let (lines, fits) = app.plan_report_with(per_tool)?;
    for line in lines {
        println!("{line}");
    }
    Ok(if fits || !check { 0 } else { 3 })
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
