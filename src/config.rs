//! Application configuration loaded from an optional `config.toml` file.
//!
//! [`AppConfig`] groups settings for the web server, statistical analysis, growth modeling,
//! and database storage. All fields have sensible defaults so the config file is optional.

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
    /// Bind address for the web server (default: "127.0.0.1")
    pub bind_address: String,
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

/// Simple tag enum for selecting a growth model type in configuration.
///
/// Unlike [`crate::analysis::growth::GrowthModel`], this enum carries no data fields —
/// it is used purely as a discriminant in config files and is serialized as a
/// lowercase string (`"exponential"`, `"logistic"`, `"linear"`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrowthModelType {
    Exponential,
    #[default]
    Logistic,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrowthConfig {
    /// Default growth model type (default: Logistic)
    pub default_model: GrowthModelType,
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
            bind_address: "127.0.0.1".to_string(),
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
            default_model: GrowthModelType::Logistic,
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
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values are within acceptable ranges.
    pub fn validate(&self) -> Result<(), ForestError> {
        let cl = self.analysis.confidence_level;
        if cl <= 0.0 || cl >= 1.0 {
            return Err(ForestError::ValidationError(format!(
                "confidence_level must be in (0.0, 1.0), got {cl}"
            )));
        }

        if self.analysis.diameter_class_width <= 0.0 {
            return Err(ForestError::ValidationError(format!(
                "diameter_class_width must be > 0.0, got {}",
                self.analysis.diameter_class_width
            )));
        }

        if self.growth.annual_rate < 0.0 {
            return Err(ForestError::ValidationError(format!(
                "annual_rate must be >= 0.0, got {}",
                self.growth.annual_rate
            )));
        }

        let mr = self.growth.mortality_rate;
        if !(0.0..1.0).contains(&mr) {
            return Err(ForestError::ValidationError(format!(
                "mortality_rate must be in [0.0, 1.0), got {mr}"
            )));
        }

        if self.server.max_upload_bytes == 0 {
            return Err(ForestError::ValidationError(
                "max_upload_bytes must be > 0".to_string(),
            ));
        }

        if self.growth.carrying_capacity <= 0.0 {
            return Err(ForestError::ValidationError(format!(
                "carrying_capacity must be > 0.0, got {}",
                self.growth.carrying_capacity
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.bind_address, "127.0.0.1");
        assert_eq!(config.server.max_upload_bytes, 50 * 1024 * 1024);
        assert!((config.analysis.confidence_level - 0.95).abs() < f64::EPSILON);
        assert!((config.analysis.diameter_class_width - 2.0).abs() < f64::EPSILON);
        assert_eq!(config.growth.default_model, GrowthModelType::Logistic);
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
        assert_eq!(config.growth.default_model, GrowthModelType::Logistic);
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
        assert_eq!(config.growth.default_model, GrowthModelType::Exponential);
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

    #[test]
    fn test_validate_default_config_is_valid() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_confidence_level_zero() {
        let mut config = AppConfig::default();
        config.analysis.confidence_level = 0.0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("confidence_level"));
    }

    #[test]
    fn test_validate_confidence_level_one() {
        let mut config = AppConfig::default();
        config.analysis.confidence_level = 1.0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("confidence_level"));
    }

    #[test]
    fn test_validate_confidence_level_negative() {
        let mut config = AppConfig::default();
        config.analysis.confidence_level = -0.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_diameter_class_width_zero() {
        let mut config = AppConfig::default();
        config.analysis.diameter_class_width = 0.0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("diameter_class_width"));
    }

    #[test]
    fn test_validate_negative_annual_rate() {
        let mut config = AppConfig::default();
        config.growth.annual_rate = -0.01;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("annual_rate"));
    }

    #[test]
    fn test_validate_mortality_rate_one() {
        let mut config = AppConfig::default();
        config.growth.mortality_rate = 1.0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("mortality_rate"));
    }

    #[test]
    fn test_validate_mortality_rate_negative() {
        let mut config = AppConfig::default();
        config.growth.mortality_rate = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_max_upload_bytes_zero() {
        let mut config = AppConfig::default();
        config.server.max_upload_bytes = 0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("max_upload_bytes"));
    }

    #[test]
    fn test_validate_carrying_capacity_zero() {
        let mut config = AppConfig::default();
        config.growth.carrying_capacity = 0.0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("carrying_capacity"));
    }

    #[test]
    fn test_validate_carrying_capacity_negative() {
        let mut config = AppConfig::default();
        config.growth.carrying_capacity = -100.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_load_invalid_values_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "[analysis]\nconfidence_level = 1.5\n",
        )
        .unwrap();
        let result = AppConfig::load(&path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("confidence_level"));
    }

    #[test]
    fn test_growth_model_type_serde() {
        // Ensure lowercase serialization roundtrips
        let model = GrowthModelType::Exponential;
        let s = serde_json::to_string(&model).unwrap();
        assert_eq!(s, "\"exponential\"");
        let back: GrowthModelType = serde_json::from_str(&s).unwrap();
        assert_eq!(back, GrowthModelType::Exponential);
    }

    #[test]
    fn test_growth_model_type_default() {
        assert_eq!(GrowthModelType::default(), GrowthModelType::Logistic);
    }
}
