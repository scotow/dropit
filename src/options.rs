use std::net::IpAddr;
use std::path::PathBuf;

use byte_unit::{Byte, ByteError};
use clap::Parser;
use log::LevelFilter;

use crate::auth::{Credential, Origin};
use crate::upload::expiration::Threshold;
use crate::Features;

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Options {
    #[clap(short = 'v', long = "verbose", parse(from_occurrences = parse_log_level))]
    pub log_level: LevelFilter,
    #[clap(short = 'u', long, default_value = "uploads")]
    pub uploads_dir: PathBuf,
    #[clap(short = 'U', long)]
    pub no_uploads_dir_creation: bool,
    #[clap(short = 'd', long, default_value = "dropit.db")]
    pub database: PathBuf,
    #[clap(short = 'D', long)]
    pub no_database_creation: bool,
    #[clap(short = 'a', long, default_value = "127.0.0.1")]
    pub address: IpAddr,
    #[clap(short = 'p', long, default_value = "8080")]
    pub port: u16,
    #[clap(short = 'R', long = "behind-reverse-proxy")]
    pub behind_proxy: bool,
    #[clap(short = 't', long = "threshold", required = true)]
    pub thresholds: Vec<Threshold>,
    #[clap(
        short = 'o',
        long,
        conflicts_with = "username-origin",
        required_unless_present = "username-origin"
    )]
    pub ip_origin: bool,
    #[clap(
        short = 'O',
        long,
        conflicts_with = "ip-origin",
        required_unless_present = "ip-origin"
    )]
    pub username_origin: bool,
    #[clap(short = 's', long, required = true, parse(try_from_str = parse_size))]
    pub origin_size_sum: u64,
    #[clap(short = 'c', long, required = true)]
    pub origin_file_count: usize,
    #[clap(short = 'S', long, required = true, parse(try_from_str = parse_size))]
    pub global_size_sum: u64,
    #[clap(long)] // requires_any = "credentials" | "ldap..."
    pub auth_upload: bool,
    #[clap(long)] // requires_any = "credentials" | "ldap..."
    pub auth_download: bool,
    #[clap(short = 'C', long = "credential")]
    pub credentials: Vec<Credential>,
    #[clap(long)]
    pub ldap_address: Option<String>,
    #[clap(long, requires = "ldap-address")]
    pub ldap_search_dn: Option<String>,
    #[clap(long, requires_all = &["ldap-search-dn", "ldap-address"])]
    pub ldap_search_password: Option<String>,
    #[clap(long, requires = "ldap-address")]
    pub ldap_base_dn: Option<String>,
    #[clap(long, default_value = "uid", requires = "ldap-address")]
    pub ldap_attribute: String,
    #[clap(short = 'T', long, default_value = "#15b154")]
    pub theme: String,
}

impl Options {
    pub fn origin(&self) -> Option<Origin> {
        if self.ip_origin {
            Some(Origin::IpAddress)
        } else if self.username_origin {
            Some(Origin::Username)
        } else {
            None
        }
    }

    pub fn access(&self) -> Features {
        let mut access = Features::empty();
        if self.auth_upload {
            access.insert(Features::UPLOAD);
        }
        if self.auth_download {
            access.insert(Features::DOWNLOAD);
        }
        access
    }
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
