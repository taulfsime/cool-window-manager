// signing.rs - code signing utilities for Spotlight app bundles

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// signs an app bundle with ad-hoc signature
/// this provides basic code integrity without requiring a developer certificate
pub fn sign_app_bundle(app_path: &Path) -> Result<SigningResult> {
    // check if codesign is available
    let codesign_check = Command::new("which").arg("codesign").output();

    if codesign_check.is_err() || !codesign_check.unwrap().status.success() {
        return Ok(SigningResult::Skipped("codesign not available".to_string()));
    }

    // sign with ad-hoc identity
    let status = Command::new("codesign")
        .args([
            "-s",
            "-", // ad-hoc signature
            "--force",
            "--deep",
            app_path.to_str().unwrap(),
        ])
        .output();

    match status {
        Ok(output) if output.status.success() => Ok(SigningResult::Success),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(SigningResult::Failed(stderr.to_string()))
        }
        Err(e) => Ok(SigningResult::Failed(e.to_string())),
    }
}

/// result of a signing operation
#[derive(Debug, Clone)]
pub enum SigningResult {
    /// signing succeeded
    Success,
    /// signing was skipped (e.g., codesign not available)
    Skipped(String),
    /// signing failed with an error
    Failed(String),
}

impl SigningResult {
    /// returns true if signing succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, SigningResult::Success)
    }

    /// returns true if signing was skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, SigningResult::Skipped(_))
    }

    /// returns a human-readable description
    pub fn description(&self) -> String {
        match self {
            SigningResult::Success => "signed successfully".to_string(),
            SigningResult::Skipped(reason) => format!("skipped: {}", reason),
            SigningResult::Failed(error) => format!("failed: {}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_result_is_success() {
        assert!(SigningResult::Success.is_success());
        assert!(!SigningResult::Skipped("test".to_string()).is_success());
        assert!(!SigningResult::Failed("test".to_string()).is_success());
    }

    #[test]
    fn test_signing_result_is_skipped() {
        assert!(!SigningResult::Success.is_skipped());
        assert!(SigningResult::Skipped("test".to_string()).is_skipped());
        assert!(!SigningResult::Failed("test".to_string()).is_skipped());
    }

    #[test]
    fn test_signing_result_description() {
        assert_eq!(SigningResult::Success.description(), "signed successfully");
        assert!(SigningResult::Skipped("reason".to_string())
            .description()
            .contains("skipped"));
        assert!(SigningResult::Failed("error".to_string())
            .description()
            .contains("failed"));
    }
}
