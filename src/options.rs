use std::net::IpAddr;
use std::path::PathBuf;

use byte_unit::{Byte, ByteError};
use log::LevelFilter;
use structopt::StructOpt;

use crate::auth::Credential;
use crate::upload::expiration::Threshold;

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(short = "v", long = "verbose", parse(from_occurrences = parse_log_level))]
    pub log_level: LevelFilter,
    #[structopt(short = "u", long, default_value = "uploads")]
    pub uploads_dir: PathBuf,
    #[structopt(short = "U", long)]
    pub no_uploads_dir_creation: bool,
    #[structopt(short = "d", long, default_value = "dropit.db")]
    pub database: PathBuf,
    #[structopt(short = "D", long)]
    pub no_database_creation: bool,
    #[structopt(short = "a", long, default_value = "127.0.0.1")]
    pub address: IpAddr,
    #[structopt(short = "p", long, default_value = "8080")]
    pub port: u16,
    #[structopt(short = "R", long = "behind-reverse-proxy")]
    pub behind_proxy: bool,
    #[structopt(short = "t", long = "threshold", required = true)]
    pub thresholds: Vec<Threshold>,
    #[structopt(short = "s", long, required = true, parse(try_from_str = parse_size))]
    pub ip_size_sum: u64,
    #[structopt(short = "c", long, required = true)]
    pub ip_file_count: usize,
    #[structopt(short = "S", long, required = true, parse(try_from_str = parse_size))]
    pub global_size_sum: u64,
    #[structopt(short = "C", long = "credential")]
    pub credentials: Vec<Credential>,
    #[structopt(long, requires = "credentials")]
    pub auth_upload: bool,
    #[structopt(long, requires = "credentials")]
    pub auth_download: bool,
    #[structopt(long, requires = "credentials")]
    pub auth_web_ui: bool,
    #[structopt(short = "T", long = "theme", default_value = "#15b154")]
    pub theme: String,
}

fn parse_size(s: &str) -> Result<u64, ByteError> {
    Ok(s.parse::<Byte>()?.get_bytes())
}

fn parse_log_level(n: u64) -> LevelFilter {
    use LevelFilter::*;
    match n {
        0 => Error,
        1 => Warn,
        2 => Info,
        3 => Debug,
        _ => Trace,
    }
}
