use std::path::Path;

use log::info;
use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    JsonSchema,
};

use crate::utils::workspace_root;

pub fn gen_schema() -> anyhow::Result<()> {
    let root_dir = {
        let mut dir = workspace_root()?;
        dir.push("schemas");
        dir
    };

    // qcore
    {
        let mut dir = root_dir.clone();
        dir.push("qcore");
        std::fs::create_dir_all(&dir)?;
        gen_schema_single::<qcore::chrono::Calendar>(&dir)?;
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

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

    #[test]
    fn test_gen_schema() {
        let root_dir = {
            let mut dir = workspace_root().unwrap();
            dir.push("schemas");
            dir
        };

        // qcore
        {
            let mut dir = root_dir.clone();
            dir.push("qcore");
            check_same_schema::<qcore::chrono::Calendar>(&dir);
        }
    }
}
