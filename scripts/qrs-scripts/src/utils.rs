use std::path::PathBuf;

pub fn workspace_root() -> anyhow::Result<PathBuf> {
    let this_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let maybe_root = this_crate.parent().unwrap().parent().unwrap().to_path_buf();
    if !maybe_root.ends_with("qrs") {
        anyhow::bail!(
            "Expected to find a `qrs` directory in the workspace root, but found: {}",
            maybe_root.display()
        );
    }
    Ok(maybe_root)
}
