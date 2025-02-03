use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[clap(long, short, default_value_t = 8080)]
    pub port: u16,
    #[clap(long, short, default_value = "MultiplayerFiles")]
    pub folder: PathBuf,
}
