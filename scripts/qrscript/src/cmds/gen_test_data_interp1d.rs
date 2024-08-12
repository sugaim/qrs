use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure};
use clap::ValueEnum;
use dialoguer::Confirm;
use qmath::{interp1d::Lerp1d, num::DerXX1d};

use crate::util::path::repo_root;

use super::Cmd;

// -----------------------------------------------------------------------------
// Format
// -----------------------------------------------------------------------------
#[derive(ValueEnum, Debug, Clone)]
#[clap(rename_all = "snake_case")]
pub enum Format {
    Csv,
}

impl Format {
    fn as_generator(&self) -> Box<dyn Generator> {
        match self {
            Format::Csv => Box::new(CsvGenerator),
        }
    }
}

// -----------------------------------------------------------------------------
// Args
// -----------------------------------------------------------------------------
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Format of generated test data
    #[clap(short = 'f', long = "format")]
    pub format: Format,

    /// Input directory of test data
    #[clap(short = 'i', long = "indir")]
    pub indir: Option<PathBuf>,

    /// Output directory of generated test data
    #[clap(short = 'o', long = "outdir")]
    pub outdir: Option<PathBuf>,

    /// Force clean output directory
    #[clap(long = "auto-clean")]
    pub auto_clean: Option<bool>,
}

impl Cmd for Args {
    fn run(&self) -> anyhow::Result<()> {
        let indir = self.indir.as_ref().cloned().unwrap_or_else(|| {
            let subdirs = ["libs", "core", "qmath", "testdata", "interp1d", "in"];
            repo_root().join(subdirs.join("/"))
        });
        let outdir = self.outdir.as_ref().cloned().unwrap_or_else(|| {
            let subdirs = ["libs", "core", "qmath", "testdata", "interp1d", "out"];
            repo_root().join(subdirs.join("/"))
        });
        if !indir.exists() {
            bail!("Input directory does not exist at {:?}", indir);
        }
        if outdir.exists() {
            if !self.auto_clean.unwrap_or(false) {
                let confirmed = Confirm::new()
                    .default(false)
                    .show_default(true)
                    .with_prompt(format!(
                        "Output directory already exists at {:?}. \nDo you want to clean it?",
                        outdir
                    ))
                    .interact()?;
                ensure!(confirmed, "Operation cancelled.");
            }
            log::info!("Cleaning output directory at {:?}", outdir);
            std::fs::remove_dir_all(&outdir)?;
        }
        std::fs::create_dir_all(&outdir)?;

        let generator = self.format.as_generator();
        for entry in std::fs::read_dir(&indir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }

            log::info!("Generating test data for {:?}", path);
            let data: InData = serde_json::from_reader(std::fs::File::open(&path)?)?;
            let case_name = path.file_stem().unwrap();
            generator.gen(case_name, data, &outdir)?;
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Interp
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Interp {
    Lerp(Lerp1d<f64, f64>),
}

impl Interp {
    fn into_dyn(
        self,
    ) -> Box<dyn DerXX1d<f64, Output = f64, DerX = f64, DerXX = f64, Error = anyhow::Error>> {
        match self {
            Interp::Lerp(interp) => Box::new(interp),
        }
    }
}

// -----------------------------------------------------------------------------
// InData
// -----------------------------------------------------------------------------
#[derive(serde::Deserialize)]
struct InData {
    start: f64,
    end: f64,
    step: f64,
    interp: Interp,
}

// -----------------------------------------------------------------------------
// Generator
// -----------------------------------------------------------------------------
trait Generator {
    fn gen(&self, name: &OsStr, data: InData, outdir: &Path) -> anyhow::Result<()>;
}

struct CsvGenerator;

impl Generator for CsvGenerator {
    fn gen(&self, name: &OsStr, data: InData, outdir: &Path) -> anyhow::Result<()> {
        let f = data.interp.into_dyn();

        let mut x = data.start;
        let mut lines = vec!["x,y,dy,d2y".to_string()];
        while x <= data.end {
            let (y, dy, d2y) = f.der_0_x_xx(&x)?;
            lines.push(format!("{x},{y},{dy},{d2y}"));
            x += data.step;
        }
        let contents = lines.join("\n");
        let mut path = outdir.join(name);
        path.set_extension("csv");

        std::fs::write(path, contents)?;

        Ok(())
    }
}
