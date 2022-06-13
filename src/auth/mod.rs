pub use authenticator::AuthStatus;
pub use authenticator::Authenticator;
use axum::routing::get;
use axum::{Extension, Router};
pub use credential::Credential;
pub use features::Features;
pub use ldap::LdapAuthenticator;
pub use origin::Origin;
use std::sync::Arc;

mod authenticator;
mod credential;
mod features;
mod ldap;
mod login;
mod origin;
mod protection;

pub fn router(authenticator: Authenticator) -> Router {
    Router::new()
        .route("/auth", get(protection::handler).post(login::handler))
        .layer(Extension(Arc::new(authenticator)))
}
