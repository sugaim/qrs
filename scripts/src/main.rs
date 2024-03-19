pub mod gen_schema;
pub mod utils;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    gen_schema::write_schema()?;
    Ok(())
}
