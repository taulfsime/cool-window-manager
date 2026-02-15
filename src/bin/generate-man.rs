// generates man page from clap CLI definition
// useful for local development and viewing the man page
// note: the main build uses build.rs to generate the man page to OUT_DIR

use clap::CommandFactory;
use clap_mangen::Man;
use cwm::cli::Cli;

fn main() -> std::io::Result<()> {
    let cmd = Cli::command();
    let man = Man::new(cmd);

    // output to target/man-gen for local development
    let out_dir = "target/man-gen";
    std::fs::create_dir_all(out_dir)?;

    let mut buffer = Vec::new();
    man.render(&mut buffer)?;

    let man_path = format!("{}/cwm.1", out_dir);
    std::fs::write(&man_path, buffer)?;

    println!("Generated {}", man_path);
    println!("View with: man {}", man_path);
    Ok(())
}
