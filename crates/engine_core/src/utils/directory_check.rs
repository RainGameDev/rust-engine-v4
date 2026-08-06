use std::path::Path;

use anyhow::Result;

/// Recursively load all files of type `extension` in a directory
pub fn load_directory(path: &Path, extension: &str) -> Result<Vec<String>> {
    let mut files: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            files.append(&mut load_directory(&path, extension).unwrap());
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            files.push(path.to_str().unwrap().into());
        }
    }

    Ok(files)
}

pub fn normalize_asset_path(path: &str) -> String {
    let normalized = path.replace('\\', "/"); // handle Windows-style paths too

    match normalized.rfind("res/") {
        Some(index) => normalized[index + "res/".len()..].to_string(),
        None => normalized,
    }
}
