use std::path::PathBuf;
use structopt::StructOpt;
use crate::upload::expiration::Threshold;
use std::net::IpAddr;

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(short, long, default_value = "uploads")]
    pub uploads_dir: PathBuf,
    #[structopt(short, long, default_value = "127.0.0.1")]
    pub address: IpAddr,
    #[structopt(short, long, default_value = "8080")]
    pub port: u16,
    #[structopt(short = "R", long = "behind-reverse-proxy")]
    pub behind_proxy: bool,
    #[structopt(short, long = "threshold", required = true)]
    pub thresholds: Vec<Threshold>,
    #[structopt(short, long, required = true)]
    pub ip_size_sum: u64,
    #[structopt(short = "I", long, required = true)]
    pub ip_file_count: usize,
}