//! Request validation and sanitization

use crate::error::{Result, SdkError};
use regex::Regex;
use std::collections::HashMap;

/// Validator trait
pub trait Validator {
    /// Validate value
    fn validate(&self, value: &str) -> Result<()>;
}

/// Email validator
pub struct EmailValidator;

impl Validator for EmailValidator {
    fn validate(&self, value: &str) -> Result<()> {
        let email_regex = Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$",
        ).unwrap();

        if email_regex.is_match(value) {
            Ok(())
        } else {
            Err(SdkError::validation("Invalid email format"))
        }
    }
}

/// URL validator
pub struct UrlValidator;

impl Validator for UrlValidator {
    fn validate(&self, value: &str) -> Result<()> {
        if value.starts_with("http://") || value.starts_with("https://") {
            Ok(())
        } else {
            Err(SdkError::validation("Invalid URL format"))
        }
    }
}

/// UUID validator
pub struct UuidValidator;

impl Validator for UuidValidator {
    fn validate(&self, value: &str) -> Result<()> {
        uuid::Uuid::parse_str(value)
            .map(|_| ())
            .map_err(|_| SdkError::validation("Invalid UUID format"))
    }
}

/// Length validator
pub struct LengthValidator {
    min: usize,
    max: usize,
}

impl LengthValidator {
    /// Create new length validator
    pub fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

impl Validator for LengthValidator {
    fn validate(&self, value: &str) -> Result<()> {
        let len = value.len();
        if len >= self.min && len <= self.max {
            Ok(())
        } else {
            Err(SdkError::validation(format!(
                "Length must be between {} and {}",
                self.min, self.max
            )))
        }
    }
}

/// Alphanumeric validator
pub struct AlphanumericValidator;

impl Validator for AlphanumericValidator {
    fn validate(&self, value: &str) -> Result<()> {
        if value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            Ok(())
        } else {
            Err(SdkError::validation(
                "Value must contain only alphanumeric characters, hyphens, and underscores",
            ))
        }
    }
}

/// Custom regex validator
pub struct RegexValidator {
    pattern: Regex,
}

impl RegexValidator {
    /// Create new regex validator
    pub fn new(pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| SdkError::validation(format!("Invalid regex pattern: {e}")))?;

        Ok(Self { pattern: regex })
    }
}

impl Validator for RegexValidator {
    fn validate(&self, value: &str) -> Result<()> {
        if self.pattern.is_match(value) {
            Ok(())
        } else {
            Err(SdkError::validation("Value does not match pattern"))
        }
    }
}

/// Request validator for complex validation rules
pub struct RequestValidator {
    rules: HashMap<String, Vec<Box<dyn Validator>>>,
}

impl RequestValidator {
    /// Create new request validator
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Add validation rule
    pub fn add_rule(mut self, field: impl Into<String>, validator: Box<dyn Validator>) -> Self {
        self.rules.entry(field.into()).or_default().push(validator);
        self
    }

    /// Validate data
    pub fn validate(&self, data: &HashMap<String, String>) -> Result<()> {
        for (field, validators) in &self.rules {
            if let Some(value) = data.get(field) {
                for validator in validators {
                    validator.validate(value)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for RequestValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitizer for input sanitization
pub struct Sanitizer;

impl Sanitizer {
    /// Sanitize HTML by removing all dangerous tags and attributes.
    ///
    /// Uses the `ammonia` crate which handles all known XSS vectors including:
    /// - `<script>`, `<iframe>`, `<object>`, `<embed>`
    /// - Event handler attributes (`onerror`, `onload`, `onclick`, etc.)
    /// - `javascript:` URLs
    /// - CSS-based attacks
    ///
    /// Safe HTML tags (p, b, i, a, ul, li, etc.) are preserved.
    pub fn sanitize_html(input: &str) -> String {
        ammonia::clean(input)
    }

    /// Sanitize HTML by stripping ALL tags, returning plain text only.
    pub fn strip_all_html(input: &str) -> String {
        ammonia::Builder::new()
            .tags(std::collections::HashSet::new())
            .clean(input)
            .to_string()
    }

    /// Sanitize SQL special characters.
    ///
    /// Note: This is a basic defense-in-depth measure. Always prefer
    /// parameterized queries (see `ParameterizedQuery`) over sanitization.
    pub fn sanitize_sql(input: &str) -> String {
        input
            .replace('\'', "''")
            .replace(';', "")
            .replace("--", "")
            .replace("/*", "")
            .replace("*/", "")
    }

    /// Sanitize path to prevent directory traversal attacks.
    pub fn sanitize_path(input: &str) -> String {
        input
            .replace("../", "")
            .replace("..\\", "")
            .replace("./", "")
            .replace(".\\", "")
    }

    /// Trim whitespace
    pub fn trim(input: &str) -> String {
        input.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validator() {
        let validator = EmailValidator;
        assert!(validator.validate("test@example.com").is_ok());
        assert!(validator.validate("invalid-email").is_err());
    }

    #[test]
    fn test_url_validator() {
        let validator = UrlValidator;
        assert!(validator.validate("https://example.com").is_ok());
        assert!(validator.validate("example.com").is_err());
    }

    #[test]
    fn test_length_validator() {
        let validator = LengthValidator::new(3, 10);
        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("ab").is_err());
        assert!(validator.validate("toolongstring").is_err());
    }

    #[test]
    fn test_alphanumeric_validator() {
        let validator = AlphanumericValidator;
        assert!(validator.validate("hello123").is_ok());
        assert!(validator.validate("hello_world").is_ok());
        assert!(validator.validate("hello@world").is_err());
    }

    #[test]
    fn test_sanitizer_html() {
        // Basic script tag
        let input = "<script>alert('xss')</script>";
        let sanitized = Sanitizer::sanitize_html(input);
        assert!(!sanitized.contains("<script"));

        // img onerror (previously bypassed)
        let input = r#"<img src=x onerror="alert(1)">"#;
        let sanitized = Sanitizer::sanitize_html(input);
        assert!(!sanitized.contains("onerror"));

        // svg onload (previously bypassed)
        let input = r#"<svg onload="alert(1)">"#;
        let sanitized = Sanitizer::sanitize_html(input);
        assert!(!sanitized.contains("onload"));

        // javascript: URL (previously bypassed)
        let input = r#"<a href="javascript:alert(1)">click</a>"#;
        let sanitized = Sanitizer::sanitize_html(input);
        assert!(!sanitized.contains("javascript:"));

        // iframe (previously handled)
        let input = r#"<iframe src="evil.com"></iframe>"#;
        let sanitized = Sanitizer::sanitize_html(input);
        assert!(!sanitized.contains("<iframe"));

        // Safe HTML should be preserved
        let input = "<p>Hello <b>world</b></p>";
        let sanitized = Sanitizer::sanitize_html(input);
        assert!(sanitized.contains("<p>"));
        assert!(sanitized.contains("<b>"));
    }

    #[test]
    fn test_sanitizer_strip_all_html() {
        let input = "<p>Hello <b>world</b></p><script>evil()</script>";
        let sanitized = Sanitizer::strip_all_html(input);
        assert_eq!(sanitized, "Hello world");
    }

    #[test]
    fn test_sanitizer_path() {
        let input = "../../etc/passwd";
        let sanitized = Sanitizer::sanitize_path(input);
        assert!(!sanitized.contains("../"));
    }
}
