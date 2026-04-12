/// Configuration loading for dialect definitions.
///
/// Supports deserializing a `Dialect` from JSON, TOML, or YAML strings,
/// as well as auto-detecting file format from a file path extension.
use std::fmt;
use std::fs;

use super::Dialect;

/// Errors that can occur when loading a dialect from a configuration file.
#[derive(Debug)]
pub enum DialectConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Toml(toml_crate::de::Error),
    Yaml(serde_yml::Error),
    UnsupportedFormat(String),
}

impl fmt::Display for DialectConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Json(e) => write!(f, "JSON parse error: {}", e),
            Self::Toml(e) => write!(f, "TOML parse error: {}", e),
            Self::Yaml(e) => write!(f, "YAML parse error: {}", e),
            Self::UnsupportedFormat(ext) => {
                write!(f, "unsupported config format: .{}", ext)
            }
        }
    }
}

impl std::error::Error for DialectConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Toml(e) => Some(e),
            Self::Yaml(e) => Some(e),
            Self::UnsupportedFormat(_) => None,
        }
    }
}

impl From<std::io::Error> for DialectConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for DialectConfigError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<toml_crate::de::Error> for DialectConfigError {
    fn from(e: toml_crate::de::Error) -> Self {
        Self::Toml(e)
    }
}

impl From<serde_yml::Error> for DialectConfigError {
    fn from(e: serde_yml::Error) -> Self {
        Self::Yaml(e)
    }
}

impl Dialect {
    /// Deserialize a `Dialect` from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Deserialize a `Dialect` from a TOML string.
    pub fn from_toml_str(toml: &str) -> Result<Self, toml_crate::de::Error> {
        toml_crate::from_str(toml)
    }

    /// Deserialize a `Dialect` from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, serde_yml::Error> {
        serde_yml::from_str(yaml)
    }

    /// Load a `Dialect` from a file, auto-detecting format by extension.
    ///
    /// Supported extensions: `.json`, `.toml`, `.yaml`, `.yml`.
    pub fn from_file(path: &str) -> Result<Self, DialectConfigError> {
        let contents = fs::read_to_string(path)?;
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "json" => Ok(Self::from_json_str(&contents)?),
            "toml" => Ok(Self::from_toml_str(&contents)?),
            "yaml" | "yml" => Ok(Self::from_yaml_str(&contents)?),
            other => Err(DialectConfigError::UnsupportedFormat(other.to_string())),
        }
    }
}
