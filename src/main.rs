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
    use std::{net::SocketAddr, sync::Arc, time::Duration};

    use axum::Router;
    use clap::Parser;
    use http_negotiator::{ContentTypeNegotiation, Negotiator};
    use hyper::Server;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use crate::{
        auth::Authenticator,
        exit_error, include_query,
        limit::{Chain as LimiterChain, Global as GlobalLimiter, Origin as OriginLimiter},
        options::Options,
        response::ResponseType,
        storage::{Cleaner, Dir},
        upload::{Determiner, RealIp},
    };

    pub(super) async fn run() {
        let options = Options::parse();
        env_logger::Builder::new()
            .filter_level(options.log_level)
            .init();

        let limiters = LimiterChain::new(vec![
            Box::new(OriginLimiter::new(
                options.origin_size_sum,
                options.origin_file_count,
            )),
            Box::new(GlobalLimiter::new(options.global_size_sum)),
        ]);
        let determiner = Arc::new(
            Determiner::new(options.thresholds.clone())
                .unwrap_or_else(|err| exit_error!("Invalid thresholds: {}", err)),
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
            .unwrap_or_else(|err| exit_error!("Cannot create database pool: {}", err));
        sqlx::query(include_query!("migration"))
            .execute(&pool)
            .await
            .unwrap_or_else(|err| exit_error!("Cannot run migration query: {}", err));

        let dir = Dir::new(options.uploads_dir.clone());
        dir.create(!options.no_uploads_dir_creation)
            .await
            .unwrap_or_else(|err| exit_error!("{}", err));

        let cleaner = Cleaner::new(dir.clone(), pool.clone());
        tokio::task::spawn(async move {
            cleaner.start().await;
        });

        let authenticator = Arc::new(Authenticator::new(
            options.access(),
            options.credentials.clone(),
            options.ldap_authenticator(),
        ));

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
            .merge(super::info::router(pool.clone()))
            .route_layer(
                Negotiator::<ContentTypeNegotiation, _>::new([
                    ResponseType::Json,
                    ResponseType::Text,
                ])
                .unwrap_or_else(|err| exit_error!("Invalid mime types: {}", err)),
            );

        let address = SocketAddr::new(options.address, options.port);
        log::info!("App is running on: {}", address);
        Server::bind(&address)
            .http1_title_case_headers(true)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap_or_else(|err| exit_error!("Server stopped: {}", err))
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    main::run().await
}
