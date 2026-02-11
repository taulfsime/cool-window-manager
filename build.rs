use std::process::Command;

fn main() {
    // get git commit hash (with fallback for non-git environments like Docker)
    let (commit, short_commit, timestamp, dirty) = get_git_info();

    fn get_git_info() -> (String, String, String, bool) {
        // try to get git commit hash
        let commit = match Command::new("git").args(["rev-parse", "HEAD"]).output() {
            Ok(output) if output.status.success() => String::from_utf8(output.stdout)
                .unwrap_or_default()
                .trim()
                .to_string(),
            _ => "unknown".to_string(),
        };

        // get short commit hash
        let short_commit = if commit.len() >= 8 {
            commit[..8].to_string()
        } else {
            commit.clone()
        };

        // get commit timestamp
        let timestamp = match Command::new("git")
            .args(["log", "-1", "--format=%ct"])
            .output()
        {
            Ok(output) if output.status.success() => String::from_utf8(output.stdout)
                .unwrap_or_default()
                .trim()
                .to_string(),
            _ => "0".to_string(),
        };

        // check if working directory is dirty
        let dirty = match Command::new("git").args(["status", "--porcelain"]).output() {
            Ok(output) if output.status.success() => !output.stdout.is_empty(),
            _ => false,
        };

        (commit, short_commit, timestamp, dirty)
    }

    // set environment variables for compilation
    println!("cargo:rustc-env=GIT_COMMIT={}", commit);
    println!("cargo:rustc-env=GIT_COMMIT_SHORT={}", short_commit);
    println!("cargo:rustc-env=GIT_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=GIT_DIRTY={}", dirty);

    // only rerun if .git/HEAD exists
    if std::path::Path::new(".git/HEAD").exists() {
        println!("cargo:rerun-if-changed=.git/HEAD");
    }

    // build date
    let build_date = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // github repo from environment or default
    let repo = std::env::var("CWM_GITHUB_REPO")
        .unwrap_or_else(|_| "taulfsime/cool-window-manager".to_string());
    println!("cargo:rustc-env=GITHUB_REPO={}", repo);

    // release channel from environment or default to dev
    let channel = std::env::var("RELEASE_CHANNEL").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=RELEASE_CHANNEL={}", channel);
}
