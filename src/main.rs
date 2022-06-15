mod alias;
mod assets;
mod auth;
mod download;
mod error;
mod info;
mod limit;
mod misc;
mod options;
mod query;
mod response;
mod storage;
mod theme;
mod update;
mod upload;

mod main {
    use std::net::SocketAddr;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    use axum::Router;
    use clap::Parser;
    use hyper::Server;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tokio::fs::File;
    use tokio::io::ErrorKind;

    use crate::auth::{Authenticator, LdapAuthenticator};
    use crate::limit::global::Global;
    use crate::limit::origin::Origin as OriginLimiter;
    use crate::limit::Chain as LimiterChain;
    use crate::options::Options;
    use crate::storage::clean::Cleaner;
    use crate::storage::dir::Dir;
    use crate::upload::expiration::Determiner;
    use crate::upload::origin::RealIp;
    use crate::{exit_error, include_query};

    pub(super) async fn run() {
        let options = Options::parse();
        env_logger::Builder::new()
            .filter_level(options.log_level)
            .init();
        options.validate();

        let limiters = LimiterChain::new(vec![
            Box::new(OriginLimiter::new(
                options.origin_size_sum,
                options.origin_file_count,
            )),
            Box::new(Global::new(options.global_size_sum)),
        ]);
        let determiner = Arc::new(
            Determiner::new(options.thresholds.clone())
                .unwrap_or_else(|| exit_error!("Invalid thresholds")),
        );

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&options.database)
                    .create_if_missing(!options.no_database_creation)
                    .busy_timeout(Duration::from_secs(30)),
            )
            .await
            .unwrap_or_else(|e| exit_error!("Cannot create database pool: {}", e));
        sqlx::query(include_query!("migration"))
            .execute(&pool)
            .await
            .unwrap_or_else(|e| exit_error!("Cannot run migration query: {}", e));

        create_uploads_dir(&options.uploads_dir, !options.no_uploads_dir_creation).await;
        let cleaner = Cleaner::new(&options.uploads_dir, pool.clone());
        tokio::task::spawn(async move {
            cleaner.start().await;
        });

        let ldap = if let (Some(ldap_address), Some(ldap_base_dn)) =
            (options.ldap_address.as_ref(), options.ldap_base_dn.as_ref())
        {
            Some(LdapAuthenticator::new(
                ldap_address.clone(),
                options.ldap_search_dn.as_ref().and_then(|lsd| {
                    options
                        .ldap_search_password
                        .as_ref()
                        .map(|lsp| (lsd.clone(), lsp.clone()))
                }),
                ldap_base_dn.clone(),
                options.ldap_attribute.clone(),
            ))
        } else {
            None
        };

        let authenticator = Arc::new(Authenticator::new(
            options.access(),
            options.credentials.clone(),
            ldap,
        ));

        let dir = Dir::new(options.uploads_dir.clone());
        let router = Router::new()
            .merge(super::assets::router())
            .merge(super::theme::router(&options.theme))
            .merge(super::auth::router(Arc::clone(&authenticator)))
            .merge(super::upload::router(
                pool.clone(),
                Arc::clone(&authenticator),
                RealIp::new(options.behind_proxy),
                options
                    .origin()
                    .unwrap_or_else(|| exit_error!("Invalid origin method")),
                limiters,
                Arc::clone(&determiner),
                dir.clone(),
            ))
            .merge(super::download::router(
                pool.clone(),
                Arc::clone(&authenticator),
                dir.clone(),
            ))
            .merge(super::update::router(
                pool.clone(),
                dir.clone(),
                Arc::clone(&determiner),
            ))
            .merge(super::info::router(pool.clone()));

        let address = SocketAddr::new(options.address, options.port);
        log::info!("App is running on: {}", address);
        Server::bind(&address)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap_or_else(|e| exit_error!("Server stopped: {}", e))
    }

    async fn create_uploads_dir(path: &Path, should_create: bool) {
        match File::open(&path).await {
            Ok(fd) => match fd.metadata().await {
                Ok(md) => {
                    if !md.is_dir() {
                        exit_error!("Uploads path is not a directory");
                    }
                }
                Err(_) => {
                    exit_error!("Cannot fetch uploads dir metadata");
                }
            },
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    if should_create {
                        if let Err(err) = tokio::fs::create_dir_all(&path).await {
                            exit_error!("Cannot create uploads directory: {}", err);
                        }
                    } else {
                        exit_error!("Uploads directory not found");
                    }
                } else {
                    exit_error!("Cannot open uploads directory");
                }
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    main::run().await
}
