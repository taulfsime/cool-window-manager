use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InstallPath {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub exists: bool,
    pub writable: bool,
    pub in_path: bool,
    pub needs_sudo: bool,
}

impl InstallPath {
    pub fn display_name(&self) -> String {
        // convert home directory to ~ for display
        if let Some(home) = dirs::home_dir() {
            if let Ok(relative) = self.path.strip_prefix(&home) {
                return format!("~/{}", relative.display());
            }
        }
        self.path.display().to_string()
    }

    pub fn status_line(&self) -> String {
        let writable = if self.writable {
            "✓ writable"
        } else if self.needs_sudo {
            "✗ needs sudo"
        } else {
            "✗ not writable"
        };

        let in_path = if self.in_path {
            "✓ in PATH"
        } else {
            "✗ not in PATH"
        };

        format!("{:<30} {} {}", self.display_name(), writable, in_path)
    }
}

pub fn detect_install_paths() -> Vec<InstallPath> {
    let mut paths = vec![];

    // check common user paths first (no sudo needed)
    let user_paths = [
        ("~/.local/bin", "User local binaries"),
        ("~/.cargo/bin", "Cargo binaries"),
    ];

    for (path_str, desc) in &user_paths {
        let expanded = shellexpand::tilde(path_str);
        let path_buf = PathBuf::from(expanded.as_ref());

        // always include ~/.local/bin even if it doesn't exist yet
        let should_include = path_str == &"~/.local/bin" || path_buf.exists();

        if should_include {
            paths.push(InstallPath {
                path: path_buf.clone(),
                description: desc.to_string(),
                exists: path_buf.exists(),
                writable: check_writable(&path_buf),
                in_path: is_in_path(&path_buf),
                needs_sudo: false,
            });
        }
    }

    // system paths (may need sudo)
    let system_paths = [
        ("/usr/local/bin", "System-wide binaries"),
        ("/opt/homebrew/bin", "Homebrew binaries (Apple Silicon)"),
    ];

    for (path_str, desc) in &system_paths {
        let path_buf = PathBuf::from(path_str);
        if path_buf.exists() {
            let writable = check_writable(&path_buf);
            paths.push(InstallPath {
                path: path_buf.clone(),
                description: desc.to_string(),
                exists: true,
                writable,
                in_path: is_in_path(&path_buf),
                needs_sudo: !writable,
            });
        }
    }

    paths
}

pub fn check_writable(path: &Path) -> bool {
    if !path.exists() {
        // check if parent is writable
        if let Some(parent) = path.parent() {
            return check_writable(parent);
        }
        return false;
    }

    // try to create a temp file to test writability
    let test_file = path.join(".cwm_write_test");
    match fs::write(&test_file, "test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

pub fn is_in_path(dir: &Path) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        for path in env::split_paths(&path_var) {
            if path == dir {
                return true;
            }
        }
    }
    false
}

pub fn get_path_instructions(dir: &Path) -> String {
    let shell = env::var("SHELL").unwrap_or_else(|_| String::from("/bin/bash"));
    let shell_name = shell.split('/').next_back().unwrap_or("bash");

    let rc_file = match shell_name {
        "zsh" => "~/.zshrc",
        "bash" => "~/.bashrc",
        "fish" => "~/.config/fish/config.fish",
        _ => "~/.profile",
    };

    let export_line = if shell_name == "fish" {
        format!("set -gx PATH {} $PATH", dir.display())
    } else {
        format!("export PATH=\"{}:$PATH\"", dir.display())
    };

    format!(
        "Add this line to your {}:\n  {}\n\nThen reload your shell or run: source {}",
        rc_file, export_line, rc_file
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_detect_install_paths_returns_paths() {
        let paths = detect_install_paths();
        // should always include at least ~/.local/bin
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_install_path_display_name_with_home() {
        let home = dirs::home_dir().unwrap();
        let path = InstallPath {
            path: home.join(".local/bin"),
            description: "Test".to_string(),
            exists: true,
            writable: true,
            in_path: true,
            needs_sudo: false,
        };

        assert_eq!(path.display_name(), "~/.local/bin");
    }

    #[test]
    fn test_install_path_display_name_system() {
        let path = InstallPath {
            path: PathBuf::from("/usr/local/bin"),
            description: "Test".to_string(),
            exists: true,
            writable: false,
            in_path: true,
            needs_sudo: true,
        };

        assert_eq!(path.display_name(), "/usr/local/bin");
    }

    #[test]
    fn test_install_path_status_line() {
        let path = InstallPath {
            path: PathBuf::from("/test/path"),
            description: "Test".to_string(),
            exists: true,
            writable: true,
            in_path: true,
            needs_sudo: false,
        };

        let status = path.status_line();
        assert!(status.contains("✓ writable"));
        assert!(status.contains("✓ in PATH"));
    }

    #[test]
    fn test_install_path_status_line_needs_sudo() {
        let path = InstallPath {
            path: PathBuf::from("/test/path"),
            description: "Test".to_string(),
            exists: true,
            writable: false,
            in_path: true,
            needs_sudo: true,
        };

        let status = path.status_line();
        assert!(status.contains("✗ needs sudo"));
    }

    #[test]
    fn test_check_writable_temp_dir() {
        let temp_dir = env::temp_dir();
        assert!(check_writable(&temp_dir));
    }

    #[test]
    fn test_check_writable_nonexistent_with_writable_parent() {
        let temp_dir = env::temp_dir();
        let nonexistent = temp_dir.join("cwm_test_nonexistent_dir");
        // should return true because parent is writable
        assert!(check_writable(&nonexistent));
    }

    #[test]
    fn test_is_in_path() {
        // PATH should contain at least /usr/bin
        let usr_bin = PathBuf::from("/usr/bin");
        assert!(is_in_path(&usr_bin));
    }

    #[test]
    fn test_is_not_in_path() {
        let random_path = PathBuf::from("/some/random/nonexistent/path");
        assert!(!is_in_path(&random_path));
    }

    #[test]
    fn test_get_path_instructions_contains_export() {
        let path = PathBuf::from("/test/bin");
        let instructions = get_path_instructions(&path);

        // should contain either export or set -gx depending on shell
        assert!(instructions.contains("export PATH") || instructions.contains("set -gx PATH"));
        assert!(instructions.contains("/test/bin"));
    }

    #[test]
    fn test_get_path_instructions_mentions_rc_file() {
        let path = PathBuf::from("/test/bin");
        let instructions = get_path_instructions(&path);

        // should mention some rc file
        assert!(
            instructions.contains(".zshrc")
                || instructions.contains(".bashrc")
                || instructions.contains(".profile")
                || instructions.contains("config.fish")
        );
    }
}
