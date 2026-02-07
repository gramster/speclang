//! Dependency graph construction and resolution.

use crate::manifest::{Manifest, DependencyKind, Version, ManifestError};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use thiserror::Error;

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("dependency cycle detected: {0}")]
    Cycle(String),
    #[error("dependency '{name}' not found at path '{path}'")]
    NotFound { name: String, path: String },
    #[error("version conflict for '{name}': {required} required but {found} found")]
    VersionConflict {
        name: String,
        required: Version,
        found: Version,
    },
}

// -----------------------------------------------------------------------
// Resolved dependency
// -----------------------------------------------------------------------

/// A fully resolved dependency with its manifest and absolute path.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: Version,
    pub manifest_dir: PathBuf,
    pub manifest: Manifest,
}

// -----------------------------------------------------------------------
// Dependency graph
// -----------------------------------------------------------------------

/// A topologically ordered dependency graph.
#[derive(Debug)]
pub struct DepGraph {
    /// Root package.
    pub root: ResolvedDep,
    /// All resolved dependencies in topological order (dependencies before dependents).
    pub deps: Vec<ResolvedDep>,
}

impl DepGraph {
    /// Resolve the dependency graph starting from a manifest file.
    pub fn resolve(manifest_path: &Path) -> Result<Self, ResolveError> {
        let manifest_path = manifest_path
            .canonicalize()
            .map_err(|e| ManifestError::Io {
                path: manifest_path.display().to_string(),
                source: e,
            })?;
        let manifest_dir = manifest_path.parent().unwrap().to_path_buf();
        let manifest = Manifest::from_file(&manifest_path)?;

        let root = ResolvedDep {
            name: manifest.package.name.clone(),
            version: manifest.package.version.clone(),
            manifest_dir: manifest_dir.clone(),
            manifest,
        };

        // BFS to discover all path dependencies
        let mut resolved: BTreeMap<String, ResolvedDep> = BTreeMap::new();
        let mut queue: VecDeque<(String, PathBuf)> = VecDeque::new();
        let mut edges: Vec<(String, String)> = Vec::new();

        // Seed with root's deps
        for (dep_name, dep) in &root.manifest.dependencies {
            if let DependencyKind::Path { path, .. } = &dep.kind {
                let dep_dir = manifest_dir.join(path);
                queue.push_back((dep_name.clone(), dep_dir));
                edges.push((root.name.clone(), dep_name.clone()));
            }
        }

        while let Some((name, dep_dir)) = queue.pop_front() {
            if resolved.contains_key(&name) {
                continue;
            }

            let pkg_toml = dep_dir.join("pkg.toml");
            if !pkg_toml.exists() {
                return Err(ResolveError::NotFound {
                    name,
                    path: dep_dir.display().to_string(),
                });
            }

            let dep_manifest = Manifest::from_file(&pkg_toml)?;
            let dep_dir_canon = dep_dir
                .canonicalize()
                .map_err(|e| ManifestError::Io {
                    path: dep_dir.display().to_string(),
                    source: e,
                })?;

            let dep_resolved = ResolvedDep {
                name: name.clone(),
                version: dep_manifest.package.version.clone(),
                manifest_dir: dep_dir_canon.clone(),
                manifest: dep_manifest,
            };

            // Check version compatibility if parent specified a version requirement
            if let Some(parent_dep) = root.manifest.dependencies.get(&name) {
                if let DependencyKind::Path {
                    version: Some(req), ..
                } = &parent_dep.kind
                {
                    if !req.is_compatible(&dep_resolved.version) {
                        return Err(ResolveError::VersionConflict {
                            name,
                            required: req.clone(),
                            found: dep_resolved.version,
                        });
                    }
                }
            }

            // Enqueue transitive path deps
            for (sub_name, sub_dep) in &dep_resolved.manifest.dependencies {
                if let DependencyKind::Path { path, .. } = &sub_dep.kind {
                    let sub_dir = dep_dir_canon.join(path);
                    edges.push((name.clone(), sub_name.clone()));
                    queue.push_back((sub_name.clone(), sub_dir));
                }
            }

            resolved.insert(name, dep_resolved);
        }

        // Topological sort (Kahn's algorithm)
        let all_names: BTreeSet<String> = resolved.keys().cloned().collect();
        let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
        let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for name in &all_names {
            in_degree.entry(name.clone()).or_insert(0);
            adjacency.entry(name.clone()).or_default();
        }
        for (from, to) in &edges {
            // Edge means "from depends on to", so in topo sort: to → from
            if all_names.contains(to) && all_names.contains(from) {
                adjacency.entry(to.clone()).or_default().push(from.clone());
                *in_degree.entry(from.clone()).or_insert(0) += 1;
            }
        }

        let mut topo_queue: VecDeque<String> = VecDeque::new();
        for (name, &deg) in &in_degree {
            if deg == 0 {
                topo_queue.push_back(name.clone());
            }
        }

        let mut sorted = Vec::new();
        while let Some(name) = topo_queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&name) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        topo_queue.push_back(neighbor.clone());
                    }
                }
            }
            sorted.push(name);
        }

        if sorted.len() != all_names.len() {
            // Cycle detected — find a node still with in_degree > 0
            let cycle_member = in_degree
                .iter()
                .find(|(_, d)| **d > 0)
                .map(|(n, _)| n.clone())
                .unwrap_or_default();
            return Err(ResolveError::Cycle(cycle_member));
        }

        // Build deps vec in topological order (leaves first)
        let deps: Vec<ResolvedDep> = sorted
            .into_iter()
            .filter_map(|name| resolved.remove(&name))
            .collect();

        Ok(DepGraph { root, deps })
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn setup_pkg(dir: &Path, toml_content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("pkg.toml"), toml_content).unwrap();
    }

    /// Create a temporary directory that is cleaned up automatically.
    fn tempdir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut dir = std::env::temp_dir();
        dir.push(format!("speclang-pkg-test-{}-{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_single_package() {
        let tmp = tempdir();
        setup_pkg(
            &tmp,
            r#"
[package]
name = "root"
version = "1.0.0"
"#,
        );

        let graph = DepGraph::resolve(&tmp.join("pkg.toml")).unwrap();
        assert_eq!(graph.root.name, "root");
        assert!(graph.deps.is_empty());
    }

    #[test]
    fn resolve_with_path_dep() {
        let tmp = tempdir();
        setup_pkg(
            &tmp,
            r#"
[package]
name = "root"
version = "1.0.0"

[dependencies]
lib = { path = "lib" }
"#,
        );
        setup_pkg(
            &tmp.join("lib"),
            r#"
[package]
name = "lib"
version = "0.2.0"
"#,
        );

        let graph = DepGraph::resolve(&tmp.join("pkg.toml")).unwrap();
        assert_eq!(graph.root.name, "root");
        assert_eq!(graph.deps.len(), 1);
        assert_eq!(graph.deps[0].name, "lib");
    }

    #[test]
    fn resolve_transitive_deps() {
        let tmp = tempdir();
        setup_pkg(
            &tmp,
            r#"
[package]
name = "root"
version = "1.0.0"

[dependencies]
mid = { path = "mid" }
"#,
        );
        setup_pkg(
            &tmp.join("mid"),
            r#"
[package]
name = "mid"
version = "0.1.0"

[dependencies]
leaf = { path = "../leaf" }
"#,
        );
        setup_pkg(
            &tmp.join("leaf"),
            r#"
[package]
name = "leaf"
version = "0.1.0"
"#,
        );

        let graph = DepGraph::resolve(&tmp.join("pkg.toml")).unwrap();
        assert_eq!(graph.deps.len(), 2);
        // leaf should come before mid in topo order
        let names: Vec<&str> = graph.deps.iter().map(|d| d.name.as_str()).collect();
        assert!(
            names.iter().position(|n| *n == "leaf").unwrap()
                < names.iter().position(|n| *n == "mid").unwrap(),
            "expected leaf before mid, got {:?}",
            names
        );
    }

    #[test]
    fn version_conflict_detected() {
        let tmp = tempdir();
        setup_pkg(
            &tmp,
            r#"
[package]
name = "root"
version = "1.0.0"

[dependencies]
lib = { version = "1.0.0", path = "lib" }
"#,
        );
        setup_pkg(
            &tmp.join("lib"),
            r#"
[package]
name = "lib"
version = "2.0.0"
"#,
        );

        let err = DepGraph::resolve(&tmp.join("pkg.toml")).unwrap_err();
        assert!(matches!(err, ResolveError::VersionConflict { .. }));
    }

    #[test]
    fn missing_dep_detected() {
        let tmp = tempdir();
        setup_pkg(
            &tmp,
            r#"
[package]
name = "root"
version = "1.0.0"

[dependencies]
missing = { path = "nope" }
"#,
        );

        let err = DepGraph::resolve(&tmp.join("pkg.toml")).unwrap_err();
        assert!(
            matches!(err, ResolveError::NotFound { .. }),
            "expected NotFound, got: {:?}",
            err
        );
    }
}
