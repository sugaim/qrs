#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod gen_schema;
pub mod utils;

#[cfg_attr(coverage_nightly, coverage(off))]
fn main() -> anyhow::Result<()> {
    env_logger::init();
    gen_schema::gen_schema()?;
    Ok(())
}
