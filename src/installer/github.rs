use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::UpdateSettings;
use crate::version::Version;

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    #[allow(dead_code)]
    pub name: String,
    pub body: Option<String>,
    #[allow(dead_code)]
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub channel: String,
    pub commit: String,
    pub date: String,
    pub download_url: String,
    pub checksum_url: String,
    pub size: u64,
    pub release_notes: Option<String>,
}

impl ReleaseInfo {
    pub fn from_github_release(release: &GitHubRelease, arch: &str) -> Result<Self> {
        // parse tag to extract channel and commit
        // format: channel-commit (e.g., "stable-a3f2b1c4", "beta-a3f2b1c4", "dev-a3f2b1c4")
        let tag = &release.tag_name;
        let parts: Vec<&str> = tag.split('-').collect();

        if parts.len() < 2 {
            return Err(anyhow!("Invalid tag format: {}", tag));
        }

        let channel = parts[0].to_string();
        let commit = parts[1].to_string();

        // validate channel
        if !["dev", "beta", "stable", "deprecated"].contains(&channel.as_str()) {
            return Err(anyhow!("Invalid channel in tag: {}", channel));
        }

        // skip deprecated releases
        if channel == "deprecated" {
            return Err(anyhow!("Release is deprecated"));
        }

        // find asset for this architecture
        let asset_name_pattern = format!("cwm-{}-{}", channel, arch);
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.contains(&asset_name_pattern) && a.name.ends_with(".tar.gz"))
            .ok_or_else(|| anyhow!("No asset found for architecture: {}", arch))?;

        // find checksum file
        let checksum_url = format!("{}.sha256", asset.browser_download_url);

        Ok(ReleaseInfo {
            version: tag.clone(),
            channel,
            commit,
            date: release.created_at.format("%Y%m%d").to_string(),
            download_url: asset.browser_download_url.clone(),
            checksum_url,
            size: asset.size,
            release_notes: release.body.clone(),
        })
    }

    pub fn to_version(&self) -> Result<Version> {
        Version::parse_from_string(&format!("{}-{}-{}", self.channel, self.commit, self.date))
    }
}

pub struct GitHubClient {
    client: reqwest::blocking::Client,
    repo: String,
    api_base_url: String,
}

fn get_api_base_url() -> String {
    std::env::var("CWM_GITHUB_API_URL").unwrap_or_else(|_| "https://api.github.com".to_string())
}

impl GitHubClient {
    pub fn new(repo: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("cwm-updater/1.0")
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            repo: repo.to_string(),
            api_base_url: get_api_base_url(),
        })
    }

    pub fn fetch_releases(&self) -> Result<Vec<GitHubRelease>> {
        let url = format!("{}/repos/{}/releases", self.api_base_url, self.repo);

        // implement retry with random delay for rate limiting
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;

        loop {
            attempts += 1;

            let response = self.client.get(&url).send()?;

            if response.status().is_success() {
                return response
                    .json::<Vec<GitHubRelease>>()
                    .context("Failed to parse GitHub releases");
            }

            if response.status() == 403 {
                // check if rate limited
                if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
                    if remaining == "0" {
                        if attempts >= MAX_ATTEMPTS {
                            return Err(anyhow!("GitHub API rate limit exceeded"));
                        }

                        // random delay between 30-90 seconds
                        let delay = rand::thread_rng().gen_range(30..90);
                        eprintln!("Rate limited, waiting {} seconds...", delay);
                        std::thread::sleep(Duration::from_secs(delay));
                        continue;
                    }
                }
            }

            return Err(anyhow!(
                "Failed to fetch releases: {} {}",
                response.status(),
                response.text().unwrap_or_default()
            ));
        }
    }

    #[allow(dead_code)]
    pub fn find_latest_release(&self, settings: &UpdateSettings) -> Result<Option<ReleaseInfo>> {
        let releases = self.fetch_releases()?;

        // detect current architecture
        let arch = detect_architecture();

        // filter by enabled channels and find latest
        let mut candidates = Vec::new();

        for release in &releases {
            // try to parse release info
            if let Ok(info) = ReleaseInfo::from_github_release(release, arch) {
                // check if channel is enabled
                let channel_enabled = match info.channel.as_str() {
                    "dev" => settings.channels.dev,
                    "beta" => settings.channels.beta,
                    "stable" => settings.channels.stable,
                    _ => false,
                };

                if channel_enabled {
                    candidates.push(info);
                }
            }
        }

        // sort by version (timestamp) and return latest
        candidates.sort_by(|a, b| b.date.cmp(&a.date));
        Ok(candidates.into_iter().next())
    }

    /// Find best available release based on settings
    /// If `interactive` is true, will prompt user to install beta/dev if no stable exists
    pub fn find_best_available_release(
        &self,
        settings: &UpdateSettings,
    ) -> Result<Option<ReleaseInfo>> {
        self.find_best_available_release_impl(settings, true)
    }

    /// Find best available release without interactive prompts (for background checks)
    pub fn find_best_available_release_silent(
        &self,
        settings: &UpdateSettings,
    ) -> Result<Option<ReleaseInfo>> {
        self.find_best_available_release_impl(settings, false)
    }

    fn find_best_available_release_impl(
        &self,
        settings: &UpdateSettings,
        interactive: bool,
    ) -> Result<Option<ReleaseInfo>> {
        let releases = self.fetch_releases()?;
        let arch = detect_architecture();

        // try channels in priority order: stable, beta, dev
        let channel_priority = vec![
            ("stable", settings.channels.stable),
            ("beta", settings.channels.beta),
            ("dev", settings.channels.dev),
        ];

        for (channel, enabled) in channel_priority {
            if !enabled && channel == "stable" {
                // special case: if user wants stable but none exists, we'll offer alternatives
                continue;
            }

            if !enabled {
                continue;
            }

            // find latest release for this channel
            for release in &releases {
                if let Ok(info) = ReleaseInfo::from_github_release(release, arch) {
                    if info.channel == channel {
                        return Ok(Some(info));
                    }
                }
            }
        }

        // if user wants stable but none exists, offer beta/dev (only in interactive mode)
        if interactive
            && settings.channels.stable
            && !settings.channels.beta
            && !settings.channels.dev
        {
            eprintln!("No stable release available yet.");

            // try beta
            for release in &releases {
                if let Ok(info) = ReleaseInfo::from_github_release(release, arch) {
                    if info.channel == "beta" {
                        eprintln!("Beta version available: {}", info.version);
                        eprint!("Would you like to install the beta version? [y/N]: ");

                        use std::io::{self, Write};
                        io::stderr().flush()?;

                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;

                        if input.trim().to_lowercase() == "y" {
                            return Ok(Some(info));
                        }
                        break;
                    }
                }
            }

            // try dev
            for release in &releases {
                if let Ok(info) = ReleaseInfo::from_github_release(release, arch) {
                    if info.channel == "dev" {
                        eprintln!("Development version available: {}", info.version);
                        eprint!("Would you like to install the dev version? [y/N]: ");

                        use std::io::{self, Write};
                        io::stderr().flush()?;

                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;

                        if input.trim().to_lowercase() == "y" {
                            return Ok(Some(info));
                        }
                        break;
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn create_issue(&self, title: &str, body: &str, labels: Vec<&str>) -> Result<()> {
        let url = format!("{}/repos/{}/issues", self.api_base_url, self.repo);

        #[derive(Serialize)]
        struct IssueRequest {
            title: String,
            body: String,
            labels: Vec<String>,
        }

        let request = IssueRequest {
            title: title.to_string(),
            body: body.to_string(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
        };

        let response = self.client.post(&url).json(&request).send()?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to create issue: {} {}",
                response.status(),
                response.text().unwrap_or_default()
            ));
        }

        Ok(())
    }
}

fn detect_architecture() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    let arch = "x86_64-apple-darwin";

    #[cfg(target_arch = "aarch64")]
    let arch = "aarch64-apple-darwin";

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let arch = "unknown";

    // check for Rosetta emulation
    use std::process::Command;

    if let Ok(output) = Command::new("sysctl")
        .args(["-n", "sysctl.proc_translated"])
        .output()
    {
        if output.stdout == b"1\n" {
            // running under Rosetta, but we compiled for arm64
            // so we should still use arm64 binaries
            return arch;
        }
    }

    arch
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_release(tag: &str, channel: &str, arch: &str) -> GitHubRelease {
        // asset name must match pattern: cwm-{channel}-{arch}*.tar.gz
        let asset_name = format!("cwm-{}-{}-20240211.tar.gz", channel, arch);
        GitHubRelease {
            tag_name: tag.to_string(),
            name: format!("Release {}", tag),
            body: Some("Test release notes".to_string()),
            prerelease: tag.starts_with("dev-") || tag.starts_with("beta-"),
            created_at: Utc::now(),
            assets: vec![GitHubAsset {
                name: asset_name.clone(),
                browser_download_url: format!("https://example.com/{}", asset_name),
                size: 1024 * 1024,
            }],
        }
    }

    #[test]
    fn test_release_info_from_github_release_stable() {
        let release = create_test_release("stable-a3f2b1c4", "stable", "aarch64-apple-darwin");

        let info = ReleaseInfo::from_github_release(&release, "aarch64-apple-darwin").unwrap();

        assert_eq!(info.channel, "stable");
        assert_eq!(info.commit, "a3f2b1c4");
        assert_eq!(info.version, "stable-a3f2b1c4");
    }

    #[test]
    fn test_release_info_from_github_release_beta() {
        let release = create_test_release("beta-12345678", "beta", "x86_64-apple-darwin");

        let info = ReleaseInfo::from_github_release(&release, "x86_64-apple-darwin").unwrap();

        assert_eq!(info.channel, "beta");
        assert_eq!(info.commit, "12345678");
    }

    #[test]
    fn test_release_info_from_github_release_dev() {
        let release = create_test_release("dev-abcdef12", "dev", "aarch64-apple-darwin");

        let info = ReleaseInfo::from_github_release(&release, "aarch64-apple-darwin").unwrap();

        assert_eq!(info.channel, "dev");
        assert_eq!(info.commit, "abcdef12");
    }

    #[test]
    fn test_release_info_from_github_release_invalid_tag() {
        let release = GitHubRelease {
            tag_name: "invalid".to_string(),
            name: "Invalid Release".to_string(),
            body: None,
            prerelease: false,
            created_at: Utc::now(),
            assets: vec![],
        };

        assert!(ReleaseInfo::from_github_release(&release, "aarch64-apple-darwin").is_err());
    }

    #[test]
    fn test_release_info_from_github_release_deprecated() {
        let release =
            create_test_release("deprecated-a3f2b1c4", "deprecated", "aarch64-apple-darwin");

        // deprecated releases should be skipped
        assert!(ReleaseInfo::from_github_release(&release, "aarch64-apple-darwin").is_err());
    }

    #[test]
    fn test_release_info_from_github_release_no_matching_asset() {
        let release = create_test_release("stable-a3f2b1c4", "stable", "x86_64-apple-darwin");

        // looking for aarch64 but only x86_64 available
        assert!(ReleaseInfo::from_github_release(&release, "aarch64-apple-darwin").is_err());
    }

    #[test]
    fn test_release_info_to_version() {
        let release = create_test_release("stable-a3f2b1c4", "stable", "aarch64-apple-darwin");

        let info = ReleaseInfo::from_github_release(&release, "aarch64-apple-darwin").unwrap();
        let version = info.to_version().unwrap();

        assert_eq!(version.channel, "stable");
        assert_eq!(version.commit, "a3f2b1c4");
    }

    #[test]
    fn test_detect_architecture() {
        let arch = detect_architecture();

        #[cfg(target_arch = "aarch64")]
        assert_eq!(arch, "aarch64-apple-darwin");

        #[cfg(target_arch = "x86_64")]
        assert_eq!(arch, "x86_64-apple-darwin");
    }

    #[test]
    fn test_github_client_new() {
        let client = GitHubClient::new("taulfsime/cool-window-manager");
        assert!(client.is_ok());
    }
}
