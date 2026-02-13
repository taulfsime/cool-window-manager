// generates man page from clap CLI definition
// outputs to man/cwm.1

use clap::CommandFactory;
use clap_mangen::Man;
use cwm::cli::Cli;

fn main() -> std::io::Result<()> {
    let cmd = Cli::command();
    let man = Man::new(cmd);

    std::fs::create_dir_all("man")?;

    let mut buffer = Vec::new();
    man.render(&mut buffer)?;
    std::fs::write("man/cwm.1", buffer)?;

    println!("Generated man/cwm.1");
    Ok(())
}
