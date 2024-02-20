use std::{collections::HashMap, path::Path, str::FromStr};

use log::info;
use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    JsonSchema,
};

use crate::utils::workspace_root;

trait ISchemaItem {
    fn gen(&self, dirpath: &Path) -> anyhow::Result<()>;
    fn check(&self, dirpath: &Path) -> anyhow::Result<()>;
}

struct SchemaItem<T>(std::marker::PhantomData<T>);

impl<T> Default for SchemaItem<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: JsonSchema> ISchemaItem for SchemaItem<T> {
    fn gen(&self, dirpath: &Path) -> anyhow::Result<()> {
        gen_schema_single::<T>(dirpath)
    }
    fn check(&self, dirpath: &Path) -> anyhow::Result<()> {
        check_same_schema::<T>(dirpath);
        Ok(())
    }
}

fn get_schema_items() -> HashMap<&'static str, Vec<Box<dyn ISchemaItem>>> {
    let mut map: HashMap<_, Vec<Box<dyn ISchemaItem>>> = HashMap::new();
    map.insert(
        "qrs_core/chrono",
        vec![
            Box::<SchemaItem<qrs_core::chrono::Calendar>>::default() as _,
            Box::<SchemaItem<qrs_core::chrono::CalendarSymbol>>::default() as _,
            Box::<SchemaItem<qrs_core::chrono::GenericDateTime<chrono::FixedOffset>>>::default()
                as _,
            Box::<SchemaItem<qrs_core::chrono::GenericDateTime<chrono::Utc>>>::default() as _,
            Box::<SchemaItem<qrs_core::chrono::GenericDateTime<chrono_tz::Tz>>>::default() as _,
            Box::<SchemaItem<qrs_core::chrono::DateTime>>::default() as _,
            Box::<SchemaItem<qrs_core::chrono::TimeZone>>::default() as _,
        ],
    );
    map.insert(
        "qrs_core/func1d",
        vec![Box::<SchemaItem<qrs_core::func1d::SemiContinuity>>::default() as _],
    );
    map
}

pub fn gen_schema() -> anyhow::Result<()> {
    let root_dir = {
        let mut dir = workspace_root()?;
        dir.push("schemas");
        dir
    };

    for (subdir, items) in get_schema_items() {
        let mut dir = root_dir.clone();
        dir.push(subdir);
        std::fs::create_dir_all(&dir)?;
        for item in items {
            item.gen(&dir)?;
        }
    }
    Ok(())
}

fn json_schema_setting() -> SchemaGenerator {
    SchemaSettings::draft07().into()
}

fn gen_schema_single<T: JsonSchema>(dirpath: &Path) -> Result<(), anyhow::Error> {
    gen_json_schema_single::<T>(dirpath)?;
    Ok(())
}

fn gen_json_schema_single<T: JsonSchema>(dirpath: &Path) -> Result<(), anyhow::Error> {
    let mut gen = json_schema_setting();
    let schema = gen.root_schema_for::<T>();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();
    let filepath = dirpath.join(format!("{}.json", T::schema_name()));
    info!("Writing schema to {}", filepath.display());
    std::fs::write(filepath, schema_str).map_err(anyhow::Error::from)
}

// check that the stored schema is the same as the latest schema
fn check_same_schema<T: JsonSchema>(dirpath: &Path) {
    let stored = {
        let filepath = dirpath.join(format!("{}.json", T::schema_name()));
        let s = std::fs::read_to_string(filepath);
        assert!(s.is_ok(), "Failed to read file: {:?}", s);
        let s = s.unwrap();
        let j = serde_json::Value::from_str(&s);
        assert!(j.is_ok(), "Failed to parse JSON: {:?}", j);
        j.unwrap()
    };
    let latest = {
        let mut gen = json_schema_setting();
        let schema = gen.root_schema_for::<T>();
        let j = serde_json::to_value(&schema);
        assert!(j.is_ok(), "Failed to serialize schema: {:?}", j);
        j.unwrap()
    };
    assert_eq!(stored, latest);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_schema() {
        let root_dir = {
            let mut dir = workspace_root().unwrap();
            dir.push("schemas");
            dir
        };

        for (subdir, items) in get_schema_items() {
            let mut dir = root_dir.clone();
            dir.push(subdir);
            for item in items {
                item.check(&dir).unwrap();
            }
        }
    }
}
