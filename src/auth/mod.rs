use std::sync::Arc;

use axum::routing::get;
use axum::{Extension, Router};

pub use authenticator::AuthStatus;
pub use authenticator::Authenticator;
pub use credential::Credential;
pub use features::Features;
pub use ldap::{LdapAuthProcess, LdapAuthenticator};
pub use origin::Origin;

mod authenticator;
mod credential;
mod features;
mod ldap;
mod login;
mod origin;
mod protection;

pub fn router(authenticator: Arc<Authenticator>) -> Router {
    Router::new()
        .route("/auth", get(protection::handler).post(login::handler))
        .route_layer(Extension(authenticator))
}
