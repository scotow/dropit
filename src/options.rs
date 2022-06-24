use std::net::IpAddr;
use std::path::PathBuf;

use byte_unit::{Byte, ByteError};
use clap::Parser;
use log::LevelFilter;

use crate::auth::{Credential, Features, LdapAuthProcess, LdapAuthenticator, Origin};
use crate::exit_error;
use crate::upload::Threshold;

#[derive(Parser, Debug)]
#[clap(version, about)]
pub struct Options {
    /// Increase logs verbosity (Error (default), Warn, Info, Debug, Trace).
    #[clap(short = 'v', long = "verbose", parse(from_occurrences = parse_log_level))]
    pub log_level: LevelFilter,
    /// Upload files directory path (relative).
    #[clap(short = 'u', long, default_value = "uploads")]
    pub uploads_dir: PathBuf,
    /// Disable upload files directory automatic creation (if missing).
    #[clap(short = 'U', long)]
    pub no_uploads_dir_creation: bool,
    /// Metadata database path (relative).
    #[clap(short = 'd', long, default_value = "dropit.db")]
    pub database: PathBuf,
    /// Disable metadata database automatic creation (if missing).
    #[clap(short = 'D', long)]
    pub no_database_creation: bool,
    /// HTTP listening address.
    #[clap(short = 'a', long, default_value = "127.0.0.1")]
    pub address: IpAddr,
    /// HTTP listening port.
    #[clap(short = 'p', long, default_value = "8080")]
    pub port: u16,
    /// Use X-Forwarded-For, X-Forwarded-Proto and X-Forwarded-Host to determine uploads' origin.
    #[clap(short = 'R', long = "behind-reverse-proxy")]
    pub behind_proxy: bool,
    /// Relations between files' sizes and their durations. Must be ordered by increasing size and decreasing duration.
    #[clap(short = 't', long = "threshold", required = true)]
    pub thresholds: Vec<Threshold>,
    /// Use usernames as uploaders' identities.
    #[clap(
        short = 'o',
        long,
        conflicts_with = "username-origin",
        required_unless_present = "username-origin"
    )]
    pub ip_origin: bool,
    /// Use IP addresses as uploaders' identities.
    #[clap(
        short = 'O',
        long,
        conflicts_with = "ip-origin",
        required_unless_present = "ip-origin"
    )] // requires_any = "credentials" | "ldap..."
    pub username_origin: bool,
    /// Cumulative size limit from the same uploader.
    #[clap(short = 's', long, required = true, parse(try_from_str = parse_size))]
    pub origin_size_sum: u64,
    /// Number of files limit from the same uploader.
    #[clap(short = 'c', long, required = true)]
    pub origin_file_count: usize,
    /// Cumulative size limit from all users.
    #[clap(short = 'S', long, required = true, parse(try_from_str = parse_size))]
    pub global_size_sum: u64,
    /// Protect upload endpoint with authentication.
    #[clap(long)] // requires_any = "credentials" | "ldap..."
    pub auth_upload: bool,
    /// Protect download endpoint with authentication.
    #[clap(long)] // requires_any = "credentials" | "ldap..."
    pub auth_download: bool,
    /// Static list of credentials.
    #[clap(short = 'C', long = "credential")]
    pub credentials: Vec<Credential>,
    /// URI of the LDAP used to authenticate users.
    #[clap(long)]
    pub ldap_address: Option<String>,
    /// LDAP DN pattern used when using single bind process.
    #[clap(
        long,
        requires = "ldap-address",
        conflicts_with = "ldap-search-base-dn"
    )]
    pub ldap_dn_pattern: Option<String>,
    /// LDAP DN used to bind during username searches.
    #[clap(long, requires_all = &["ldap-address", "ldap-search-password", "ldap-search-base-dn", "ldap-search-attribute-pattern"])]
    pub ldap_search_dn: Option<String>,
    /// LDAP password used to bind during username searches.
    #[clap(long, requires_all = &["ldap-address", "ldap-search-dn"])]
    pub ldap_search_password: Option<String>,
    /// LDAP base DN used during username searches.
    #[clap(long, requires = "ldap-address", conflicts_with = "ldap-dn-pattern")]
    pub ldap_search_base_dn: Option<String>,
    /// LDAP attribute(s) pattern used to match usernames during searches.
    #[clap(long, default_value = "(uid=%u)", requires_all = &["ldap-address", "ldap-search-base-dn"], conflicts_with = "ldap-dn-pattern")]
    pub ldap_search_attribute_pattern: String,
    /// CSS color used in the web UI.
    #[clap(short = 'T', long, default_value = "#15b154")]
    pub theme: String,
}

impl Options {
    pub fn validate(&self) {
        if (self.auth_upload || self.auth_download)
            && (self.credentials.is_empty() && self.ldap_address.is_none())
        {
            exit_error!(
                "At least one authentication method is required if you protect parts of the API"
            );
        }
        if self.username_origin && self.credentials.is_empty() && self.ldap_address.is_none() {
            exit_error!("At least one authentication method is required if you calculate quota using usernames")
        }
        if self.ldap_address.is_some()
            && self.ldap_dn_pattern.is_none()
            && self.ldap_search_base_dn.is_none()
        {
            exit_error!(
                "LDAP address is useless if ldap-dn-pattern and ldap-search-base-dn are empty"
            )
        }
    }

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

    pub fn ldap_authenticator(&self) -> Option<LdapAuthenticator> {
        let process = match (&self.ldap_dn_pattern, &self.ldap_search_base_dn) {
            (Some(dn_pattern), _) => LdapAuthProcess::SingleBind {
                dn_pattern: dn_pattern.clone(),
            },
            (_, Some(base_dn)) => LdapAuthProcess::SearchThenBind {
                search_credentials: self.ldap_search_dn.as_ref().and_then(|lsd| {
                    self.ldap_search_password
                        .as_ref()
                        .map(|lsp| (lsd.clone(), lsp.clone()))
                }),
                base_dn: base_dn.clone(),
                filter_pattern: self.ldap_search_attribute_pattern.clone(),
            },
            _ => return None,
        };
        Some(LdapAuthenticator::new(
            self.ldap_address.as_ref()?.clone(),
            process,
        ))
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
