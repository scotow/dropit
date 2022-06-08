use std::collections::HashMap;

use hyper::http::HeaderValue;
use hyper::{header, Body, Request, Response};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::Credential;
use crate::error::auth as AuthError;
use crate::misc::header_str;
use crate::response::adaptive_error;
use crate::response::generic_500;
use crate::{Features, LdapAuthenticator};

pub enum AuthStatus {
    NotNeeded,
    Valid(String),
    Error(Response<Body>),
}

pub struct Authenticator {
    protected: Features,
    static_credentials: HashMap<String, String>,
    ldap: Option<LdapAuthenticator>,
    sessions: RwLock<HashMap<String, String>>,
}

impl Authenticator {
    pub fn new(
        protected: Features,
        credentials: Vec<Credential>,
        ldap: Option<LdapAuthenticator>,
    ) -> Self {
        Self {
            protected,
            static_credentials: credentials
                .into_iter()
                .map(|Credential(u, p)| (u, p))
                .collect(),
            ldap,
            sessions: Default::default(),
        }
    }

    pub fn protects(&self, feature: Features) -> bool {
        self.protected.contains(feature)
    }

    pub async fn allows(&self, req: &Request<Body>, feature: Features) -> AuthStatus {
        if !self.protected.contains(feature) {
            return AuthStatus::NotNeeded;
        }

        match self.verify_authorization_header(req).await {
            Ok(Some(username)) => return AuthStatus::Valid(username),
            Err(err) => return AuthStatus::Error(err),
            Ok(None) => (),
        };

        if let Some(username) = self.verify_cookie(req).await {
            return AuthStatus::Valid(username);
        }

        AuthStatus::Error(
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

    async fn verify_authorization_header(
        &self,
        req: &Request<Body>,
    ) -> Result<Option<String>, Response<Body>> {
        let header = match header_str(req, header::AUTHORIZATION) {
            Some(header) => header,
            None => return Ok(None),
        };

        let response_type = req.headers().get(header::ACCEPT).cloned();
        let content = header.trim_start_matches("Basic ");
        let decoded = match base64::decode(content)
            .map(|b| String::from_utf8(b).ok())
            .ok()
            .flatten()
        {
            Some(decoded) => decoded,
            None => return Err(forbidden_error_response(response_type)),
        };
        let (username, password) = match decoded.split_once(':') {
            Some(parts) => parts,
            None => return Err(forbidden_error_response(response_type)),
        };

        Ok(self.verify_credentials(username, password).await)
    }

    pub async fn verify_cookie(&self, req: &Request<Body>) -> Option<String> {
        let header = header_str(req, header::COOKIE)?;
        let session = header
            .split("; ")
            .filter_map(|p| p.split_once('='))
            .find_map(|(k, v)| (k == "session").then(|| v))?;

        self.sessions.read().await.get(session).cloned()
    }

    async fn verify_credentials(&self, username: &str, password: &str) -> Option<String> {
        if let Some(p) = self.static_credentials.get(username) {
            return (password == p).then(|| username.to_owned());
        }

        if let Some(ldap) = &self.ldap {
            return match ldap.is_authorized(username, password).await {
                Ok(success) => success.then(|| username.to_owned()),
                Err(err) => {
                    log::error!("Cannot authenticate user using LDAP: {:?}", err);
                    None
                }
            };
        }

        None
    }

    pub async fn create_session(
        &self,
        username: &str,
        password: &str,
        response_type: Option<HeaderValue>,
    ) -> Result<String, Response<Body>> {
        if self.verify_credentials(username, password).await.is_none() {
            return Err(forbidden_error_response(response_type));
        }

        let token = Uuid::new_v4().to_hyphenated_ref().to_string();
        self.sessions
            .write()
            .await
            .insert(token.clone(), username.to_owned());
        Ok(token)
    }
}

fn forbidden_error_response(response_type: Option<HeaderValue>) -> Response<Body> {
    adaptive_error(response_type, AuthError::AccessForbidden).unwrap_or_else(|_| generic_500())
}
