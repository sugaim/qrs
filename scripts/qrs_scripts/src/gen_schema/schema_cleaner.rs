use schemars::{
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
    visit::{visit_schema_object, Visitor},
};

pub(super) struct SchemaCleaner;

impl Visitor for SchemaCleaner {
    fn visit_schema_object(&mut self, schema: &mut schemars::schema::SchemaObject) {
        visit_schema_object(self, schema);

        single_enum_to_const(schema);
        single_subsch_to_ref(schema);
        one_of_strings_to_enum(schema);
    }
}

fn single_enum_to_const(obj: &mut SchemaObject) {
    let value = if let Some(enum_values) = &obj.enum_values {
        if enum_values.len() != 1 {
            return;
        }
        enum_values[0].clone()
    } else {
        return;
    };
    obj.enum_values = None;
    obj.const_value = Some(value);
}

fn single_subsch_to_ref(obj: &mut SchemaObject) {
    let Some(ss) = &obj.subschemas else {
        return;
    };
    if ss.if_schema.is_some()
        || ss.then_schema.is_some()
        || ss.else_schema.is_some()
        || ss.not.is_some()
    {
        return;
    }
    let subsch = if let Some(all_of) = ss.all_of.as_ref() {
        if ss.any_of.is_some() || ss.one_of.is_some() || all_of.len() != 1 {
            return;
        }
        let Schema::Object(o) = &all_of[0] else {
            return;
        };
        o
    } else if let Some(any_of) = ss.any_of.as_ref() {
        if ss.one_of.is_some() || any_of.len() != 1 {
            return;
        }
        let Schema::Object(o) = &any_of[0] else {
            return;
        };
        o
    } else if let Some(one_of) = ss.one_of.as_ref() {
        if one_of.len() != 1 {
            return;
        }
        let Schema::Object(o) = &one_of[0] else {
            return;
        };
        o
    } else {
        return;
    };
    let md = obj.metadata.clone();
    *obj = subsch.clone();
    if md.is_some() {
        obj.metadata = md;
    }
}
fn one_of_strings_to_enum(obj: &mut SchemaObject) {
    let Some(ss) = &obj.subschemas else {
        return;
    };
    if ss.if_schema.is_some()
        || ss.then_schema.is_some()
        || ss.else_schema.is_some()
        || ss.not.is_some()
        || ss.any_of.is_some()
        || ss.all_of.is_some()
    {
        return;
    }
    let Some(one_of) = ss.one_of.as_ref() else {
        return;
    };
    let mut values = Vec::default();
    for s in one_of {
        let Schema::Object(o) = s else {
            return;
        };
        let Some(SingleOrVec::Single(inst)) = o.instance_type.as_ref() else {
            return;
        };
        if inst.as_ref() != &InstanceType::String {
            return;
        }
        let Some(c) = o.const_value.as_ref() else {
            return;
        };
        values.push(c.clone());
    }
    if values.is_empty() {
        return;
    }
    obj.subschemas = None;
    obj.enum_values = Some(values);
    obj.instance_type = Some(SingleOrVec::Single(InstanceType::String.into()));
}
