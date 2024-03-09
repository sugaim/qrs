use std::collections::HashMap;

use schemars::{
    schema::{RootSchema, Schema, SchemaObject},
    visit::{visit_root_schema, visit_schema_object, Visitor},
};

#[derive(Default)]
pub struct SchemaCollector {
    pub roots: HashMap<String, RootSchema>,
    pub definitions: HashMap<String, Schema>,
}

impl Visitor for SchemaCollector {
    fn visit_root_schema(&mut self, root: &mut RootSchema) {
        visit_root_schema(self, root);
        while let Some((k, mut v)) = root.definitions.pop_last() {
            if let Schema::Object(ref mut o) = v {
                o.metadata().title = Some(k.clone());
            }
            self.definitions.insert(k, v);
        }
        let name = root
            .schema
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .expect("Schema must have a title");
        self.roots.insert(name, root.clone());
    }
    fn visit_schema_object(&mut self, schema: &mut SchemaObject) {
        visit_schema_object(self, schema);
        let mut new_ref = None;
        if let Some(reference) = &schema.reference {
            let name = reference
                .split('/')
                .last()
                .expect("Reference must have a name");
            new_ref = Some(format!("./{}.yaml", name));
        }
        schema.reference = new_ref;
    }
}
