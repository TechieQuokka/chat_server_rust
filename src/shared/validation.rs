//! Validation Utilities

use validator::ValidationErrors;

use super::error::{AppError, FieldError};

/// Convert validation errors to AppError
pub fn validation_error(errors: ValidationErrors) -> AppError {
    let field_errors: Vec<FieldError> = errors
        .field_errors()
        .iter()
        .flat_map(|(field, errs)| {
            errs.iter().map(move |e| FieldError {
                field: field.to_string(),
                message: e.message.clone().map(|m| m.to_string()).unwrap_or_default(),
            })
        })
        .collect();

    let message = field_errors
        .first()
        .map(|e| format!("{}: {}", e.field, e.message))
        .unwrap_or_else(|| "Validation failed".into());

    AppError::Validation(message)
}
