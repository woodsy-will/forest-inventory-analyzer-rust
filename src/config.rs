use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ForestError;

/// Application configuration loaded from an optional `config.toml` file.
///
/// CLI arguments override values from the config file. All fields have defaults
/// so the config file itself is entirely optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub analysis: AnalysisConfig,
    pub growth: GrowthConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Port for the web server (default: 8080)
    pub port: u16,
    /// Maximum upload size in bytes (default: 50 MB)
    pub max_upload_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalysisConfig {
    /// Confidence level for statistical analysis (default: 0.95)
    pub confidence_level: f64,
    /// Diameter class width in inches (default: 2.0)
    pub diameter_class_width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrowthConfig {
    /// Default growth model: "exponential", "logistic", or "linear" (default: "logistic")
    pub default_model: String,
    /// Default annual growth rate (default: 0.03)
    pub annual_rate: f64,
    /// Default carrying capacity in sq ft/acre (default: 300.0)
    pub carrying_capacity: f64,
    /// Default mortality rate (default: 0.005)
    pub mortality_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Path to SQLite database file (default: "forest_analyzer.db")
    pub path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            max_upload_bytes: 50 * 1024 * 1024,
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            confidence_level: 0.95,
            diameter_class_width: 2.0,
        }
    }
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self {
            default_model: "logistic".to_string(),
            annual_rate: 0.03,
            carrying_capacity: 300.0,
            mortality_rate: 0.005,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "forest_analyzer.db".to_string(),
        }
    }
}

impl AppConfig {
    /// Load configuration from a TOML file, or return defaults if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, ForestError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content).map_err(|e| {
            ForestError::ParseError(format!("Failed to parse config file: {e}"))
        })?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.max_upload_bytes, 50 * 1024 * 1024);
        assert!((config.analysis.confidence_level - 0.95).abs() < f64::EPSILON);
        assert!((config.analysis.diameter_class_width - 2.0).abs() < f64::EPSILON);
        assert_eq!(config.growth.default_model, "logistic");
        assert!((config.growth.annual_rate - 0.03).abs() < f64::EPSILON);
        assert!((config.growth.carrying_capacity - 300.0).abs() < f64::EPSILON);
        assert!((config.growth.mortality_rate - 0.005).abs() < f64::EPSILON);
        assert_eq!(config.database.path, "forest_analyzer.db");
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let config = AppConfig::load(Path::new("nonexistent_config.toml")).unwrap();
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_load_partial_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[server]\nport = 9090\n").unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.server.port, 9090);
        // Other fields should be defaults
        assert!((config.analysis.confidence_level - 0.95).abs() < f64::EPSILON);
        assert_eq!(config.growth.default_model, "logistic");
    }

    #[test]
    fn test_load_full_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[server]
port = 3000
max_upload_bytes = 10485760

[analysis]
confidence_level = 0.90
diameter_class_width = 4.0

[growth]
default_model = "exponential"
annual_rate = 0.05
carrying_capacity = 250.0
mortality_rate = 0.01

[database]
path = "custom.db"
"#,
        )
        .unwrap();

        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.max_upload_bytes, 10_485_760);
        assert!((config.analysis.confidence_level - 0.90).abs() < f64::EPSILON);
        assert!((config.analysis.diameter_class_width - 4.0).abs() < f64::EPSILON);
        assert_eq!(config.growth.default_model, "exponential");
        assert!((config.growth.annual_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(config.database.path, "custom.db");
    }

    #[test]
    fn test_load_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "not valid toml {{{").unwrap();

        let result = AppConfig::load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to parse config file"));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.server.port, config.server.port);
        assert_eq!(deserialized.growth.default_model, config.growth.default_model);
    }
}
