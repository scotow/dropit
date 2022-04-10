pub use authenticator::Authenticator;
pub use credential::Credential;
pub use features::Features;
pub use ldap::LdapAuthenticator;

mod authenticator;
mod credential;
mod features;
mod ldap;

pub mod upload_requires_auth {
    use std::sync::Arc;

    use hyper::{Body, Request, Response, StatusCode};
    use routerify::ext::RequestExt;
    use serde::Serialize;

    use crate::response::json_response;
    use crate::{Authenticator, Error as AuthError, Error, Features};

    #[derive(Serialize)]
    struct RequiresAuth {
        pub required: bool,
    }

    pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
        let auth = req
            .data::<Arc<Authenticator>>()
            .ok_or(AuthError::AuthProcess)?;
        if !auth.protects(Features::UPLOAD) {
            return Ok(json_response(
                StatusCode::OK,
                RequiresAuth { required: false },
            )?);
        }

        Ok(json_response(
            StatusCode::OK,
            RequiresAuth {
                required: !auth.verify_cookie(&req).await,
            },
        )?)
    }
}

pub mod login {
    use std::sync::Arc;

    use hyper::body::HttpBody;
    use hyper::{header, Body, Request, Response, StatusCode};
    use routerify::ext::RequestExt;
    use serde::Deserialize;
    use serde::Serialize;

    use crate::response::{json_error, json_response};
    use crate::{Authenticator, Error as AuthError, Error};

    #[derive(Deserialize)]
    struct LoginRequest {
        pub username: String,
        pub password: String,
    }

    #[derive(Serialize)]
    struct LoginResponse {
        pub token: String,
    }

    pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
        if req.size_hint().upper().unwrap_or(u64::MAX) > 2048 {
            return Ok(json_error(AuthError::AuthProcess)?);
        }

        let response_type = req.headers().get(header::ACCEPT).cloned();

        let auth = req
            .data::<Arc<Authenticator>>()
            .ok_or(AuthError::AuthProcess)?
            .clone();

        let body = hyper::body::to_bytes(req.into_body()).await?;
        let login_request =
            serde_json::from_slice::<LoginRequest>(&body).map_err(|_| AuthError::AuthProcess)?;

        let token = match auth
            .create_session(
                &login_request.username,
                &login_request.password,
                response_type,
            )
            .await
        {
            Ok(token) => token,
            Err(resp) => return Ok(resp),
        };

        Ok(json_response(StatusCode::CREATED, LoginResponse { token })?)
    }
}
