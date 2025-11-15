//! Error types for the synth engine.
//!
//! This module defines all error types that can occur during score loading,
//! audio generation, and playback.

use std::fmt;

/// Main error type for the synth engine.
#[derive(Debug)]
pub enum SynthError {
    /// JSON deserialization error
    Json(serde_json::Error),

    /// Invalid parameter value (NaN, infinity, out of range)
    InvalidParameter {
        param: String,
        value: String,
        reason: String,
    },

    /// Audio device error
    AudioDevice(String),

    /// I/O error (file reading/writing)
    Io(std::io::Error),

    /// Invalid path in tree structure
    InvalidPath { path: Vec<usize>, reason: String },

    /// Token not found
    TokenNotFound(usize),
}

impl fmt::Display for SynthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SynthError::Json(e) => write!(f, "JSON error: {}", e),
            SynthError::InvalidParameter {
                param,
                value,
                reason,
            } => {
                write!(f, "Invalid parameter '{}' = {}: {}", param, value, reason)
            }
            SynthError::AudioDevice(msg) => write!(f, "Audio device error: {}", msg),
            SynthError::Io(e) => write!(f, "I/O error: {}", e),
            SynthError::InvalidPath { path, reason } => {
                write!(f, "Invalid path {:?}: {}", path, reason)
            }
            SynthError::TokenNotFound(token) => write!(f, "Token {} not found", token),
        }
    }
}

impl std::error::Error for SynthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SynthError::Json(e) => Some(e),
            SynthError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for SynthError {
    fn from(err: serde_json::Error) -> Self {
        SynthError::Json(err)
    }
}

impl From<std::io::Error> for SynthError {
    fn from(err: std::io::Error) -> Self {
        SynthError::Io(err)
    }
}

/// Result type alias for synth operations.
pub type Result<T> = std::result::Result<T, SynthError>;

/// Validate that a float is finite (not NaN or infinity).
pub fn validate_finite(value: f64, param_name: &str) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SynthError::InvalidParameter {
            param: param_name.to_string(),
            value: format!("{}", value),
            reason: "must be finite (not NaN or infinity)".to_string(),
        })
    }
}

/// Validate that a value is within a range.
pub fn validate_range(value: f64, min: f64, max: f64, param_name: &str) -> Result<f64> {
    validate_finite(value, param_name)?;
    if value >= min && value <= max {
        Ok(value)
    } else {
        Err(SynthError::InvalidParameter {
            param: param_name.to_string(),
            value: format!("{}", value),
            reason: format!("must be in range [{}, {}]", min, max),
        })
    }
}

/// Validate that a value is positive.
pub fn validate_positive(value: f64, param_name: &str) -> Result<f64> {
    validate_finite(value, param_name)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(SynthError::InvalidParameter {
            param: param_name.to_string(),
            value: format!("{}", value),
            reason: "must be positive".to_string(),
        })
    }
}

/// Validate that a value is non-negative.
pub fn validate_non_negative(value: f64, param_name: &str) -> Result<f64> {
    validate_finite(value, param_name)?;
    if value >= 0.0 {
        Ok(value)
    } else {
        Err(SynthError::InvalidParameter {
            param: param_name.to_string(),
            value: format!("{}", value),
            reason: "must be non-negative".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_finite() {
        assert!(validate_finite(1.0, "test").is_ok());
        assert!(validate_finite(0.0, "test").is_ok());
        assert!(validate_finite(-1.0, "test").is_ok());
        assert!(validate_finite(f64::NAN, "test").is_err());
        assert!(validate_finite(f64::INFINITY, "test").is_err());
        assert!(validate_finite(f64::NEG_INFINITY, "test").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(0.5, 0.0, 1.0, "pan").is_ok());
        assert!(validate_range(0.0, 0.0, 1.0, "pan").is_ok());
        assert!(validate_range(1.0, 0.0, 1.0, "pan").is_ok());
        assert!(validate_range(-0.1, 0.0, 1.0, "pan").is_err());
        assert!(validate_range(1.1, 0.0, 1.0, "pan").is_err());
        assert!(validate_range(f64::NAN, 0.0, 1.0, "pan").is_err());
    }

    #[test]
    fn test_validate_positive() {
        assert!(validate_positive(1.0, "freq").is_ok());
        assert!(validate_positive(0.1, "freq").is_ok());
        assert!(validate_positive(0.0, "freq").is_err());
        assert!(validate_positive(-1.0, "freq").is_err());
        assert!(validate_positive(f64::NAN, "freq").is_err());
    }

    #[test]
    fn test_validate_non_negative() {
        assert!(validate_non_negative(1.0, "volume").is_ok());
        assert!(validate_non_negative(0.0, "volume").is_ok());
        assert!(validate_non_negative(-0.1, "volume").is_err());
        assert!(validate_non_negative(f64::NAN, "volume").is_err());
    }
}
