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
    /// CalVer semantic version: YYYY.M.D+channel.commit or YYYY.M.D+commit
    pub semver: String,
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
            semver: env!("SEMVER").to_string(),
        }
    }

    pub fn version_string(&self) -> String {
        // format: semver (date) with optional dirty marker
        let dirty_marker = if self.dirty { " *" } else { "" };
        format!(
            "{}{} ({})",
            self.semver,
            dirty_marker,
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

        // generate CalVer semver from parsed data
        let calver = format!(
            "{}.{}.{}",
            timestamp.format("%Y"),
            timestamp.format("%-m"),
            timestamp.format("%-d")
        );
        let semver = match channel.as_str() {
            "stable" => format!("{}+{}", calver, short_commit),
            _ => format!("{}+{}.{}", calver, channel, short_commit),
        };

        Ok(Self {
            channel,
            commit,
            short_commit,
            timestamp,
            build_date: Utc::now(), // not stored in string
            dirty: false,
            semver,
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
    /// version when schema was last generated
    #[serde(default)]
    pub schema_version: Option<String>,
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
            schema_version: None,
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

        // format: "semver (date)" or "semver * (date)" if dirty
        // semver contains commit hash in build metadata
        assert!(version_str.contains(&version.short_commit));
        assert!(version_str.contains("("));
        assert!(version_str.contains(")"));
        // should contain CalVer date pattern (YYYY.M.D)
        assert!(version_str.contains("."));
    }

    #[test]
    fn test_version_semver() {
        let version = Version::current();

        // semver should be in CalVer format: YYYY.M.D+channel.commit or YYYY.M.D+commit
        assert!(version.semver.contains("+"));
        assert!(version.semver.contains(&version.short_commit));
        // should contain year
        let year = chrono::Utc::now().format("%Y").to_string();
        assert!(version.semver.contains(&year) || version.semver.starts_with("20"));
    }

    #[test]
    fn test_version_parse_from_string() {
        let version = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();

        assert_eq!(version.channel, "stable");
        assert_eq!(version.commit, "a3f2b1c4");
        assert_eq!(version.short_commit, "a3f2b1c4");
        assert_eq!(version.timestamp.format("%Y%m%d").to_string(), "20240211");
        // stable semver: YYYY.M.D+commit (no channel prefix)
        assert_eq!(version.semver, "2024.2.11+a3f2b1c4");
    }

    #[test]
    fn test_version_parse_from_string_beta() {
        let version = Version::parse_from_string("beta-12345678-20240315").unwrap();

        assert_eq!(version.channel, "beta");
        assert_eq!(version.commit, "12345678");
        // beta semver: YYYY.M.D+beta.commit
        assert_eq!(version.semver, "2024.3.15+beta.12345678");
    }

    #[test]
    fn test_version_parse_from_string_dev() {
        let version = Version::parse_from_string("dev-abcdef12-20240101").unwrap();

        assert_eq!(version.channel, "dev");
        assert_eq!(version.commit, "abcdef12");
        // dev semver: YYYY.M.D+dev.commit
        assert_eq!(version.semver, "2024.1.1+dev.abcdef12");
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
            schema_version: Some("stable-a3f2b1c4-20240211".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: VersionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.current, info.current);
        assert_eq!(parsed.previous, info.previous);
        assert_eq!(parsed.install_path, info.install_path);
        assert_eq!(parsed.schema_version, info.schema_version);
    }

    // ========================================================================
    // Additional Version tests
    // ========================================================================

    #[test]
    fn test_version_dirty_marker() {
        // test version_string with dirty flag
        let mut version = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        version.dirty = true;

        let version_str = version.version_string();
        assert!(version_str.contains(" *"));
    }

    #[test]
    fn test_version_clean_no_marker() {
        let mut version = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        version.dirty = false;

        let version_str = version.version_string();
        assert!(!version_str.contains(" *"));
    }

    #[test]
    fn test_version_is_newer_than_same_timestamp() {
        let v1 = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        let v2 = Version::parse_from_string("beta-12345678-20240211").unwrap();

        // same date, neither is newer
        assert!(!v1.is_newer_than(&v2));
        assert!(!v2.is_newer_than(&v1));
    }

    #[test]
    fn test_version_parse_from_string_with_extra_dashes() {
        // version string with extra dashes (e.g., commit hash with dashes)
        let result = Version::parse_from_string("stable-a3f2-b1c4-20240211");
        // this should fail because the date parsing will fail
        assert!(result.is_err());
    }

    #[test]
    fn test_version_parse_from_string_invalid_date() {
        let result = Version::parse_from_string("stable-a3f2b1c4-99999999");
        assert!(result.is_err());
    }

    #[test]
    fn test_version_full_version_string_format() {
        let version = Version::parse_from_string("beta-12345678-20240315").unwrap();
        let full = version.full_version_string();

        // should be in format: channel-commit-date
        assert!(full.starts_with("beta-"));
        assert!(full.contains("12345678"));
        assert!(full.ends_with("20240315"));
    }

    #[test]
    fn test_version_clone() {
        let v1 = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        let v2 = v1.clone();

        assert_eq!(v1.channel, v2.channel);
        assert_eq!(v1.commit, v2.commit);
        assert_eq!(v1.short_commit, v2.short_commit);
        assert_eq!(v1.timestamp, v2.timestamp);
    }

    #[test]
    fn test_version_debug() {
        let version = Version::parse_from_string("stable-a3f2b1c4-20240211").unwrap();
        let debug_str = format!("{:?}", version);

        assert!(debug_str.contains("Version"));
        assert!(debug_str.contains("channel"));
        assert!(debug_str.contains("stable"));
    }

    // ========================================================================
    // Additional VersionInfo tests
    // ========================================================================

    #[test]
    fn test_version_info_default() {
        let info = VersionInfo::default();

        assert!(!info.current.is_empty());
        assert!(info.previous.is_none());
        assert!(info.last_seen_available.is_none());
        assert!(info.schema_version.is_none());
    }

    #[test]
    fn test_version_info_serialization_without_optional_fields() {
        let info = VersionInfo {
            current: "dev-12345678-20240101".to_string(),
            previous: None,
            last_seen_available: None,
            install_date: Utc::now(),
            install_path: PathBuf::from("/usr/local/bin/cwm"),
            schema_version: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: VersionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.current, info.current);
        assert!(parsed.previous.is_none());
        assert!(parsed.last_seen_available.is_none());
        assert!(parsed.schema_version.is_none());
    }

    #[test]
    fn test_version_info_serialization_with_all_fields() {
        let info = VersionInfo {
            current: "stable-a3f2b1c4-20240211".to_string(),
            previous: Some("stable-00000000-20240210".to_string()),
            last_seen_available: Some("stable-b4c5d6e7-20240212".to_string()),
            install_date: Utc::now(),
            install_path: PathBuf::from("/usr/local/bin/cwm"),
            schema_version: Some("stable-a3f2b1c4-20240211".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: VersionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.last_seen_available, info.last_seen_available);
    }

    #[test]
    fn test_version_info_path() {
        let path = VersionInfo::path();
        let path_str = path.to_string_lossy();

        assert!(path_str.contains(".cwm"));
        assert!(path_str.ends_with("version.json"));
    }

    #[test]
    fn test_version_info_clone() {
        let info = VersionInfo {
            current: "stable-a3f2b1c4-20240211".to_string(),
            previous: Some("stable-00000000-20240210".to_string()),
            last_seen_available: None,
            install_date: Utc::now(),
            install_path: PathBuf::from("/usr/local/bin/cwm"),
            schema_version: None,
        };

        let cloned = info.clone();

        assert_eq!(info.current, cloned.current);
        assert_eq!(info.previous, cloned.previous);
        assert_eq!(info.install_path, cloned.install_path);
    }

    #[test]
    fn test_version_info_debug() {
        let info = VersionInfo::default();
        let debug_str = format!("{:?}", info);

        assert!(debug_str.contains("VersionInfo"));
        assert!(debug_str.contains("current"));
        assert!(debug_str.contains("install_path"));
    }

    #[test]
    fn test_version_info_deserialize_without_schema_version() {
        // test backward compatibility - older version.json files won't have schema_version
        let json = r#"{
            "current": "stable-a3f2b1c4-20240211",
            "previous": null,
            "last_seen_available": null,
            "install_date": "2024-02-11T00:00:00Z",
            "install_path": "/usr/local/bin/cwm"
        }"#;

        let parsed: VersionInfo = serde_json::from_str(json).unwrap();

        assert_eq!(parsed.current, "stable-a3f2b1c4-20240211");
        assert!(parsed.schema_version.is_none()); // default value
    }
}
