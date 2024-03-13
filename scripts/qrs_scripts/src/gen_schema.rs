mod schema_cleaner;
mod schema_collector;

use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    visit::Visitor,
    JsonSchema,
};

use crate::{gen_schema::schema_cleaner::SchemaCleaner, utils::workspace_root};

use self::schema_collector::SchemaCollector;

trait ISchemaItem {
    fn gen(&self, collector: &mut SchemaCollector) -> anyhow::Result<()>;
}

struct SchemaItem<T>(std::marker::PhantomData<T>);

impl<T> Default for SchemaItem<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: JsonSchema> ISchemaItem for SchemaItem<T> {
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
        SchItem::<qrs_finance::products::general::ProductData>::default() as _,
        SchItem::<qrs_model::core::curve::ComponentCurve<f64>>::default() as _,
    ]
}

fn gen_schema(remove_defs: bool) -> anyhow::Result<SchemaCollector> {
    let mut collector = SchemaCollector {
        remove_defs,
        ..Default::default()
    };

    for item in get_schema_items() {
        item.gen(&mut collector)?;
    }
    Ok(collector)
}

pub fn write_schema() -> anyhow::Result<()> {
    let root_dir = {
        let mut dir = workspace_root()?;
        dir.push("schemas");
        dir
    };

    if root_dir.exists() {
        std::fs::remove_dir_all(&root_dir)?;
    }
    let mut decomposed = root_dir.clone();
    decomposed.push("_decomposed");
    let cases = [(false, root_dir), (true, decomposed)];
    for (remove_defs, dir) in cases {
        std::fs::create_dir_all(&dir)?;
        let collector = gen_schema(remove_defs)?;
        if remove_defs {
            for (name, sch) in &collector.definitions {
                let filepath = dir.join(format!("{name}.yaml"));
                let y = serde_yaml::to_string(&sch);
                assert!(y.is_ok(), "Failed to serialize schema: {:?}", name);
                std::fs::write(filepath, y.unwrap())?;
            }
        }
        for (name, sch) in &collector.roots {
            let filepath = dir.join(format!("{name}.yaml"));
            let y = serde_yaml::to_string(&sch);
            assert!(y.is_ok(), "Failed to serialize schema: {:?}", name);
            std::fs::write(filepath, y.unwrap())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_gen_schema() {
        let root_dir = {
            let mut dir = workspace_root().unwrap();
            dir.push("schemas");
            dir.push("_decomposed");
            dir
        };
        let expected = {
            let mut res: HashMap<String, serde_yaml::Value> = HashMap::default();
            for file in std::fs::read_dir(root_dir).unwrap() {
                let file = file.unwrap();
                let path = file.path();
                let name = path.file_stem().unwrap().to_str().unwrap();
                let s = std::fs::read_to_string(&path).unwrap();
                let y: Result<serde_yaml::Value, _> = serde_yaml::from_str(&s);
                assert!(y.is_ok(), "Failed to parse schema: {:?}", y);
                res.insert(name.to_string(), y.unwrap());
            }
            res
        };
        let generated = {
            let schemas = gen_schema(true);
            assert!(schemas.is_ok(), "Failed to generate schema");
            let schemas = schemas.unwrap();
            let mut res = HashMap::default();
            for (name, sch) in &schemas.definitions {
                let y = serde_yaml::to_value(sch);
                assert!(y.is_ok(), "Failed to serialize schema: {:?}", y);
                res.insert(name.clone(), y.unwrap());
            }
            for (name, sch) in &schemas.roots {
                let y = serde_yaml::to_value(sch);
                assert!(y.is_ok(), "Failed to serialize schema: {:?}", y);
                res.insert(name.clone(), y.unwrap());
            }
            res
        };
        assert_eq!(expected, generated);
    }
}
