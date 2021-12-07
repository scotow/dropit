use std::collections::HashMap;
use std::str::FromStr;

use bitflags::bitflags;
use hyper::{Body, header, Request, Response};
use ldap3::{ldap_escape, LdapConnAsync, LdapError, Scope, SearchEntry};

use crate::error::auth as AuthError;
use crate::response::adaptive_error;
use crate::response::generic_500;

pub struct Authenticator {
    access: Access,
    credentials: HashMap<String, String>,
    ldap: Option<LdapAuthenticator>
}

bitflags! {
    pub struct Access: u8 {
        const UPLOAD = 1 << 0;
        const DOWNLOAD = 1 << 1;
        const WEB_UI = 1 << 2;
    }
}

impl Authenticator {
    pub fn new(access: Access, credentials: Vec<Credential>, ldap: Option<LdapAuthenticator>) -> Self {
        Self {
            access,
            credentials: credentials
                .into_iter()
                .map(|Credential(u, p)| (u, p))
                .collect(),
            ldap,
        }
    }

    pub async fn allows(&self, req: &Request<Body>, access: Access) -> Option<Response<Body>> {
        if !self.access.contains(access) {
            return None;
        }

        let response_type = req.headers().get(header::ACCEPT).cloned();
        if let Some(auth_header) = req
            .headers()
            .get(header::AUTHORIZATION)
            .map(|h| h.to_str().ok())
            .flatten()
        {
            if self.is_authorized_using_header(auth_header).await {
                None
            } else {
                Some(
                    adaptive_error(response_type, AuthError::AccessForbidden)
                        .unwrap_or_else(|_| generic_500()),
                )
            }
        } else {
            Some(
                Response::builder()
                    .status(AuthError::InvalidAuthorizationHeader.status_code())
                    .header(header::CONTENT_TYPE, "text/plain")
                    .header(header::WWW_AUTHENTICATE, "Basic")
                    .body(Body::from(
                        AuthError::InvalidAuthorizationHeader.to_string(),
                    ))
                    .unwrap_or_else(|_| generic_500()),
            )
        }
    }

    async fn is_authorized_using_header(&self, header: &str) -> bool {
        let content = header.trim_start_matches("Basic ");
        let decoded = match base64::decode(content)
            .map(|b| String::from_utf8(b).ok())
            .ok()
            .flatten()
        {
            Some(decoded) => decoded,
            None => return false,
        };
        let (username, password) = match decoded.split_once(':') {
            Some(parts) => parts,
            None => return false,
        };

        if let Some(p) = self.credentials.get(username) {
            return password == p
        }

        if let Some(ldap) = &self.ldap {
            return match ldap.is_authorized(username, password).await {
                Ok(success) => success,
                Err(err) => {
                    log::error!("Cannot authenticate user using LDAP: {:?}", err);
                    false
                }
            }
        }

        false
    }
}

#[derive(Debug)]
pub struct Credential(String, String);

impl FromStr for Credential {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (username, password) = s
            .split_once(':')
            .ok_or("invalid format (should be USERNAME:PASSWORD)")?;

        Ok(Self(username.to_owned(), password.to_owned()))
    }
}

pub struct LdapAuthenticator {
    address: String,
    search_credentials: Option<(String, String)>,
    base_dn: String,
    attribute: String,
}

impl LdapAuthenticator {
    #[allow(dead_code)]
    pub fn new(address: String, search_credentials: Option<(String, String)>, base_dn: String, attribute: String) -> Self {
        Self {
            address,
            search_credentials,
            base_dn,
            attribute,
        }
    }

    async fn is_authorized(&self, username: &str, password: &str) -> Result<bool, LdapError> {
        let (conn, mut ldap) = LdapConnAsync::new(&self.address).await?;
        ldap3::drive!(conn);

        if let Some((search_username, search_password)) = &self.search_credentials {
            ldap.simple_bind(search_username, search_password).await?;
        }
        let (mut entries, _res) = ldap.search(
            &self.base_dn,
            Scope::Subtree,
            &format!("({}={})", &self.attribute, ldap_escape(username)),
            vec![""],
        ).await?.success()?;

        if entries.len() != 1 {
            return Ok(false);
        }
        let entry = SearchEntry::construct(entries.pop().unwrap());
        let res = ldap.simple_bind(&entry.dn, password).await?;

        Ok(res.rc != 49)
    }
}
