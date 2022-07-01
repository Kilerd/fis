use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};


#[derive(Parser, Debug)]
pub struct Opts {
    /// base path of zhang project
    pub path: PathBuf,

    /// name of author, keep it empty to use default git name
    #[clap(long)]
    pub author_name: Option<String>,

    /// email of author, keep it empty to use default git email
    #[clap(long)]
    pub author_email: Option<String>
}