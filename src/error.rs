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
