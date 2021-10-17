use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;

use bitflags::bitflags;
use hyper::{Body, header, Request, Response};

use crate::error::auth as AuthError;
use crate::misc::generic_500;
use crate::response::adaptive_error;

pub struct Authenticator {
    access: Access,
    credentials: HashMap<String, String>,
}

bitflags! {
    pub struct Access: u32 {
        const UPLOAD = 1 << 0;
        const DOWNLOAD = 1 << 1;
        const WEB_UI = 1 << 2;
    }
}

impl Authenticator {
    pub fn new(access: Access, credentials: Vec<Credential>) -> Self {
        Self {
            access,
            credentials: credentials.into_iter().map(|Credential(u, p)| (u, p)).collect(),
        }
    }

    fn authorize_by_header(&self, header: &str) -> bool {
        let content = header.trim_start_matches("Basic ");
        let decoded = match base64::decode(content).map(|b| String::from_utf8(b).ok()).ok().flatten() {
            Some(decoded) => decoded,
            None => return false,
        };
        let [username, password]: [&str; 2] = match  decoded.split(':').collect::<Vec<_>>().try_into() {
            Ok(parts) => parts,
            Err(_) => return false,
        };
        if let Some(p) = self.credentials.get(username) {
            password == p
        } else {
            false
        }
    }

    pub fn allows(&self, req: &Request<Body>, access: Access) -> Option<Response<Body>> {
        if !self.access.contains(access) {
            return None
        }

        let response_type = req.headers().get(header::ACCEPT).cloned();
        if let Some(auth_header) = req.headers().get(header::AUTHORIZATION).map(|h| h.to_str().ok()).flatten() {
            if self.authorize_by_header(auth_header) {
                None
            } else {
                Some(
                    adaptive_error(response_type, AuthError::AccessForbidden)
                        .unwrap_or_else(|_| generic_500())
                )
            }
        } else {
            Some(
                Response::builder()
                    .status(AuthError::InvalidAuthorizationHeader.status_code())
                    .header(header::CONTENT_TYPE, "text/plain")
                    .header(header::WWW_AUTHENTICATE, "Basic")
                    .body(Body::from(AuthError::InvalidAuthorizationHeader.to_string()))
                    .unwrap_or_else(|_| generic_500())
            )
        }
    }
}

#[derive(Debug)]
pub struct Credential(String, String);

impl FromStr for Credential {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [username, password]: [&str; 2] = s.split(':')
            .collect::<Vec<_>>()
            .try_into().map_err(|_| "invalid format (should be USERNAME:PASSWORD)")?;

        Ok(Self(username.to_owned(), password.to_owned()))
    }
}