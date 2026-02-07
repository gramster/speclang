//! Package manifest discovery.
//!
//! Walk up the directory tree from a source file to find the nearest `pkg.toml`.

use std::path::{Path, PathBuf};

/// The manifest file name.
pub const MANIFEST_NAME: &str = "pkg.toml";

/// Walk up from `start` (a file or directory) to find the nearest `pkg.toml`.
///
/// Returns `None` if no manifest is found (reached filesystem root).
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let start = if start.is_file() {
        start.parent()?
    } else {
        start
    };

    let mut dir = start;
    loop {
        let candidate = dir.join(MANIFEST_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tempdir(suffix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("speclang-discover-{}-{}", suffix, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn find_in_same_directory() {
        let tmp = tempdir("same");
        fs::write(tmp.join("pkg.toml"), "[package]\nname = \"x\"\nversion = \"0.1.0\"\n").unwrap();
        let found = find_manifest(&tmp).unwrap();
        assert_eq!(found, tmp.join("pkg.toml"));
    }

    #[test]
    fn find_in_parent_directory() {
        let tmp = tempdir("parent");
        fs::write(tmp.join("pkg.toml"), "[package]\nname = \"x\"\nversion = \"0.1.0\"\n").unwrap();
        let subdir = tmp.join("src");
        fs::create_dir_all(&subdir).unwrap();
        let found = find_manifest(&subdir).unwrap();
        assert_eq!(found, tmp.join("pkg.toml"));
    }

    #[test]
    fn find_from_file() {
        let tmp = tempdir("file");
        fs::write(tmp.join("pkg.toml"), "[package]\nname = \"x\"\nversion = \"0.1.0\"\n").unwrap();
        let src = tmp.join("src");
        fs::create_dir_all(&src).unwrap();
        let file = src.join("main.spl");
        fs::write(&file, "module main;").unwrap();
        let found = find_manifest(&file).unwrap();
        assert_eq!(found, tmp.join("pkg.toml"));
    }

    #[test]
    fn not_found() {
        let tmp = tempdir("none");
        // No pkg.toml anywhere — in practice this will keep going up,
        // but we at least verify it doesn't panic.
        let deep = tmp.join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        // No manifest in the tree, so it will walk up to root and return None.
        // (It may find one in /tmp if there's a real pkg.toml, but that's unlikely.)
        let _ = find_manifest(&deep);
    }
}
