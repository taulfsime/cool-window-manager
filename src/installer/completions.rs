//! shell completion generation and installation for bash, zsh, and fish

use anyhow::{anyhow, Context, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::cli::Cli;

/// supported shells for completion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

impl FromStr for CompletionShell {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            _ => Err(()),
        }
    }
}

impl CompletionShell {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }

    fn to_clap_shell(self) -> Shell {
        match self {
            Self::Bash => Shell::Bash,
            Self::Zsh => Shell::Zsh,
            Self::Fish => Shell::Fish,
        }
    }

    /// get user-level completion file path
    pub fn user_path(&self) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_default();
        match self {
            Self::Zsh => home.join(".zsh/completions/_cwm"),
            Self::Bash => home.join(".bash_completion.d/cwm"),
            Self::Fish => home.join(".config/fish/completions/cwm.fish"),
        }
    }

    /// all supported shells
    pub fn all() -> [Self; 3] {
        [Self::Zsh, Self::Bash, Self::Fish]
    }
}

/// detect current shell from SHELL environment variable
pub fn detect_shell() -> Option<CompletionShell> {
    std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(String::from))
        .and_then(|name| name.parse().ok())
}

/// generate completion script content for a shell
pub fn generate_completion(shell: CompletionShell) -> Vec<u8> {
    let mut cmd = Cli::command();
    let mut buf = Vec::new();
    generate(shell.to_clap_shell(), &mut cmd, "cwm", &mut buf);
    buf
}

/// install completion for a single shell
pub fn install_for_shell(shell: CompletionShell) -> Result<PathBuf> {
    let path = shell.user_path();

    // create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    // generate and write completion
    let content = generate_completion(shell);
    fs::write(&path, &content)
        .with_context(|| format!("failed to write completion to: {}", path.display()))?;

    Ok(path)
}

/// install completions for all shells
#[allow(dead_code)]
pub fn install_all() -> Vec<(CompletionShell, Result<PathBuf>)> {
    CompletionShell::all()
        .into_iter()
        .map(|shell| (shell, install_for_shell(shell)))
        .collect()
}

/// remove installed completion files
pub fn uninstall_all() -> Vec<(CompletionShell, PathBuf)> {
    let mut removed = Vec::new();

    for shell in CompletionShell::all() {
        let path = shell.user_path();
        if path.exists() && fs::remove_file(&path).is_ok() {
            removed.push((shell, path));
        }
    }

    removed
}

/// check which shells have completions installed
pub fn get_installed_shells() -> Vec<CompletionShell> {
    CompletionShell::all()
        .into_iter()
        .filter(|shell| shell.user_path().exists())
        .collect()
}

/// prompt user to select shell for completions
pub fn prompt_shell_selection() -> Result<Option<Vec<CompletionShell>>> {
    let detected = detect_shell();

    println!();
    println!("Shell completion installation:");

    if let Some(shell) = detected {
        println!("  Detected shell: {}", shell.name());
        print!("  Install completions for {}? [Y/n/all]: ", shell.name());
    } else {
        print!("  Could not detect shell. Install for which? [zsh/bash/fish/all/none]: ");
    }

    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if let Some(shell) = detected {
        match input.as_str() {
            "" | "y" | "yes" => Ok(Some(vec![shell])),
            "n" | "no" | "none" => Ok(None),
            "all" => Ok(Some(CompletionShell::all().to_vec())),
            _ => {
                // try to parse as shell name
                if let Ok(s) = input.parse() {
                    Ok(Some(vec![s]))
                } else {
                    Ok(Some(vec![shell])) // default to detected
                }
            }
        }
    } else {
        match input.as_str() {
            "n" | "no" | "none" | "" => Ok(None),
            "all" => Ok(Some(CompletionShell::all().to_vec())),
            _ => {
                if let Ok(s) = input.parse() {
                    Ok(Some(vec![s]))
                } else {
                    eprintln!("Unknown shell: {}. Skipping completions.", input);
                    Ok(None)
                }
            }
        }
    }
}

/// print instructions for enabling completions
pub fn print_enable_instructions(shell: CompletionShell, path: &Path) {
    println!();
    match shell {
        CompletionShell::Zsh => {
            println!("To enable zsh completions, add to ~/.zshrc:");
            println!("  fpath=(~/.zsh/completions $fpath)");
            println!("  autoload -Uz compinit && compinit");
        }
        CompletionShell::Bash => {
            println!("To enable bash completions, add to ~/.bashrc:");
            println!("  [ -f {} ] && source {}", path.display(), path.display());
        }
        CompletionShell::Fish => {
            println!("Fish completions are auto-loaded. Open a new terminal.");
        }
    }
}

/// install completions based on shell argument
/// - "auto": detect shell and install
/// - "all": install for all shells
/// - "zsh"/"bash"/"fish": install for specific shell
pub fn install_completions_for_arg(shell_arg: &str) -> Result<Vec<(CompletionShell, PathBuf)>> {
    let shells = match shell_arg {
        "auto" => match detect_shell() {
            Some(shell) => vec![shell],
            None => {
                return Err(anyhow!(
                    "could not detect shell. Specify shell explicitly: --completions=zsh"
                ));
            }
        },
        "all" => CompletionShell::all().to_vec(),
        name => match name.parse() {
            Ok(shell) => vec![shell],
            Err(_) => {
                return Err(anyhow!(
                    "unknown shell: {}. Valid options: bash, zsh, fish, all",
                    name
                ));
            }
        },
    };

    let mut installed = Vec::new();
    for shell in shells {
        let path = install_for_shell(shell)?;
        installed.push((shell, path));
    }

    Ok(installed)
}

/// refresh completions for any shells that already have them installed
pub fn refresh_installed_completions() -> Vec<(CompletionShell, Result<PathBuf>)> {
    get_installed_shells()
        .into_iter()
        .map(|shell| (shell, install_for_shell(shell)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_from_str() {
        assert_eq!("bash".parse::<CompletionShell>(), Ok(CompletionShell::Bash));
        assert_eq!("ZSH".parse::<CompletionShell>(), Ok(CompletionShell::Zsh));
        assert_eq!("Fish".parse::<CompletionShell>(), Ok(CompletionShell::Fish));
        assert_eq!("unknown".parse::<CompletionShell>(), Err(()));
        assert_eq!("".parse::<CompletionShell>(), Err(()));
    }

    #[test]
    fn test_shell_name() {
        assert_eq!(CompletionShell::Bash.name(), "bash");
        assert_eq!(CompletionShell::Zsh.name(), "zsh");
        assert_eq!(CompletionShell::Fish.name(), "fish");
    }

    #[test]
    fn test_user_paths() {
        let zsh_path = CompletionShell::Zsh.user_path();
        assert!(zsh_path.to_string_lossy().contains("_cwm"));
        assert!(zsh_path.to_string_lossy().contains(".zsh/completions"));

        let bash_path = CompletionShell::Bash.user_path();
        assert!(bash_path.to_string_lossy().ends_with("cwm"));
        assert!(bash_path.to_string_lossy().contains(".bash_completion.d"));

        let fish_path = CompletionShell::Fish.user_path();
        assert!(fish_path.to_string_lossy().ends_with("cwm.fish"));
        assert!(fish_path.to_string_lossy().contains(".config/fish"));
    }

    #[test]
    fn test_generate_zsh_completion() {
        let content = generate_completion(CompletionShell::Zsh);
        let script = String::from_utf8_lossy(&content);
        assert!(
            script.contains("#compdef cwm"),
            "zsh completion should have #compdef header"
        );
        assert!(
            script.contains("focus"),
            "completion should include focus command"
        );
    }

    #[test]
    fn test_generate_bash_completion() {
        let content = generate_completion(CompletionShell::Bash);
        let script = String::from_utf8_lossy(&content);
        // bash completions use _cwm function or complete command
        assert!(
            script.contains("_cwm") || script.contains("complete"),
            "bash completion should define completion function"
        );
    }

    #[test]
    fn test_generate_fish_completion() {
        let content = generate_completion(CompletionShell::Fish);
        let script = String::from_utf8_lossy(&content);
        assert!(
            script.contains("complete -c cwm"),
            "fish completion should use complete -c cwm"
        );
    }

    #[test]
    fn test_all_shells() {
        let all = CompletionShell::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&CompletionShell::Zsh));
        assert!(all.contains(&CompletionShell::Bash));
        assert!(all.contains(&CompletionShell::Fish));
    }

    #[test]
    fn test_detect_shell_returns_valid_or_none() {
        // this test just verifies detect_shell doesn't panic
        let shell = detect_shell();
        if let Some(s) = shell {
            assert!(!s.name().is_empty());
        }
    }
}
