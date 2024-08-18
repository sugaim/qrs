mod schema_cleaner;
mod schema_collector;

use std::path::PathBuf;

use anyhow::ensure;
use dialoguer::Confirm;
use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    visit::Visitor,
    JsonSchema,
};

use schema_cleaner::SchemaCleaner;

use crate::util::path::repo_root;

use self::schema_collector::SchemaCollector;

use super::Cmd;

// -----------------------------------------------------------------------------
// Args
// -----------------------------------------------------------------------------
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Output directory of generated test data
    #[clap(short = 'o', long = "outdir")]
    pub outdir: Option<PathBuf>,

    /// Force clean output directory
    #[clap(long = "auto-clean")]
    pub auto_clean: Option<bool>,
}

impl Cmd for Args {
    fn run(&self) -> anyhow::Result<()> {
        let outdir = self.outdir.as_ref().cloned().unwrap_or_else(|| {
            let subdirs = ["docs", "schemas", "libs"];
            repo_root().join(subdirs.join("/"))
        });

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

        let mut decomposed = outdir.clone();
        decomposed.push("_decomposed");
        let cases = [(false, outdir), (true, decomposed)];
        for (remove_defs, dir) in cases {
            std::fs::create_dir_all(&dir)?;
            let collector = gen_schema(remove_defs)?;
            if remove_defs {
                for (name, sch) in &collector.definitions {
                    let j = serde_json::to_string_pretty(&sch);
                    assert!(j.is_ok(), "Failed to serialize schema: {:?}", name);
                    std::fs::write(dir.join(format!("{name}.json")), j.unwrap())?;
                }
            }
            for (name, sch) in &collector.roots {
                let j = serde_json::to_string_pretty(&sch);
                assert!(j.is_ok(), "Failed to serialize schema: {:?}", name);
                std::fs::write(dir.join(format!("{name}.json")), j.unwrap())?;
            }
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// ISchemaItem
// -----------------------------------------------------------------------------
trait ISchemaItem {
    fn name(&self) -> &'static str;
    fn gen(&self, collector: &mut SchemaCollector) -> anyhow::Result<()>;
}

struct SchemaItem<T>(std::marker::PhantomData<T>);

impl<T> Default for SchemaItem<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: 'static + JsonSchema> SchemaItem<T> {
    fn create() -> Box<dyn ISchemaItem> {
        Box::<SchemaItem<T>>::default() as _
    }
}

impl<T: JsonSchema> ISchemaItem for SchemaItem<T> {
    fn name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    fn gen(&self, collector: &mut SchemaCollector) -> anyhow::Result<()> {
        let mut gen: SchemaGenerator = SchemaSettings::draft07()
            .with(|s| {
                s.option_add_null_type = false;
            })
            .into();
        let mut schema = gen.root_schema_for::<T>();
        SchemaCleaner.visit_root_schema(&mut schema);
        collector.visit_root_schema(&mut schema);
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// get_schema_items
// -----------------------------------------------------------------------------
fn get_schema_items() -> Vec<Box<dyn ISchemaItem>> {
    vec![
        SchemaItem::<qchrono::calendar::Calendar>::create(),
        SchemaItem::<qfincore::Ccy>::create(),
        SchemaItem::<qmodel::curve::composite::CompositeReq<qmodel::curve::adjust::Adj<f64>>>::create(),
        SchemaItem::<qmodel::curve::atom::Atom<f64>>::create(),
        SchemaItem::<qfincore::daycount::DayCountSym>::create(),
        SchemaItem::<qfincore::fxmkt::FxSpotMktReq>::create(),
    ]
}

fn gen_schema(remove_defs: bool) -> anyhow::Result<SchemaCollector> {
    let mut collector = SchemaCollector {
        remove_defs,
        ..Default::default()
    };

    for item in get_schema_items() {
        log::info!("Generating schema for: {}", item.name());
        item.gen(&mut collector)?;
    }
    Ok(collector)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_gen_schema() {
        let expected = {
            let root_dir = {
                let subdirs = ["docs", "schemas", "libs"];
                repo_root().join(subdirs.join("/"))
            };
            let mut res: HashMap<String, serde_json::Value> = HashMap::default();
            for file in std::fs::read_dir(root_dir.clone()).unwrap() {
                let file = file.unwrap();
                if file.file_type().unwrap().is_dir() {
                    continue;
                }
                let path = file.path();
                let name = path.file_stem().unwrap().to_str().unwrap();
                let s = std::fs::read_to_string(&path).unwrap();
                let j: Result<serde_json::Value, _> = serde_json::from_str(&s);
                assert!(j.is_ok(), "Failed to parse schema: {:?}", j);
                res.insert(name.to_string(), j.unwrap());
            }
            for file in std::fs::read_dir(root_dir.join("_decomposed")).unwrap() {
                let file = file.unwrap();
                if file.file_type().unwrap().is_dir() {
                    continue;
                }
                let path = file.path();
                let name = path.file_stem().unwrap().to_str().unwrap();
                let s = std::fs::read_to_string(&path).unwrap();
                let j: Result<serde_json::Value, _> = serde_json::from_str(&s);
                assert!(j.is_ok(), "Failed to parse schema: {:?}", j);
                res.insert(name.to_string(), j.unwrap());
            }
            res
        };
        let generated = {
            let schemas = gen_schema(true);
            assert!(schemas.is_ok(), "Failed to generate schema");
            let schemas = schemas.unwrap();
            let mut res = HashMap::default();
            for (name, sch) in &schemas.definitions {
                let j = serde_json::to_value(sch);
                assert!(j.is_ok(), "Failed to serialize schema: {:?}", j);
                res.insert(name.clone(), j.unwrap());
            }
            for (name, sch) in &schemas.roots {
                let j = serde_json::to_value(sch);
                assert!(j.is_ok(), "Failed to serialize schema: {:?}", j);
                res.insert(name.clone(), j.unwrap());
            }
            res
        };
        assert!(expected == generated);
    }
}
