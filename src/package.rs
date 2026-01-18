//! Package Manager
//!
//! Implements dependency management for Hate:
//! - Package manifest (hate.toml)
//! - Dependency resolution
//! - Version constraints
//! - Package registry
//! - Lock file generation

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

/// Semantic version
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
            build: None,
        }
    }
    
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        
        // Split off build metadata
        let (version_pre, build) = if let Some(idx) = s.find('+') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };
        
        // Split off prerelease
        let (version, prerelease) = if let Some(idx) = version_pre.find('-') {
            (&version_pre[..idx], Some(version_pre[idx + 1..].to_string()))
        } else {
            (version_pre, None)
        };
        
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
            prerelease,
            build,
        })
    }
    
    /// Check if this version satisfies a constraint
    pub fn satisfies(&self, constraint: &VersionConstraint) -> bool {
        constraint.is_satisfied_by(self)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.prerelease {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        
        // Prerelease versions have lower precedence
        match (&self.prerelease, &other.prerelease) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

/// Version constraint
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionConstraint {
    /// Exact version: =1.2.3
    Exact(Version),
    /// Greater than: >1.2.3
    Greater(Version),
    /// Greater or equal: >=1.2.3
    GreaterEq(Version),
    /// Less than: <1.2.3
    Less(Version),
    /// Less or equal: <=1.2.3
    LessEq(Version),
    /// Caret (compatible): ^1.2.3 means >=1.2.3, <2.0.0
    Caret(Version),
    /// Tilde (minor compatible): ~1.2.3 means >=1.2.3, <1.3.0
    Tilde(Version),
    /// Wildcard: 1.2.* means >=1.2.0, <1.3.0
    Wildcard { major: u32, minor: Option<u32> },
    /// Any version: *
    Any,
    /// Compound: multiple constraints
    And(Vec<VersionConstraint>),
    /// Or: any of the constraints
    Or(Vec<VersionConstraint>),
}

impl VersionConstraint {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        
        if s == "*" {
            return Some(VersionConstraint::Any);
        }
        
        // Handle compound constraints
        if s.contains(',') {
            let parts: Vec<_> = s.split(',')
                .filter_map(|p| VersionConstraint::parse(p.trim()))
                .collect();
            return if parts.is_empty() {
                None
            } else {
                Some(VersionConstraint::And(parts))
            };
        }
        
        if s.contains("||") {
            let parts: Vec<_> = s.split("||")
                .filter_map(|p| VersionConstraint::parse(p.trim()))
                .collect();
            return if parts.is_empty() {
                None
            } else {
                Some(VersionConstraint::Or(parts))
            };
        }
        
        // Handle wildcards
        if s.ends_with(".*") || s.ends_with(".x") {
            let parts: Vec<&str> = s.split('.').collect();
            return match parts.len() {
                2 => {
                    let major = parts[0].parse().ok()?;
                    Some(VersionConstraint::Wildcard { major, minor: None })
                }
                3 => {
                    let major = parts[0].parse().ok()?;
                    let minor = parts[1].parse().ok()?;
                    Some(VersionConstraint::Wildcard { major, minor: Some(minor) })
                }
                _ => None,
            };
        }
        
        // Handle prefixed versions
        if let Some(rest) = s.strip_prefix("^") {
            return Version::parse(rest).map(VersionConstraint::Caret);
        }
        if let Some(rest) = s.strip_prefix("~") {
            return Version::parse(rest).map(VersionConstraint::Tilde);
        }
        if let Some(rest) = s.strip_prefix(">=") {
            return Version::parse(rest).map(VersionConstraint::GreaterEq);
        }
        if let Some(rest) = s.strip_prefix("<=") {
            return Version::parse(rest).map(VersionConstraint::LessEq);
        }
        if let Some(rest) = s.strip_prefix('>') {
            return Version::parse(rest).map(VersionConstraint::Greater);
        }
        if let Some(rest) = s.strip_prefix('<') {
            return Version::parse(rest).map(VersionConstraint::Less);
        }
        if let Some(rest) = s.strip_prefix('=') {
            return Version::parse(rest).map(VersionConstraint::Exact);
        }
        
        // Plain version = exact
        Version::parse(s).map(VersionConstraint::Exact)
    }
    
    pub fn is_satisfied_by(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Greater(v) => version > v,
            VersionConstraint::GreaterEq(v) => version >= v,
            VersionConstraint::Less(v) => version < v,
            VersionConstraint::LessEq(v) => version <= v,
            VersionConstraint::Caret(v) => {
                // ^1.2.3 means >=1.2.3, <2.0.0 (for major > 0)
                // ^0.2.3 means >=0.2.3, <0.3.0 (for major = 0)
                // ^0.0.3 means >=0.0.3, <0.0.4 (for major = 0, minor = 0)
                if version < v {
                    return false;
                }
                if v.major > 0 {
                    version.major == v.major
                } else if v.minor > 0 {
                    version.major == 0 && version.minor == v.minor
                } else {
                    version.major == 0 && version.minor == 0 && version.patch == v.patch
                }
            }
            VersionConstraint::Tilde(v) => {
                // ~1.2.3 means >=1.2.3, <1.3.0
                version >= v && version.major == v.major && version.minor == v.minor
            }
            VersionConstraint::Wildcard { major, minor } => {
                if version.major != *major {
                    return false;
                }
                if let Some(m) = minor {
                    version.minor == *m
                } else {
                    true
                }
            }
            VersionConstraint::Any => true,
            VersionConstraint::And(constraints) => {
                constraints.iter().all(|c| c.is_satisfied_by(version))
            }
            VersionConstraint::Or(constraints) => {
                constraints.iter().any(|c| c.is_satisfied_by(version))
            }
        }
    }
}

impl std::fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionConstraint::Exact(v) => write!(f, "={}", v),
            VersionConstraint::Greater(v) => write!(f, ">{}", v),
            VersionConstraint::GreaterEq(v) => write!(f, ">={}", v),
            VersionConstraint::Less(v) => write!(f, "<{}", v),
            VersionConstraint::LessEq(v) => write!(f, "<={}", v),
            VersionConstraint::Caret(v) => write!(f, "^{}", v),
            VersionConstraint::Tilde(v) => write!(f, "~{}", v),
            VersionConstraint::Wildcard { major, minor } => {
                if let Some(m) = minor {
                    write!(f, "{}.{}.*", major, m)
                } else {
                    write!(f, "{}.*", major)
                }
            }
            VersionConstraint::Any => write!(f, "*"),
            VersionConstraint::And(cs) => {
                for (i, c) in cs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", c)?;
                }
                Ok(())
            }
            VersionConstraint::Or(cs) => {
                for (i, c) in cs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " || ")?;
                    }
                    write!(f, "{}", c)?;
                }
                Ok(())
            }
        }
    }
}

/// Package dependency
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Package name
    pub name: String,
    /// Version constraint
    pub version: VersionConstraint,
    /// Is it a dev dependency?
    pub dev: bool,
    /// Optional features to enable
    pub features: Vec<String>,
    /// Git repository (if from git)
    pub git: Option<String>,
    /// Path to local package
    pub path: Option<PathBuf>,
}

impl Dependency {
    pub fn new(name: impl Into<String>, version: VersionConstraint) -> Self {
        Self {
            name: name.into(),
            version,
            dev: false,
            features: Vec::new(),
            git: None,
            path: None,
        }
    }
    
    pub fn dev(mut self) -> Self {
        self.dev = true;
        self
    }
    
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

/// Package manifest (hate.toml)
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Package name
    pub name: String,
    /// Package version
    pub version: Version,
    /// Description
    pub description: Option<String>,
    /// Authors
    pub authors: Vec<String>,
    /// License
    pub license: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Entry point
    pub main: Option<String>,
    /// Dependencies
    pub dependencies: Vec<Dependency>,
    /// Dev dependencies
    pub dev_dependencies: Vec<Dependency>,
    /// Package features
    pub features: HashMap<String, Vec<String>>,
}

impl Manifest {
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            main: None,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            features: HashMap::new(),
        }
    }
    
    /// Parse from TOML string
    pub fn parse(_toml: &str) -> Result<Self, String> {
        // In production, use a TOML parser
        // This is a simplified placeholder
        Err("TOML parsing not implemented".to_string())
    }
    
    /// Serialize to TOML string
    pub fn to_toml(&self) -> String {
        let mut s = String::new();
        
        s.push_str("[package]\n");
        s.push_str(&format!("name = \"{}\"\n", self.name));
        s.push_str(&format!("version = \"{}\"\n", self.version));
        
        if let Some(desc) = &self.description {
            s.push_str(&format!("description = \"{}\"\n", desc));
        }
        
        if !self.authors.is_empty() {
            s.push_str(&format!(
                "authors = [{}]\n",
                self.authors.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(", ")
            ));
        }
        
        if let Some(license) = &self.license {
            s.push_str(&format!("license = \"{}\"\n", license));
        }
        
        if !self.dependencies.is_empty() {
            s.push_str("\n[dependencies]\n");
            for dep in &self.dependencies {
                s.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version));
            }
        }
        
        if !self.dev_dependencies.is_empty() {
            s.push_str("\n[dev-dependencies]\n");
            for dep in &self.dev_dependencies {
                s.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version));
            }
        }
        
        s
    }
}

/// Resolved package
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Package name
    pub name: String,
    /// Resolved version
    pub version: Version,
    /// Dependencies (name -> version)
    pub dependencies: HashMap<String, Version>,
    /// Source (registry, git, path)
    pub source: PackageSource,
}

/// Package source
#[derive(Debug, Clone)]
pub enum PackageSource {
    Registry { url: String },
    Git { url: String, rev: String },
    Path { path: PathBuf },
}

/// Lock file entry
#[derive(Debug, Clone)]
pub struct LockEntry {
    pub name: String,
    pub version: Version,
    pub source: PackageSource,
    pub checksum: Option<String>,
    pub dependencies: Vec<String>,
}

/// Lock file
#[derive(Debug, Clone, Default)]
pub struct LockFile {
    /// Lock file version
    pub version: u32,
    /// Locked packages
    pub packages: Vec<LockEntry>,
}

impl LockFile {
    pub fn new() -> Self {
        Self {
            version: 1,
            packages: Vec::new(),
        }
    }
    
    /// Serialize to lock file format
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("# This file is auto-generated by hate package manager\n"));
        s.push_str(&format!("version = {}\n\n", self.version));
        
        for pkg in &self.packages {
            s.push_str(&format!("[[package]]\n"));
            s.push_str(&format!("name = \"{}\"\n", pkg.name));
            s.push_str(&format!("version = \"{}\"\n", pkg.version));
            
            match &pkg.source {
                PackageSource::Registry { url } => {
                    s.push_str(&format!("source = \"registry+{}\"\n", url));
                }
                PackageSource::Git { url, rev } => {
                    s.push_str(&format!("source = \"git+{}#{}\"\n", url, rev));
                }
                PackageSource::Path { path } => {
                    s.push_str(&format!("source = \"path+{}\"\n", path.display()));
                }
            }
            
            if let Some(checksum) = &pkg.checksum {
                s.push_str(&format!("checksum = \"{}\"\n", checksum));
            }
            
            if !pkg.dependencies.is_empty() {
                s.push_str("dependencies = [\n");
                for dep in &pkg.dependencies {
                    s.push_str(&format!("  \"{}\",\n", dep));
                }
                s.push_str("]\n");
            }
            
            s.push('\n');
        }
        
        s
    }
}

/// Dependency resolver
pub struct Resolver {
    /// Available packages (from registry)
    available: HashMap<String, Vec<Version>>,
    /// Resolved packages
    resolved: HashMap<String, Version>,
    /// Resolution queue
    queue: VecDeque<(String, VersionConstraint)>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            resolved: HashMap::new(),
            queue: VecDeque::new(),
        }
    }
    
    /// Register available versions for a package
    pub fn register_package(&mut self, name: impl Into<String>, versions: Vec<Version>) {
        self.available.insert(name.into(), versions);
    }
    
    /// Resolve dependencies
    pub fn resolve(&mut self, dependencies: &[Dependency]) -> Result<HashMap<String, Version>, String> {
        self.resolved.clear();
        self.queue.clear();
        
        // Queue initial dependencies
        for dep in dependencies {
            self.queue.push_back((dep.name.clone(), dep.version.clone()));
        }
        
        // Resolve all dependencies
        while let Some((name, constraint)) = self.queue.pop_front() {
            // Skip if already resolved
            if let Some(resolved_version) = self.resolved.get(&name) {
                // Check compatibility
                if !constraint.is_satisfied_by(resolved_version) {
                    return Err(format!(
                        "Conflicting requirements for {}: need {}, but {} is already resolved",
                        name, constraint, resolved_version
                    ));
                }
                continue;
            }
            
            // Find best matching version
            let versions = self.available.get(&name).ok_or_else(|| {
                format!("Package not found: {}", name)
            })?;
            
            let best_version = versions
                .iter()
                .filter(|v| constraint.is_satisfied_by(v))
                .max()
                .ok_or_else(|| {
                    format!(
                        "No version of {} satisfies {}",
                        name, constraint
                    )
                })?;
            
            self.resolved.insert(name, best_version.clone());
            
            // In a full implementation, we would:
            // 1. Fetch the package's manifest
            // 2. Queue its dependencies
        }
        
        Ok(self.resolved.clone())
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Package manager
pub struct PackageManager {
    /// Workspace root
    root: PathBuf,
    /// Registry URL
    registry: String,
    /// Cache directory
    cache_dir: PathBuf,
    /// Resolver
    resolver: Resolver,
}

impl PackageManager {
    pub fn new(root: PathBuf) -> Self {
        let cache_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".cache"))
            .unwrap_or_else(|_| PathBuf::from(".cache"))
            .join("hate");
        
        Self {
            root,
            registry: "https://registry.hate-lang.org".to_string(),
            cache_dir,
            resolver: Resolver::new(),
        }
    }
    
    /// Initialize a new package
    pub fn init(&self) -> Result<Manifest, String> {
        let name = self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-package");
        
        Ok(Manifest::new(name, Version::new(0, 1, 0)))
    }
    
    /// Install dependencies
    pub fn install(&mut self, manifest: &Manifest) -> Result<LockFile, String> {
        // Resolve dependencies
        let resolved = self.resolver.resolve(&manifest.dependencies)?;
        
        // Generate lock file
        let mut lock = LockFile::new();
        
        for (name, version) in resolved {
            lock.packages.push(LockEntry {
                name,
                version,
                source: PackageSource::Registry {
                    url: self.registry.clone(),
                },
                checksum: None,
                dependencies: Vec::new(),
            });
        }
        
        Ok(lock)
    }
    
    /// Add a dependency
    pub fn add(&self, manifest: &mut Manifest, name: &str, version: &str) -> Result<(), String> {
        let constraint = VersionConstraint::parse(version)
            .ok_or_else(|| format!("Invalid version constraint: {}", version))?;
        
        manifest.dependencies.push(Dependency::new(name, constraint));
        Ok(())
    }
    
    /// Remove a dependency
    pub fn remove(&self, manifest: &mut Manifest, name: &str) -> Result<(), String> {
        let before = manifest.dependencies.len();
        manifest.dependencies.retain(|d| d.name != name);
        
        if manifest.dependencies.len() == before {
            return Err(format!("Dependency not found: {}", name));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        
        let v = Version::parse("1.0.0-alpha").unwrap();
        assert_eq!(v.prerelease, Some("alpha".to_string()));
        
        let v = Version::parse("1.0.0+build123").unwrap();
        assert_eq!(v.build, Some("build123".to_string()));
    }
    
    #[test]
    fn test_version_comparison() {
        assert!(Version::new(1, 0, 0) < Version::new(2, 0, 0));
        assert!(Version::new(1, 1, 0) < Version::new(1, 2, 0));
        assert!(Version::new(1, 0, 1) > Version::new(1, 0, 0));
        
        let mut pre = Version::new(1, 0, 0);
        pre.prerelease = Some("alpha".to_string());
        assert!(pre < Version::new(1, 0, 0));
    }
    
    #[test]
    fn test_caret_constraint() {
        let constraint = VersionConstraint::Caret(Version::new(1, 2, 3));
        
        assert!(constraint.is_satisfied_by(&Version::new(1, 2, 3)));
        assert!(constraint.is_satisfied_by(&Version::new(1, 2, 4)));
        assert!(constraint.is_satisfied_by(&Version::new(1, 9, 9)));
        assert!(!constraint.is_satisfied_by(&Version::new(2, 0, 0)));
        assert!(!constraint.is_satisfied_by(&Version::new(1, 2, 2)));
    }
    
    #[test]
    fn test_tilde_constraint() {
        let constraint = VersionConstraint::Tilde(Version::new(1, 2, 3));
        
        assert!(constraint.is_satisfied_by(&Version::new(1, 2, 3)));
        assert!(constraint.is_satisfied_by(&Version::new(1, 2, 9)));
        assert!(!constraint.is_satisfied_by(&Version::new(1, 3, 0)));
    }
    
    #[test]
    fn test_resolver() {
        let mut resolver = Resolver::new();
        
        resolver.register_package("foo", vec![
            Version::new(1, 0, 0),
            Version::new(1, 1, 0),
            Version::new(2, 0, 0),
        ]);
        
        let deps = vec![
            Dependency::new("foo", VersionConstraint::Caret(Version::new(1, 0, 0))),
        ];
        
        let resolved = resolver.resolve(&deps).unwrap();
        
        assert_eq!(resolved.get("foo"), Some(&Version::new(1, 1, 0)));
    }
    
    #[test]
    fn test_manifest() {
        let mut manifest = Manifest::new("my-pkg", Version::new(1, 0, 0));
        manifest.description = Some("A test package".to_string());
        manifest.license = Some("MIT".to_string());
        
        let toml = manifest.to_toml();
        assert!(toml.contains("name = \"my-pkg\""));
        assert!(toml.contains("version = \"1.0.0\""));
    }
}
