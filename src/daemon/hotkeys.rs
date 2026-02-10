use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
    pub shift: bool,
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            cmd: false,
            shift: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub key: String,
}

impl Hotkey {
    /// Parse a hotkey string like "ctrl+alt+s" or "cmd+shift+return"
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<String> = s.split('+').map(|p| p.trim().to_lowercase()).collect();

        if parts.is_empty() {
            return Err(anyhow!("Empty hotkey string"));
        }

        let mut modifiers = Modifiers::default();
        let mut key: Option<String> = None;

        for part in &parts {
            match part.as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "option" | "opt" => modifiers.alt = true,
                "cmd" | "command" | "meta" | "super" => modifiers.cmd = true,
                "shift" => modifiers.shift = true,
                _ => {
                    if key.is_some() {
                        return Err(anyhow!(
                            "Multiple non-modifier keys in hotkey: '{}'. Only one key allowed.",
                            s
                        ));
                    }
                    key = Some(part.clone());
                }
            }
        }

        let key = key.ok_or_else(|| anyhow!("No key specified in hotkey: '{}'", s))?;

        Ok(Hotkey { modifiers, key })
    }

    /// Convert hotkey back to string representation
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.ctrl {
            parts.push("ctrl");
        }
        if self.modifiers.alt {
            parts.push("alt");
        }
        if self.modifiers.cmd {
            parts.push("cmd");
        }
        if self.modifiers.shift {
            parts.push("shift");
        }

        parts.push(&self.key);

        parts.join("+")
    }
}

/// Record a single keypress and return the hotkey string
#[cfg(target_os = "macos")]
pub fn record_hotkey() -> Result<String> {
    // TODO: implement hotkey recording using CGEventTap
    Err(anyhow!("Hotkey recording not yet implemented"))
}

#[cfg(not(target_os = "macos"))]
pub fn record_hotkey() -> Result<String> {
    Err(anyhow!("Hotkey recording is only supported on macOS"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_hotkey() {
        let hk = Hotkey::parse("ctrl+alt+s").unwrap();
        assert!(hk.modifiers.ctrl);
        assert!(hk.modifiers.alt);
        assert!(!hk.modifiers.cmd);
        assert!(!hk.modifiers.shift);
        assert_eq!(hk.key, "s");
    }

    #[test]
    fn test_parse_hotkey_with_cmd() {
        let hk = Hotkey::parse("cmd+shift+return").unwrap();
        assert!(!hk.modifiers.ctrl);
        assert!(!hk.modifiers.alt);
        assert!(hk.modifiers.cmd);
        assert!(hk.modifiers.shift);
        assert_eq!(hk.key, "return");
    }

    #[test]
    fn test_parse_hotkey_aliases() {
        let hk = Hotkey::parse("control+option+command+a").unwrap();
        assert!(hk.modifiers.ctrl);
        assert!(hk.modifiers.alt);
        assert!(hk.modifiers.cmd);
        assert_eq!(hk.key, "a");
    }

    #[test]
    fn test_hotkey_to_string() {
        let hk = Hotkey {
            modifiers: Modifiers {
                ctrl: true,
                alt: true,
                cmd: false,
                shift: false,
            },
            key: "s".to_string(),
        };
        assert_eq!(hk.to_string(), "ctrl+alt+s");
    }

    #[test]
    fn test_parse_invalid_hotkey() {
        assert!(Hotkey::parse("").is_err());
        assert!(Hotkey::parse("ctrl+alt").is_err()); // no key
        assert!(Hotkey::parse("ctrl+a+b").is_err()); // multiple keys
    }
}
