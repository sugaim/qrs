mod schema_cleaner;
mod schema_collector;

use std::{path::Path, str::FromStr};

use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    visit::Visitor,
    JsonSchema,
};

use crate::{gen_schema::schema_cleaner::SchemaCleaner, utils::workspace_root};

use self::schema_collector::SchemaCollector;

trait ISchemaItem {
    fn gen(&self, collector: &mut SchemaCollector) -> anyhow::Result<()>;
    fn check(&self, dirpath: &Path) -> anyhow::Result<()>;
}

struct SchemaItem<T>(std::marker::PhantomData<T>);

impl<T> Default for SchemaItem<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: JsonSchema> ISchemaItem for SchemaItem<T> {
    fn gen(&self, collector: &mut SchemaCollector) -> anyhow::Result<()> {
        gen_schema_single::<T>(collector)
    }
    fn check(&self, dirpath: &Path) -> anyhow::Result<()> {
        check_same_schema::<T>(dirpath);
        Ok(())
    }
}

fn get_schema_items() -> Vec<Box<dyn ISchemaItem>> {
    type SchItem<T> = Box<SchemaItem<T>>;
    vec![
        SchItem::<qrs_chrono::Calendar>::default() as _,
        SchItem::<qrs_chrono::CalendarSymbol>::default() as _,
        SchItem::<qrs_chrono::DateTime<chrono::FixedOffset>>::default() as _,
        SchItem::<qrs_chrono::DateTime<chrono::Utc>>::default() as _,
        SchItem::<qrs_chrono::DateTime<chrono_tz::Tz>>::default() as _,
        SchItem::<qrs_chrono::DateTime>::default() as _,
        SchItem::<qrs_chrono::Tz>::default() as _,
        SchItem::<qrs_chrono::Duration>::default() as _,
        SchItem::<qrs_model::core::curve::ComponentCurve<f64>>::default() as _,
    ]
}

pub fn gen_schema() -> anyhow::Result<()> {
    let root_dir = {
        let mut dir = workspace_root()?;
        dir.push("schemas");
        dir
    };

    if root_dir.exists() {
        std::fs::remove_dir_all(&root_dir)?;
    }
    std::fs::create_dir_all(&root_dir)?;
    let mut collector = SchemaCollector::default();
    for item in get_schema_items() {
        item.gen(&mut collector)?;
    }
    for (name, sch) in &collector.definitions {
        let filepath = root_dir.join(format!("{name}.yaml"));
        let y = serde_yaml::to_string(&sch);
        assert!(y.is_ok(), "Failed to serialize schema: {:?}", name);
        std::fs::write(filepath, y.unwrap())?;
    }
    for (name, sch) in &collector.roots {
        let filepath = root_dir.join(format!("{name}.yaml"));
        let y = serde_yaml::to_string(&sch);
        assert!(y.is_ok(), "Failed to serialize schema: {:?}", name);
        std::fs::write(filepath, y.unwrap())?;
    }
    Ok(())
}

fn json_schema_setting() -> SchemaGenerator {
    SchemaSettings::draft07().into()
}

fn gen_schema_single<T: JsonSchema>(collector: &mut SchemaCollector) -> Result<(), anyhow::Error> {
    gen_json_schema_single::<T>(collector)?;
    Ok(())
}

fn gen_json_schema_single<T: JsonSchema>(collector: &mut SchemaCollector) -> anyhow::Result<()> {
    let mut gen = json_schema_setting();
    let mut schema = gen.root_schema_for::<T>();
    SchemaCleaner.visit_root_schema(&mut schema);
    collector.visit_root_schema(&mut schema);
    Ok(())
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

        for item in get_schema_items() {
            item.check(&root_dir).unwrap();
        }
    }
}
