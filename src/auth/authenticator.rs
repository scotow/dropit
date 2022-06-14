use axum::headers::authorization::Basic;
use axum::headers::{Authorization, Cookie};
use std::collections::HashMap;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::Credential;
use crate::auth::{Features, LdapAuthenticator};
use crate::error::{auth as AuthError, Error};

pub enum AuthStatus {
    NotNeeded,
    Prompt,
    Valid(String),
    Error(Error),
}

enum AuthProcess {
    Valid(String),
    Continue,
    Stop,
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

    pub async fn allows(
        &self,
        authorization: Option<Authorization<Basic>>,
        cookie: Option<Cookie>,
        feature: Features,
    ) -> AuthStatus {
        if !self.protected.contains(feature) {
            return AuthStatus::NotNeeded;
        }

        match self.verify_authorization_header(authorization).await {
            AuthProcess::Valid(username) => return AuthStatus::Valid(username),
            AuthProcess::Continue => (),
            AuthProcess::Stop => return AuthStatus::Error(AuthError::AccessForbidden),
        }

        match self.verify_cookie(cookie).await {
            AuthProcess::Valid(username) => return AuthStatus::Valid(username),
            AuthProcess::Continue => (),
            AuthProcess::Stop => return AuthStatus::Error(AuthError::AccessForbidden),
        }

        AuthStatus::Prompt
    }

    async fn verify_authorization_header(
        &self,
        header: Option<Authorization<Basic>>,
    ) -> AuthProcess {
        let header = match header {
            Some(header) => header,
            None => return AuthProcess::Continue,
        };
        self.verify_credentials(header.username(), header.password())
            .await
    }

    async fn verify_cookie(&self, cookie: Option<Cookie>) -> AuthProcess {
        let cookie = match cookie {
            Some(cookie) => cookie,
            None => return AuthProcess::Continue,
        };
        let session = match cookie.get("session") {
            Some(session) => session,
            None => return AuthProcess::Continue,
        };
        match self.sessions.read().await.get(session).cloned() {
            Some(username) => AuthProcess::Valid(username),
            None => AuthProcess::Stop,
        }
    }

    async fn verify_credentials(&self, username: &str, password: &str) -> AuthProcess {
        if let Some(p) = self.static_credentials.get(username) {
            return if password == p {
                AuthProcess::Valid(username.to_owned())
            } else {
                AuthProcess::Stop
            };
        }

        if let Some(ldap) = &self.ldap {
            return match ldap.is_authorized(username, password).await {
                Ok(true) => AuthProcess::Valid(username.to_owned()),
                Ok(false) => AuthProcess::Stop,
                Err(err) => {
                    log::error!("Cannot authenticate user using LDAP: {:?}", err);
                    AuthProcess::Stop
                }
            };
        }

        AuthProcess::Stop
    }

    pub async fn create_session(&self, username: &str, password: &str) -> Result<String, Error> {
        match self.verify_credentials(username, password).await {
            AuthProcess::Valid(_) => (),
            AuthProcess::Continue | AuthProcess::Stop => return Err(AuthError::AccessForbidden),
        };

        let token = Uuid::new_v4().to_hyphenated_ref().to_string();
        self.sessions
            .write()
            .await
            .insert(token.clone(), username.to_owned());
        Ok(token)
    }
}
