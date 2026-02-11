use std::process::Command;

fn main() {
    // get git commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Failed to get git commit hash");
    let commit = String::from_utf8(output.stdout).unwrap().trim().to_string();

    // get short commit hash
    let short_commit = if commit.len() >= 8 {
        &commit[..8]
    } else {
        &commit
    };

    // get commit timestamp
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .output()
        .expect("Failed to get commit timestamp");
    let timestamp = String::from_utf8(output.stdout).unwrap().trim().to_string();

    // check if working directory is dirty
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .expect("Failed to check git status");
    let dirty = !output.stdout.is_empty();

    // set environment variables for compilation
    println!("cargo:rustc-env=GIT_COMMIT={}", commit);
    println!("cargo:rustc-env=GIT_COMMIT_SHORT={}", short_commit);
    println!("cargo:rustc-env=GIT_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=GIT_DIRTY={}", dirty);

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

    // rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
