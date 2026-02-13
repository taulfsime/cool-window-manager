//! exit codes for cwm commands
//!
//! these follow Unix conventions where 0 = success and non-zero = error
//! specific codes help scripts distinguish between failure types

#![allow(dead_code)]

/// command completed successfully
pub const SUCCESS: i32 = 0;

/// general or unknown error
pub const ERROR: i32 = 1;

/// target application not running
pub const APP_NOT_FOUND: i32 = 2;

/// accessibility permissions not granted
pub const PERMISSION_DENIED: i32 = 3;

/// invalid command-line arguments
pub const INVALID_ARGS: i32 = 4;

/// configuration file error
pub const CONFIG_ERROR: i32 = 5;

/// app is running but has no window
pub const WINDOW_NOT_FOUND: i32 = 6;

/// target display doesn't exist
pub const DISPLAY_NOT_FOUND: i32 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes_are_distinct() {
        let codes = [
            SUCCESS,
            ERROR,
            APP_NOT_FOUND,
            PERMISSION_DENIED,
            INVALID_ARGS,
            CONFIG_ERROR,
            WINDOW_NOT_FOUND,
            DISPLAY_NOT_FOUND,
        ];

        // verify all codes are unique
        for (i, &code) in codes.iter().enumerate() {
            for (j, &other) in codes.iter().enumerate() {
                if i != j {
                    assert_ne!(code, other, "exit codes must be unique");
                }
            }
        }
    }

    #[test]
    fn test_success_is_zero() {
        assert_eq!(SUCCESS, 0);
    }

    #[test]
    fn test_error_codes_are_positive() {
        assert!(ERROR > 0);
        assert!(APP_NOT_FOUND > 0);
        assert!(PERMISSION_DENIED > 0);
        assert!(INVALID_ARGS > 0);
        assert!(CONFIG_ERROR > 0);
        assert!(WINDOW_NOT_FOUND > 0);
        assert!(DISPLAY_NOT_FOUND > 0);
    }
}
