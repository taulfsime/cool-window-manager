use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Version {
    #[allow(dead_code)]
    pub commit: String,
    pub short_commit: String,
    pub timestamp: DateTime<Utc>,
    pub channel: String,
    pub build_date: DateTime<Utc>,
    pub dirty: bool,
}

impl Version {
    pub fn current() -> Self {
        let timestamp =
            DateTime::from_timestamp(env!("GIT_TIMESTAMP").parse::<i64>().unwrap_or(0), 0)
                .unwrap_or_else(Utc::now);

        Self {
            commit: env!("GIT_COMMIT").to_string(),
            short_commit: env!("GIT_COMMIT_SHORT").to_string(),
            timestamp,
            channel: env!("RELEASE_CHANNEL").to_string(),
            build_date: env!("BUILD_DATE").parse().unwrap_or_else(|_| Utc::now()),
            dirty: env!("GIT_DIRTY") == "true",
        }
    }

    pub fn version_string(&self) -> String {
        // format B: hash (channel, date)
        let dirty_marker = if self.dirty { " *" } else { "" };
        format!(
            "{}{} ({}, {})",
            self.short_commit,
            dirty_marker,
            self.channel,
            self.timestamp.format("%Y-%m-%d")
        )
    }

    pub fn is_newer_than(&self, other: &Version) -> bool {
        self.timestamp > other.timestamp
    }

    pub fn parse_from_string(s: &str) -> Result<Self> {
        // parse format: channel-commit-date
        // e.g., "stable-a3f2b1c4-20240211"
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() < 3 {
            anyhow::bail!("Invalid version string: {}", s);
        }

        let channel = parts[0].to_string();
        let commit = parts[1].to_string();
        let short_commit = commit.clone();

        // parse date from YYYYMMDD format
        let date_str = parts[2];
        let timestamp = DateTime::parse_from_str(
            &format!("{} 00:00:00 +0000", date_str),
            "%Y%m%d %H:%M:%S %z",
        )?
        .with_timezone(&Utc);

        Ok(Self {
            channel,
            commit,
            short_commit,
            timestamp,
            build_date: Utc::now(), // not stored in string
            dirty: false,
        })
    }

    pub fn full_version_string(&self) -> String {
        // format: channel-commit-date
        format!(
            "{}-{}-{}",
            self.channel,
            self.short_commit,
            self.timestamp.format("%Y%m%d")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: String,
    pub previous: Option<String>,
    pub last_seen_available: Option<String>,
    pub install_date: DateTime<Utc>,
    pub install_path: PathBuf,
}

impl VersionInfo {
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".cwm")
            .join("version.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            // return default if not found
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self {
            current: Version::current().full_version_string(),
            previous: None,
            last_seen_available: None,
            install_date: Utc::now(),
            install_path: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cwm")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_current() {
        let version = Version::current();
        assert!(!version.commit.is_empty());
        assert!(!version.short_commit.is_empty());
        assert_eq!(version.short_commit.len(), 8);
        assert!(!version.channel.is_empty());
    }

    #[test]
    fn test_version_string_format() {
        let version = Version::current();
        let version_str = version.version_string();

        // format: "hash (channel, date)" or "hash * (channel, date)" if dirty
        assert!(version_str.contains(&version.short_commit));
        assert!(version_str.contains(&version.channel));
        assert!(version_str.contains("("));
        assert!(version_str.contains(")"));
    }

    #[test]
    fn test_version_parse_from_string() {
        let version = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();

        assert_eq!(version.channel, "stable");
        assert_eq!(version.commit, "a3f2b1c4");
        assert_eq!(version.short_commit, "a3f2b1c4");
        assert_eq!(version.timestamp.format("%Y%m%d").to_string(), "20240211");
    }

    #[test]
    fn test_version_parse_from_string_beta() {
        let version = Version::parse_from_string("beta-12345678-20240315").unwrap();

        assert_eq!(version.channel, "beta");
        assert_eq!(version.commit, "12345678");
    }

    #[test]
    fn test_version_parse_from_string_dev() {
        let version = Version::parse_from_string("dev-abcdef12-20240101").unwrap();

        assert_eq!(version.channel, "dev");
        assert_eq!(version.commit, "abcdef12");
    }

    #[test]
    fn test_version_parse_invalid() {
        assert!(Version::parse_from_string("invalid").is_err());
        assert!(Version::parse_from_string("stable").is_err());
        assert!(Version::parse_from_string("stable-abc").is_err());
    }

    #[test]
    fn test_version_full_version_string() {
        let version = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        let full = version.full_version_string();

        assert_eq!(full, "stable-a3f2b1c4-20240211");
    }

    #[test]
    fn test_version_is_newer_than() {
        let older = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        let newer = Version::parse_from_string("stable-b4c5d6e7-20240212").unwrap();

        assert!(newer.is_newer_than(&older));
        assert!(!older.is_newer_than(&newer));
        assert!(!older.is_newer_than(&older));
    }

    #[test]
    fn test_version_info_serialization() {
        let info = VersionInfo {
            current: "stable-a3f2b1c4-20240211".to_string(),
            previous: Some("stable-00000000-20240210".to_string()),
            last_seen_available: None,
            install_date: Utc::now(),
            install_path: PathBuf::from("/usr/local/bin/cwm"),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: VersionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.current, info.current);
        assert_eq!(parsed.previous, info.previous);
        assert_eq!(parsed.install_path, info.install_path);
    }
}
