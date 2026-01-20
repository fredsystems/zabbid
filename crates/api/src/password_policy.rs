// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Password policy validation.
//!
//! This module enforces password requirements for operator credentials.

use thiserror::Error;

/// Password policy errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PasswordPolicyError {
    /// Password is too short.
    #[error("Password must be at least {min_length} characters long")]
    TooShort { min_length: usize },

    /// Password does not meet complexity requirements.
    #[error(
        "Password must contain at least {required} of the following: uppercase letter, lowercase letter, digit, symbol (found {found})"
    )]
    InsufficientComplexity { required: usize, found: usize },

    /// Password matches a forbidden value.
    #[error("Password must not match {field}")]
    MatchesForbiddenField { field: String },

    /// Password and confirmation do not match.
    #[error("Password and confirmation do not match")]
    ConfirmationMismatch,
}

/// Password policy configuration.
pub struct PasswordPolicy {
    /// Minimum password length.
    pub min_length: usize,
    /// Minimum number of character classes required (out of 4).
    pub min_complexity: usize,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 12,
            min_complexity: 3,
        }
    }
}

impl PasswordPolicy {
    /// Validates a password against the policy.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to validate
    /// * `confirmation` - The password confirmation
    /// * `login_name` - The operator login name (password must not match)
    /// * `display_name` - The operator display name (password must not match)
    ///
    /// # Errors
    ///
    /// Returns a `PasswordPolicyError` if the password does not meet policy requirements.
    pub fn validate(
        &self,
        password: &str,
        confirmation: &str,
        login_name: &str,
        display_name: &str,
    ) -> Result<(), PasswordPolicyError> {
        // Check confirmation match
        if password != confirmation {
            return Err(PasswordPolicyError::ConfirmationMismatch);
        }

        // Check minimum length
        if password.len() < self.min_length {
            return Err(PasswordPolicyError::TooShort {
                min_length: self.min_length,
            });
        }

        // Check complexity
        let complexity: usize = Self::calculate_complexity(password);
        if complexity < self.min_complexity {
            return Err(PasswordPolicyError::InsufficientComplexity {
                required: self.min_complexity,
                found: complexity,
            });
        }

        // Check forbidden values (case-insensitive)
        let password_lower: String = password.to_lowercase();
        let login_lower: String = login_name.to_lowercase();
        let display_lower: String = display_name.to_lowercase();

        if password_lower == login_lower {
            return Err(PasswordPolicyError::MatchesForbiddenField {
                field: String::from("login_name"),
            });
        }

        if password_lower == display_lower {
            return Err(PasswordPolicyError::MatchesForbiddenField {
                field: String::from("display_name"),
            });
        }

        Ok(())
    }

    /// Calculates the complexity score of a password.
    ///
    /// Returns the number of character classes present:
    /// - Uppercase letters
    /// - Lowercase letters
    /// - Digits
    /// - Symbols
    fn calculate_complexity(password: &str) -> usize {
        let mut has_uppercase: bool = false;
        let mut has_lowercase: bool = false;
        let mut has_digit: bool = false;
        let mut has_symbol: bool = false;

        for c in password.chars() {
            if c.is_ascii_uppercase() {
                has_uppercase = true;
            } else if c.is_ascii_lowercase() {
                has_lowercase = true;
            } else if c.is_ascii_digit() {
                has_digit = true;
            } else if c.is_ascii_punctuation() || c.is_ascii_graphic() && !c.is_ascii_alphanumeric()
            {
                has_symbol = true;
            }
        }

        let mut complexity: usize = 0;
        if has_uppercase {
            complexity += 1;
        }
        if has_lowercase {
            complexity += 1;
        }
        if has_digit {
            complexity += 1;
        }
        if has_symbol {
            complexity += 1;
        }

        complexity
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_password() {
        let policy: PasswordPolicy = PasswordPolicy::default();

        // Valid: has uppercase, lowercase, digit, and symbol
        assert!(
            policy
                .validate("MyP@ssw0rd123", "MyP@ssw0rd123", "testuser", "Test User")
                .is_ok()
        );

        // Valid: has uppercase, lowercase, and digit (3 of 4)
        assert!(
            policy
                .validate("MyPassword123", "MyPassword123", "testuser", "Test User")
                .is_ok()
        );

        // Valid: has lowercase, digit, and symbol (3 of 4)
        assert!(
            policy
                .validate("mypassword123!", "mypassword123!", "testuser", "Test User")
                .is_ok()
        );

        // Valid: exactly 12 characters
        assert!(
            policy
                .validate("MyPass123!ab", "MyPass123!ab", "testuser", "Test User")
                .is_ok()
        );
    }

    #[test]
    fn test_password_too_short() {
        let policy: PasswordPolicy = PasswordPolicy::default();

        let result: Result<(), PasswordPolicyError> =
            policy.validate("Short1!", "Short1!", "testuser", "Test User");

        assert_eq!(
            result,
            Err(PasswordPolicyError::TooShort { min_length: 12 })
        );
    }

    #[test]
    fn test_insufficient_complexity() {
        let policy: PasswordPolicy = PasswordPolicy::default();

        // Only lowercase (1 of 4)
        let result: Result<(), PasswordPolicyError> =
            policy.validate("alllowercase", "alllowercase", "testuser", "Test User");

        assert_eq!(
            result,
            Err(PasswordPolicyError::InsufficientComplexity {
                required: 3,
                found: 1
            })
        );

        // Only uppercase and lowercase (2 of 4)
        let result: Result<(), PasswordPolicyError> = policy.validate(
            "OnlyLettersHere",
            "OnlyLettersHere",
            "testuser",
            "Test User",
        );

        assert_eq!(
            result,
            Err(PasswordPolicyError::InsufficientComplexity {
                required: 3,
                found: 2
            })
        );
    }

    #[test]
    fn test_matches_login_name() {
        let policy: PasswordPolicy = PasswordPolicy::default();

        // Exact match (case-insensitive)
        let result: Result<(), PasswordPolicyError> =
            policy.validate("TestUser123!", "TestUser123!", "TestUser123!", "Test User");

        assert_eq!(
            result,
            Err(PasswordPolicyError::MatchesForbiddenField {
                field: String::from("login_name")
            })
        );

        // Case-insensitive match
        let result: Result<(), PasswordPolicyError> =
            policy.validate("testuser123!", "testuser123!", "TestUser123!", "Test User");

        assert_eq!(
            result,
            Err(PasswordPolicyError::MatchesForbiddenField {
                field: String::from("login_name")
            })
        );
    }

    #[test]
    fn test_matches_display_name() {
        let policy: PasswordPolicy = PasswordPolicy::default();

        // Exact match (case-insensitive)
        let result: Result<(), PasswordPolicyError> =
            policy.validate("TestUser123!", "TestUser123!", "testuser", "TestUser123!");

        assert_eq!(
            result,
            Err(PasswordPolicyError::MatchesForbiddenField {
                field: String::from("display_name")
            })
        );

        // Case-insensitive match
        let result: Result<(), PasswordPolicyError> =
            policy.validate("testuser123!", "testuser123!", "testuser", "TestUser123!");

        assert_eq!(
            result,
            Err(PasswordPolicyError::MatchesForbiddenField {
                field: String::from("display_name")
            })
        );
    }

    #[test]
    fn test_confirmation_mismatch() {
        let policy: PasswordPolicy = PasswordPolicy::default();

        let result: Result<(), PasswordPolicyError> =
            policy.validate("MyP@ssw0rd123", "MyP@ssw0rd124", "testuser", "Test User");

        assert_eq!(result, Err(PasswordPolicyError::ConfirmationMismatch));
    }

    #[test]
    fn test_complexity_calculation() {
        // All 4 classes
        assert_eq!(PasswordPolicy::calculate_complexity("Aa1!"), 4);

        // 3 classes: uppercase, lowercase, digit
        assert_eq!(PasswordPolicy::calculate_complexity("Aa1"), 3);

        // 2 classes: lowercase, symbol
        assert_eq!(PasswordPolicy::calculate_complexity("abc!"), 2);

        // 1 class: lowercase
        assert_eq!(PasswordPolicy::calculate_complexity("abc"), 1);

        // Empty
        assert_eq!(PasswordPolicy::calculate_complexity(""), 0);
    }
}
