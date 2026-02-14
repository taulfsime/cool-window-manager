use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::config::UpdateSettings;
use crate::installer::github::{GitHubClient, ReleaseInfo};
use crate::version::{Version, VersionInfo};

pub fn check_for_updates(settings: &UpdateSettings, verbose: bool) -> Result<Option<ReleaseInfo>> {
    check_for_updates_impl(settings, verbose, true)
}

/// Check for updates without any interactive prompts (for background checks)
pub fn check_for_updates_silent(settings: &UpdateSettings) -> Result<Option<ReleaseInfo>> {
    check_for_updates_impl(settings, false, false)
}

fn check_for_updates_impl(
    settings: &UpdateSettings,
    verbose: bool,
    interactive: bool,
) -> Result<Option<ReleaseInfo>> {
    if !settings.enabled {
        if verbose {
            eprintln!("Updates are disabled in configuration");
        }
        return Ok(None);
    }

    let repo = env!("GITHUB_REPO");
    let client = GitHubClient::new(repo)?;

    // get current version
    let current = Version::current();

    if verbose {
        eprintln!("Current version: {}", current.version_string());
        eprintln!("Checking for updates...");
    }

    // find latest release based on settings
    let latest = if interactive {
        client.find_best_available_release(settings)?
    } else {
        client.find_best_available_release_silent(settings)?
    };

    if let Some(ref release) = latest {
        let latest_version = release.to_version()?;

        if latest_version.is_newer_than(&current) {
            if verbose {
                eprintln!("Found newer version: {}", release.version);
            }
            return Ok(Some(release.clone()));
        } else if verbose {
            eprintln!("You are on the latest version");
        }
    } else if verbose {
        eprintln!("No releases found for enabled channels");
    }

    Ok(None)
}

pub fn download_release(release: &ReleaseInfo) -> Result<PathBuf> {
    let temp_dir = TempDir::new()?;
    let archive_path = temp_dir.path().join("cwm.tar.gz");
    let checksum_path = temp_dir.path().join("checksum.sha256");

    // download binary archive with progress bar
    println!(
        "Downloading {} ({:.2} MB)...",
        release.version,
        release.size as f64 / 1_048_576.0
    );

    download_with_progress(&release.download_url, &archive_path)?;

    // download checksum
    println!("Downloading checksum...");
    download_file(&release.checksum_url, &checksum_path)?;

    // verify checksum
    println!("Verifying checksum...");
    verify_checksum(&archive_path, &checksum_path)?;

    // extract binary
    println!("Extracting...");
    let binary_path = extract_binary(&archive_path, temp_dir.path())?;

    // return path to extracted binary
    // move to a persistent location
    let final_path = temp_dir.path().join("cwm_new");
    fs::rename(&binary_path, &final_path)?;

    // keep the temp dir so it doesn't get cleaned up
    let _ = temp_dir.keep();

    Ok(final_path)
}

fn download_with_progress(url: &str, path: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let mut response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed: {}", response.status()));
    }

    let total_size = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40}] {percent}% ({bytes}/{total_bytes})")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut file = File::create(path)?;
    let mut downloaded = 0u64;
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }

    pb.finish();
    Ok(())
}

fn download_file(url: &str, path: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let mut response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!("Download failed: {}", response.status()));
    }

    let mut file = File::create(path)?;
    io::copy(&mut response, &mut file)?;

    Ok(())
}

fn verify_checksum(file_path: &Path, checksum_path: &Path) -> Result<()> {
    // read expected checksum
    let checksum_content = fs::read_to_string(checksum_path)?;
    let expected_checksum = checksum_content
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("Invalid checksum file format"))?;

    // calculate actual checksum
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let actual_checksum = hex::encode(hasher.finalize());

    if actual_checksum != expected_checksum {
        return Err(anyhow!(
            "Checksum verification failed!\nExpected: {}\nActual: {}",
            expected_checksum,
            actual_checksum
        ));
    }

    println!("✓ Checksum verified");
    Ok(())
}

fn extract_binary(archive_path: &Path, dest_dir: &Path) -> Result<PathBuf> {
    use std::process::Command;

    // use tar to extract
    let output = Command::new("tar")
        .args(["-xzf"])
        .arg(archive_path)
        .arg("-C")
        .arg(dest_dir)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to extract archive: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // find the extracted binary
    let binary_path = dest_dir.join("cwm");
    if !binary_path.exists() {
        return Err(anyhow!("Binary not found in archive"));
    }

    Ok(binary_path)
}

pub fn perform_update(release: ReleaseInfo, _force: bool) -> Result<()> {
    // download new version
    let new_binary = download_release(&release)?;

    // get current binary path
    let current_exe = std::env::current_exe()?;

    // create backup
    let backup_path = current_exe.with_extension("backup");

    println!("Creating backup...");
    fs::copy(&current_exe, &backup_path)?;

    // test new binary
    println!("Testing new binary...");
    if let Err(e) = test_binary(&new_binary) {
        // restore backup
        println!("Test failed, restoring backup...");
        fs::rename(&backup_path, &current_exe)?;
        return Err(anyhow!("Update failed: {}", e));
    }

    // replace current binary
    println!("Installing update...");

    // remove the old binary first before renaming
    fs::remove_file(&current_exe)?;
    fs::rename(&new_binary, &current_exe)?;

    // set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&current_exe)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&current_exe, perms)?;
    }

    // update version info
    let mut version_info = VersionInfo::load().unwrap_or_default();
    version_info.previous = Some(version_info.current.clone());
    version_info.current = release.version.clone();
    version_info.save()?;

    // remove backup
    fs::remove_file(&backup_path).ok();

    // clean up temp files
    if let Some(parent) = new_binary.parent() {
        fs::remove_dir_all(parent).ok();
    }

    // update man page (warn on failure, don't abort)
    if let Err(e) = crate::installer::install::install_man_page(false) {
        eprintln!("⚠️  Failed to update man page: {}", e);
    }

    println!("✓ Successfully updated to {}", release.version);
    Ok(())
}

fn test_binary(binary_path: &Path) -> Result<()> {
    use std::process::Command;

    // make sure it's executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(binary_path, perms)?;
    }

    // run --version to test
    let output = Command::new(binary_path).arg("--version").output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Binary test failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_verify_checksum_valid() {
        let temp_dir = TempDir::new().unwrap();

        // create a test file
        let file_path = temp_dir.path().join("test_file.bin");
        let file_content = b"Hello, World!";
        fs::write(&file_path, file_content).unwrap();

        // calculate expected checksum
        let mut hasher = Sha256::new();
        hasher.update(file_content);
        let expected_checksum = hex::encode(hasher.finalize());

        // create checksum file
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(
            &checksum_path,
            format!("{}  test_file.bin\n", expected_checksum),
        )
        .unwrap();

        // verify should succeed
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_ok(), "checksum verification should succeed");
    }

    #[test]
    fn test_verify_checksum_invalid() {
        let temp_dir = TempDir::new().unwrap();

        // create a test file
        let file_path = temp_dir.path().join("test_file.bin");
        fs::write(&file_path, b"Hello, World!").unwrap();

        // create checksum file with wrong checksum
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(
            &checksum_path,
            "0000000000000000000000000000000000000000000000000000000000000000  test_file.bin\n",
        )
        .unwrap();

        // verify should fail
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_err(), "checksum verification should fail");

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Checksum verification failed"),
            "error should mention checksum failure"
        );
    }

    #[test]
    fn test_verify_checksum_empty_checksum_file() {
        let temp_dir = TempDir::new().unwrap();

        // create a test file
        let file_path = temp_dir.path().join("test_file.bin");
        fs::write(&file_path, b"Hello, World!").unwrap();

        // create empty checksum file
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(&checksum_path, "").unwrap();

        // verify should fail
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_err(), "empty checksum file should fail");

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid checksum file format"),
            "error should mention invalid format"
        );
    }

    #[test]
    fn test_verify_checksum_whitespace_only() {
        let temp_dir = TempDir::new().unwrap();

        // create a test file
        let file_path = temp_dir.path().join("test_file.bin");
        fs::write(&file_path, b"Hello, World!").unwrap();

        // create checksum file with only whitespace
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(&checksum_path, "   \n\t  \n").unwrap();

        // verify should fail
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_err(), "whitespace-only checksum file should fail");
    }

    #[test]
    fn test_verify_checksum_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("nonexistent.bin");
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(&checksum_path, "abc123  nonexistent.bin\n").unwrap();

        // verify should fail because file doesn't exist
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_err(), "missing file should fail");
    }

    #[test]
    fn test_verify_checksum_checksum_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("test_file.bin");
        fs::write(&file_path, b"Hello, World!").unwrap();

        let checksum_path = temp_dir.path().join("nonexistent.sha256");

        // verify should fail because checksum file doesn't exist
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_err(), "missing checksum file should fail");
    }

    #[test]
    fn test_verify_checksum_large_file() {
        let temp_dir = TempDir::new().unwrap();

        // create a larger test file (100KB)
        let file_path = temp_dir.path().join("large_file.bin");
        let mut file = File::create(&file_path).unwrap();
        let data = vec![0xABu8; 100 * 1024];
        file.write_all(&data).unwrap();

        // calculate expected checksum
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let expected_checksum = hex::encode(hasher.finalize());

        // create checksum file
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(&checksum_path, format!("{}\n", expected_checksum)).unwrap();

        // verify should succeed
        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_ok(), "large file checksum should verify");
    }

    #[test]
    fn test_verify_checksum_format_with_filename() {
        let temp_dir = TempDir::new().unwrap();

        // create a test file
        let file_path = temp_dir.path().join("test.bin");
        let content = b"test content";
        fs::write(&file_path, content).unwrap();

        // calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(content);
        let checksum = hex::encode(hasher.finalize());

        // test format: "checksum  filename" (two spaces, common format)
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(&checksum_path, format!("{}  test.bin\n", checksum)).unwrap();

        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_ok(), "checksum with filename should work");
    }

    #[test]
    fn test_verify_checksum_format_checksum_only() {
        let temp_dir = TempDir::new().unwrap();

        // create a test file
        let file_path = temp_dir.path().join("test.bin");
        let content = b"test content";
        fs::write(&file_path, content).unwrap();

        // calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(content);
        let checksum = hex::encode(hasher.finalize());

        // test format: just the checksum
        let checksum_path = temp_dir.path().join("checksum.sha256");
        fs::write(&checksum_path, format!("{}\n", checksum)).unwrap();

        let result = verify_checksum(&file_path, &checksum_path);
        assert!(result.is_ok(), "checksum-only format should work");
    }
}
