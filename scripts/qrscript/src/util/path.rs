use std::path::PathBuf;

pub fn repo_root() -> PathBuf {
    let git_root = git2::Repository::discover(".").expect("Failed to find git root");
    let git_root = git_root.workdir().expect("Failed to find git root");
    git_root.to_path_buf()
}
