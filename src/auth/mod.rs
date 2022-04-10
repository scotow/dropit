pub use authenticator::Authenticator;
pub use credential::Credential;
pub use features::Features;
pub use ldap::LdapAuthenticator;

mod authenticator;
mod credential;
mod features;
mod ldap;

pub mod upload_requires_auth {
    use crate::response::json_response;
    use crate::{Authenticator, Error as AuthError, Error, Features};
    use hyper::{Body, Request, Response, StatusCode};
    use routerify::ext::RequestExt;
    use serde::Serialize;

    #[derive(Serialize)]
    struct RequiresAuth {
        pub required: bool,
    }

    pub async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
        let auth = req.data::<Authenticator>().ok_or(AuthError::AuthProcess)?;
        if !auth.protects(Features::UPLOAD) {
            return Ok(json_response(
                StatusCode::OK,
                RequiresAuth { required: false },
            )?);
        }

        Ok(json_response(
            StatusCode::OK,
            RequiresAuth {
                required: !auth.verify_cookie(&req),
            },
        )?)
    }
}
