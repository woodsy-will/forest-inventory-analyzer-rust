use thiserror::Error;

/// Errors that can occur in forest inventory analysis.
#[derive(Error, Debug)]
pub enum ForestError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Excel error: {0}")]
    Excel(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Analysis error: {0}")]
    AnalysisError(String),

    #[error("Insufficient data: {0}")]
    InsufficientData(String),
}

impl From<calamine::Error> for ForestError {
    fn from(e: calamine::Error) -> Self {
        ForestError::Excel(e.to_string())
    }
}

impl From<calamine::XlsxError> for ForestError {
    fn from(e: calamine::XlsxError) -> Self {
        ForestError::Excel(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ForestError::from(io_err);
        let msg = err.to_string();
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_excel_error_display() {
        let err = ForestError::Excel("bad sheet".to_string());
        assert_eq!(err.to_string(), "Excel error: bad sheet");
    }

    #[test]
    fn test_parse_error_display() {
        let err = ForestError::ParseError("invalid format".to_string());
        assert_eq!(err.to_string(), "Parse error: invalid format");
    }

    #[test]
    fn test_validation_error_display() {
        let err = ForestError::ValidationError("DBH must be positive".to_string());
        assert_eq!(err.to_string(), "Validation error: DBH must be positive");
    }

    #[test]
    fn test_analysis_error_display() {
        let err = ForestError::AnalysisError("division by zero".to_string());
        assert_eq!(err.to_string(), "Analysis error: division by zero");
    }

    #[test]
    fn test_insufficient_data_display() {
        let err = ForestError::InsufficientData("need 2 plots".to_string());
        assert_eq!(err.to_string(), "Insufficient data: need 2 plots");
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let forest_err: ForestError = io_err.into();
        assert!(matches!(forest_err, ForestError::Io(_)));
    }

    #[test]
    fn test_json_error_from_conversion() {
        let result: Result<serde_json::Value, _> = serde_json::from_str("not valid json{{{");
        let json_err = result.unwrap_err();
        let forest_err: ForestError = json_err.into();
        assert!(matches!(forest_err, ForestError::Json(_)));
        assert!(forest_err.to_string().contains("JSON error"));
    }

    #[test]
    fn test_error_is_debug() {
        let err = ForestError::ParseError("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ParseError"));
    }
}
