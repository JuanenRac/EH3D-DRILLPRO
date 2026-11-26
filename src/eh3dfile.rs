//! The `.eh3d` project of the suite: the drill layers are read from the job of the project (the branch `manufacturing/`) and the plan made on paper from one of them is kept in the
//! branch `drillpro/` only, with a copy of the layer and a reference to the content of the job, so that a job that changes makes the plans **out of date**. The reading, the writing and
//! the references are the ones every manufacturing program shares (`eh3d_project::manufacturing` of EH3D-COMMON). A plan is a text: no file for the machine is made and no machine is
//! needed. ROUTEPRO uses [`ProjectJob`] with its own branch.

use std::path::{Path, PathBuf};

use eh3d_project::appbranch::BranchKit;
use eh3d_project::manufacturing::{self as suite, JobFile};
use eh3d_project::suitetext::{fill, S};

/// The branch of this program.
pub const KIT: BranchKit<'static> = suite::kit(
    "drillpro",
    "drillpro.json",
    env!("CARGO_PKG_VERSION"),
);

/// The drill layers of the job of a project; an error when the job has none.
pub fn drill_layers(path: &Path) -> Result<Vec<JobFile>, String> {
    suite::layers_of(path, "drill")
}

/// The drill layer of the job called `layer` (its path inside the job or its file name).
pub fn choose<'a>(layers: &'a [JobFile], layer: &str) -> Result<&'a JobFile, String> {
    suite::choose(layers, "drill", layer)
}

/// The name of the plan of a layer inside `result/`: `drill-<name of the layer>.txt`.
pub fn result_name(layer: &JobFile) -> String {
    suite::result_name("drill", layer, "txt")
}

/// Keeps the plan of one layer in the branch of this program. The plans of the other layers stay while the job is the one they were made from. Returns what a person should be told.
pub fn save_plan(path: &Path, layer: &JobFile, lines: &[String]) -> Result<Vec<String>, String> {
    suite::keep_result(&KIT, path, layer, &result_name(layer), plan_bytes(lines))
}

/// The bytes of a plan as it is kept: one line of the report on each line.
pub fn plan_bytes(lines: &[String]) -> Vec<u8> {
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

/// The layer of the job of a project that a window has open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectJob {
    /// The project.
    pub path: PathBuf,
    /// The layer that is open: the first of its kind in the job.
    pub layer: JobFile,
    /// How many layers of that kind the job has.
    pub layers: usize,
    /// Whether the job changed since the plans that are kept were made.
    pub out_of_date: bool,
}

impl ProjectJob {
    /// Reads the first layer of a kind from the job, with the state of the branch of `kit`.
    pub fn open(kit: &BranchKit<'_>, path: &Path, kind: &str) -> Result<ProjectJob, String> {
        let mut layers = suite::layers_of(path, kind)?;
        let out_of_date = suite::state(kit, path)?.out_of_date;
        let count = layers.len();
        Ok(ProjectJob {
            path: path.to_path_buf(),
            layer: layers.swap_remove(0),
            layers: count,
            out_of_date,
        })
    }

    /// What the window says once the layer is open, in the language of `code`.
    pub fn notice(&self, code: &str) -> String {
        if self.out_of_date {
            S::JobStale.tr(code).to_string()
        } else {
            fill(S::JobOpened.tr(code), &[&self.layers.to_string()])
        }
    }

    /// What the window says once the plan called `name` is kept.
    pub fn saved_notice(name: &str, code: &str) -> String {
        fill(S::JobSaved.tr(code), &[name])
    }
}
