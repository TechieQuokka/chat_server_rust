//! Validation Utilities

use validator::ValidationErrors;

use super::error::{AppError, FieldError};

/// Validate password strength
///
/// Requirements:
/// - At least 8 characters
/// - At least one uppercase letter
/// - At least one lowercase letter
/// - At least one digit
/// - At least one special character
pub fn validate_password_strength(password: &str) -> Result<(), validator::ValidationError> {
    let mut has_uppercase = false;
    let mut has_lowercase = false;
    let mut has_digit = false;
    let mut has_special = false;

    if password.len() < 8 {
        let mut err = validator::ValidationError::new("password_too_short");
        err.message = Some("Password must be at least 8 characters".into());
        return Err(err);
    }

    for ch in password.chars() {
        if ch.is_uppercase() {
            has_uppercase = true;
        } else if ch.is_lowercase() {
            has_lowercase = true;
        } else if ch.is_ascii_digit() {
            has_digit = true;
        } else if !ch.is_alphanumeric() {
            has_special = true;
        }
    }

    if !has_uppercase {
        let mut err = validator::ValidationError::new("password_no_uppercase");
        err.message = Some("Password must contain at least one uppercase letter".into());
        return Err(err);
    }

    if !has_lowercase {
        let mut err = validator::ValidationError::new("password_no_lowercase");
        err.message = Some("Password must contain at least one lowercase letter".into());
        return Err(err);
    }

    if !has_digit {
        let mut err = validator::ValidationError::new("password_no_digit");
        err.message = Some("Password must contain at least one digit".into());
        return Err(err);
    }

    if !has_special {
        let mut err = validator::ValidationError::new("password_no_special");
        err.message = Some("Password must contain at least one special character".into());
        return Err(err);
    }

    Ok(())
}

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
