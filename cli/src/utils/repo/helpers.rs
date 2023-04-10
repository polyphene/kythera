use anyhow::{Context, format_err, Result};
use path_clean::PathClean;
use std::env;
use std::fmt::format;
use std::path::{Path, PathBuf};

/// Check that a path leads to a location that is part of the project
pub fn to_relative_path_to_project_root(tested_path_os_str: &str) -> Result<String> {
    // Parse path os string and canonicalize it
    let path = absolute_path(PathBuf::from(tested_path_os_str))?;
    // Try to strip root directory path from it. If it fails, then return an error as the location
    // is not part of the project, otherwise return an equivalent path string relative to the
    // project root directory.
    let root_path = env::current_dir()?;
    let printable_path = path.to_str().unwrap_or("");
    let stripped_path = path.strip_prefix(&root_path).with_context(|| {
        format!(
            "file path outside the project directory: {0}",
            printable_path.to_string()
        )
    })?;
    let stripped_path_str = stripped_path
        .to_str()
        .ok_or(format_err!("invalid unicode characters in the file path"))?;
    Ok(stripped_path_str.to_string())
}

/// Get an absolute path string from a possibly relative one, even if the path does not exist.
/// Reference: https://stackoverflow.com/a/54817755 (with light modifications).
pub fn absolute_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }
    .clean();
    Ok(absolute_path)
}
