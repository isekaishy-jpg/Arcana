use std::cmp::Ordering;
use std::fmt;

use crate::PackageResult;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemverVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemverVersion {
    pub fn parse(text: &str) -> PackageResult<Self> {
        let parts = text.split('.').collect::<Vec<_>>();
        let [major, minor, patch] = parts.as_slice() else {
            return Err(format!(
                "versions must use `MAJOR.MINOR.PATCH` (found `{text}`)"
            ));
        };
        Ok(Self {
            major: major
                .parse::<u64>()
                .map_err(|_| format!("invalid major version in `{text}`"))?,
            minor: minor
                .parse::<u64>()
                .map_err(|_| format!("invalid minor version in `{text}`"))?,
            patch: patch
                .parse::<u64>()
                .map_err(|_| format!("invalid patch version in `{text}`"))?,
        })
    }

    pub fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for SemverVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionReq {
    Exact(SemverVersion),
    Caret(SemverVersion),
    Tilde(SemverVersion),
}

impl VersionReq {
    pub fn parse(text: &str) -> PackageResult<Self> {
        if let Some(version) = text.strip_prefix('^') {
            return Ok(Self::Caret(SemverVersion::parse(version)?));
        }
        if let Some(version) = text.strip_prefix('~') {
            return Ok(Self::Tilde(SemverVersion::parse(version)?));
        }
        Ok(Self::Exact(SemverVersion::parse(text)?))
    }

    pub fn matches(&self, version: &SemverVersion) -> bool {
        match self {
            Self::Exact(expected) => expected == version,
            Self::Caret(base) => {
                if version < base {
                    return false;
                }
                let upper = if base.major > 0 {
                    SemverVersion {
                        major: base.major + 1,
                        minor: 0,
                        patch: 0,
                    }
                } else if base.minor > 0 {
                    SemverVersion {
                        major: 0,
                        minor: base.minor + 1,
                        patch: 0,
                    }
                } else {
                    SemverVersion {
                        major: 0,
                        minor: 0,
                        patch: base.patch + 1,
                    }
                };
                version < &upper
            }
            Self::Tilde(base) => {
                if version < base {
                    return false;
                }
                let upper = SemverVersion {
                    major: base.major,
                    minor: base.minor + 1,
                    patch: 0,
                };
                version < &upper
            }
        }
    }

    pub fn minimum(&self) -> &SemverVersion {
        match self {
            Self::Exact(version) | Self::Caret(version) | Self::Tilde(version) => version,
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(version) => write!(f, "{version}"),
            Self::Caret(version) => write!(f, "^{version}"),
            Self::Tilde(version) => write!(f, "~{version}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitSelector {
    Rev(String),
    Tag(String),
    Branch(String),
}

impl GitSelector {
    pub fn render(&self) -> String {
        match self {
            Self::Rev(value) => format!("rev:{value}"),
            Self::Tag(value) => format!("tag:{value}"),
            Self::Branch(value) => format!("branch:{value}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceId {
    Path(String),
    Registry {
        registry_name: String,
    },
    Git {
        url: String,
        selector: Option<String>,
    },
}

impl SourceId {
    pub fn render(&self) -> String {
        match self {
            Self::Path(path) => format!("path:{path}"),
            Self::Registry { registry_name } => format!("registry:{registry_name}"),
            Self::Git { url, selector } => match selector {
                Some(selector) => format!("git:{url}#{selector}"),
                None => format!("git:{url}"),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageId {
    Path {
        rel_path: String,
    },
    Registry {
        registry_name: String,
        package_name: String,
        version: SemverVersion,
    },
    Git {
        url: String,
        selector: String,
        package_name: String,
    },
}

impl PackageId {
    pub fn render(&self) -> String {
        match self {
            Self::Path { rel_path } => format!("path:{rel_path}"),
            Self::Registry {
                registry_name,
                package_name,
                version,
            } => {
                format!("registry:{registry_name}:{package_name}@{version}")
            }
            Self::Git {
                url,
                selector,
                package_name,
            } => {
                format!("git:{url}#{selector}:{package_name}")
            }
        }
    }

    pub fn parse(text: &str) -> PackageResult<Self> {
        if let Some(rel_path) = text.strip_prefix("path:") {
            return Ok(Self::Path {
                rel_path: rel_path.to_string(),
            });
        }
        if let Some(rest) = text.strip_prefix("registry:") {
            let (registry_name, package_and_version) = rest
                .split_once(':')
                .ok_or_else(|| format!("invalid registry package id `{text}`"))?;
            let (package_name, version) = package_and_version
                .rsplit_once('@')
                .ok_or_else(|| format!("invalid registry package id `{text}`"))?;
            return Ok(Self::Registry {
                registry_name: registry_name.to_string(),
                package_name: package_name.to_string(),
                version: SemverVersion::parse(version)?,
            });
        }
        if let Some(rest) = text.strip_prefix("git:") {
            let (source, package_name) = rest
                .rsplit_once(':')
                .ok_or_else(|| format!("invalid git package id `{text}`"))?;
            let (url, selector) = source
                .split_once('#')
                .ok_or_else(|| format!("invalid git package id `{text}`"))?;
            return Ok(Self::Git {
                url: url.to_string(),
                selector: selector.to_string(),
                package_name: package_name.to_string(),
            });
        }
        Err(format!("unsupported package id `{text}`"))
    }

    pub fn compare_rendered(left: &str, right: &str) -> Ordering {
        match (Self::parse(left), Self::parse(right)) {
            (Ok(left), Ok(right)) => left.cmp(&right),
            _ => left.cmp(right),
        }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}
