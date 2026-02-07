//! Package manifest data model and TOML parsing.
//!
//! Defines the `pkg.toml` format for speclang packages.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("cannot read '{path}': {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid manifest: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid version '{0}': expected MAJOR.MINOR or MAJOR.MINOR.PATCH")]
    BadVersion(String),
}

// -----------------------------------------------------------------------
// Version
// -----------------------------------------------------------------------

/// Semver-compatible version: MAJOR.MINOR.PATCH (patch defaults to 0).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn parse(s: &str) -> Result<Self, ManifestError> {
        let parts: Vec<&str> = s.split('.').collect();
        match parts.len() {
            2 => {
                let major = parts[0].parse::<u32>().map_err(|_| ManifestError::BadVersion(s.to_string()))?;
                let minor = parts[1].parse::<u32>().map_err(|_| ManifestError::BadVersion(s.to_string()))?;
                Ok(Version { major, minor, patch: 0 })
            }
            3 => {
                let major = parts[0].parse::<u32>().map_err(|_| ManifestError::BadVersion(s.to_string()))?;
                let minor = parts[1].parse::<u32>().map_err(|_| ManifestError::BadVersion(s.to_string()))?;
                let patch = parts[2].parse::<u32>().map_err(|_| ManifestError::BadVersion(s.to_string()))?;
                Ok(Version { major, minor, patch })
            }
            _ => Err(ManifestError::BadVersion(s.to_string())),
        }
    }

    /// Check if `other` is compatible with this version requirement.
    /// Compatible means same major version and >= minor.patch.
    pub fn is_compatible(&self, other: &Version) -> bool {
        if self.major == 0 {
            // Pre-1.0: exact minor match required
            self.major == other.major && self.minor == other.minor && other.patch >= self.patch
        } else {
            self.major == other.major
                && (other.minor > self.minor
                    || (other.minor == self.minor && other.patch >= self.patch))
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// -----------------------------------------------------------------------
// Raw TOML structures (serde)
// -----------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawManifest {
    package: RawPackage,
    #[serde(default)]
    dependencies: BTreeMap<String, RawDependency>,
    #[serde(default)]
    target: Vec<RawTarget>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: String,
    version: String,
    #[serde(default)]
    edition: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawDependency {
    /// Short form: `dep = "0.1"`
    Version(String),
    /// Table form: `dep = { version = "0.1", path = "../dep" }`
    Table {
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct RawTarget {
    name: String,
    #[serde(default)]
    spec: Option<String>,
    #[serde(rename = "impl")]
    #[serde(default)]
    impl_file: Option<String>,
}

// -----------------------------------------------------------------------
// Public data model
// -----------------------------------------------------------------------

/// A parsed and validated package manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub package: PackageMeta,
    pub dependencies: BTreeMap<String, Dependency>,
    pub targets: Vec<Target>,
}

/// Package metadata.
#[derive(Debug, Clone)]
pub struct PackageMeta {
    pub name: String,
    pub version: Version,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub authors: Vec<String>,
}

/// How a dependency is specified.
#[derive(Debug, Clone)]
pub enum DependencyKind {
    /// Version requirement only (resolve from registry).
    Registry { version: Version },
    /// Local path dependency.
    Path { path: String, version: Option<Version> },
}

/// A resolved dependency entry.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub kind: DependencyKind,
}

/// A compilation target within the package.
#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    /// Path to the SPL spec file (relative to manifest).
    pub spec: Option<String>,
    /// Path to the IMPL file (relative to manifest).
    pub impl_file: Option<String>,
}

// -----------------------------------------------------------------------
// Parsing
// -----------------------------------------------------------------------

impl Manifest {
    /// Parse a manifest from a TOML string.
    pub fn from_str(toml_str: &str) -> Result<Self, ManifestError> {
        let raw: RawManifest = toml::from_str(toml_str)?;
        Self::from_raw(raw)
    }

    /// Load a manifest from a file path.
    pub fn from_file(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        Self::from_str(&content)
    }

    fn from_raw(raw: RawManifest) -> Result<Self, ManifestError> {
        let version = Version::parse(&raw.package.version)?;

        let package = PackageMeta {
            name: raw.package.name,
            version,
            edition: raw.package.edition,
            description: raw.package.description,
            license: raw.package.license,
            authors: raw.package.authors,
        };

        let mut dependencies = BTreeMap::new();
        for (name, raw_dep) in raw.dependencies {
            let kind = match raw_dep {
                RawDependency::Version(v) => {
                    DependencyKind::Registry { version: Version::parse(&v)? }
                }
                RawDependency::Table { version, path } => {
                    if let Some(p) = path {
                        DependencyKind::Path {
                            path: p,
                            version: version.map(|v| Version::parse(&v)).transpose()?,
                        }
                    } else if let Some(v) = version {
                        DependencyKind::Registry { version: Version::parse(&v)? }
                    } else {
                        return Err(ManifestError::MissingField(
                            "dependency must have 'version' or 'path'",
                        ));
                    }
                }
            };
            dependencies.insert(name.clone(), Dependency { name, kind });
        }

        let targets = raw
            .target
            .into_iter()
            .map(|t| Target {
                name: t.name,
                spec: t.spec,
                impl_file: t.impl_file,
            })
            .collect();

        Ok(Manifest {
            package,
            dependencies,
            targets,
        })
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[package]
name = "hello"
version = "0.1.0"
"#;
        let m = Manifest::from_str(toml).unwrap();
        assert_eq!(m.package.name, "hello");
        assert_eq!(m.package.version, Version::new(0, 1, 0));
        assert!(m.dependencies.is_empty());
        assert!(m.targets.is_empty());
    }

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[package]
name = "my-app"
version = "1.2.3"
edition = "2025"
description = "A sample app"
license = "MIT"
authors = ["Alice", "Bob"]

[dependencies]
std = "0.1"
utils = { path = "../utils" }
net = { version = "0.3.0", path = "../net" }

[[target]]
name = "main"
spec = "src/main.spl"
impl = "src/main.impl"

[[target]]
name = "lib"
spec = "src/lib.spl"
"#;
        let m = Manifest::from_str(toml).unwrap();
        assert_eq!(m.package.name, "my-app");
        assert_eq!(m.package.version, Version::new(1, 2, 3));
        assert_eq!(m.package.edition.as_deref(), Some("2025"));
        assert_eq!(m.package.authors.len(), 2);
        assert_eq!(m.dependencies.len(), 3);

        // std — registry dep
        let std_dep = &m.dependencies["std"];
        match &std_dep.kind {
            DependencyKind::Registry { version } => {
                assert_eq!(*version, Version::new(0, 1, 0));
            }
            _ => panic!("expected registry dep"),
        }

        // utils — path dep
        let utils_dep = &m.dependencies["utils"];
        match &utils_dep.kind {
            DependencyKind::Path { path, version } => {
                assert_eq!(path, "../utils");
                assert!(version.is_none());
            }
            _ => panic!("expected path dep"),
        }

        // net — path dep with version
        let net_dep = &m.dependencies["net"];
        match &net_dep.kind {
            DependencyKind::Path { path, version } => {
                assert_eq!(path, "../net");
                assert_eq!(version.as_ref().unwrap(), &Version::new(0, 3, 0));
            }
            _ => panic!("expected path dep"),
        }

        assert_eq!(m.targets.len(), 2);
        assert_eq!(m.targets[0].name, "main");
        assert_eq!(m.targets[0].spec.as_deref(), Some("src/main.spl"));
        assert_eq!(m.targets[0].impl_file.as_deref(), Some("src/main.impl"));
        assert_eq!(m.targets[1].name, "lib");
        assert!(m.targets[1].impl_file.is_none());
    }

    #[test]
    fn parse_two_part_version() {
        let v = Version::parse("1.2").unwrap();
        assert_eq!(v, Version::new(1, 2, 0));
    }

    #[test]
    fn version_compatibility() {
        let req = Version::new(1, 2, 0);
        assert!(req.is_compatible(&Version::new(1, 2, 0)));
        assert!(req.is_compatible(&Version::new(1, 3, 0)));
        assert!(req.is_compatible(&Version::new(1, 2, 1)));
        assert!(!req.is_compatible(&Version::new(2, 0, 0)));
        assert!(!req.is_compatible(&Version::new(1, 1, 0)));
    }

    #[test]
    fn pre_1_0_compatibility() {
        let req = Version::new(0, 2, 0);
        assert!(req.is_compatible(&Version::new(0, 2, 0)));
        assert!(req.is_compatible(&Version::new(0, 2, 1)));
        assert!(!req.is_compatible(&Version::new(0, 3, 0)));
        assert!(!req.is_compatible(&Version::new(1, 2, 0)));
    }

    #[test]
    fn bad_version() {
        assert!(Version::parse("abc").is_err());
        assert!(Version::parse("1.2.3.4").is_err());
        assert!(Version::parse("").is_err());
    }

    #[test]
    fn missing_dep_version_or_path() {
        let toml = r#"
[package]
name = "bad"
version = "0.1.0"

[dependencies]
broken = {}
"#;
        assert!(Manifest::from_str(toml).is_err());
    }
}
