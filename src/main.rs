use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use hyper::{header, Body, Response, Server};
use routerify::{RequestInfo, Router, RouterService};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::io::ErrorKind;

use crate::assets::Assets;
use crate::auth::{Authenticator, Features, LdapAuthenticator};
use crate::error::{assets as AssetsError, Error};
use crate::limit::global::Global;
use crate::limit::ip::Ip as IpLimiter;
use crate::limit::Chain as LimiterChain;
use crate::options::Options;
use crate::response::adaptive_error;
use crate::response::generic_500;
use crate::storage::clean::Cleaner;
use crate::storage::dir::Dir;
use crate::upload::expiration::Determiner;
use crate::upload::origin::RealIp;

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
mod update;
mod upload;

fn router(
    uploads_dir: PathBuf,
    real_ip: RealIp,
    limiters: LimiterChain,
    determiner: Determiner,
    pool: SqlitePool,
    assets: Assets,
    auth: Authenticator,
) -> Router<Body, Error> {
    Router::builder()
        .data(Dir::new(uploads_dir))
        .data(real_ip)
        .data(limiters)
        .data(determiner)
        .data(pool)
        .data(assets)
        .data(auth)
        .get("/", assets::handler)
        .get("/index.html", assets::handler)
        .get("/style.css", assets::handler)
        .get("/app.js", assets::handler)
        .get("/icon.png", assets::handler)
        .get("/login/", assets::handler)
        .get("/login/index.html", assets::handler)
        .get("/login/app.js", assets::handler)
        .get("/auth", auth::upload_requires_auth::handler)
        .get("/:alias", download::handler)
        .post("/", upload::handler)
        .post("/upload", upload::handler)
        .delete("/:alias", update::revoke::handler)
        .patch("/:alias/aliases", update::alias::handler_both)
        .patch("/:alias/aliases/short", update::alias::handler_short)
        .patch("/:alias/aliases/long", update::alias::handler_long)
        .patch("/:alias/expiration", update::expiration::handler)
        .patch("/:alias/downloads/:count", update::downloads::handler)
        .get("/valids/:alias", info::valid::handler)
        .err_handler_with_info(error_handler)
        .build()
        .unwrap_or_else(|_| exit_error!("Cannot create HTTP router"))
}

async fn error_handler(error: routerify::RouteError, req_info: RequestInfo) -> Response<Body> {
    let error = match error.downcast::<Error>() {
        Ok(error) => error,
        Err(_) => return generic_500(),
    };
    let response_type = req_info.headers().get(header::ACCEPT).cloned();
    adaptive_error(response_type, *error).unwrap_or_else(|_| generic_500())
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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let options = Options::from_args();
    env_logger::Builder::new()
        .filter_level(options.log_level)
        .init();

    let limiters = LimiterChain::new(vec![
        Box::new(IpLimiter::new(options.ip_size_sum, options.ip_file_count)),
        Box::new(Global::new(options.global_size_sum)),
    ]);
    let determiner =
        Determiner::new(options.thresholds).unwrap_or_else(|| exit_error!("Invalid thresholds"));

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

    let mut access = Features::empty();
    if options.auth_upload {
        access.insert(Features::UPLOAD);
    }
    if options.auth_download {
        access.insert(Features::DOWNLOAD);
    }

    let ldap = if let (Some(ldap_address), Some(ldap_base_dn)) =
        (options.ldap_address, options.ldap_base_dn)
    {
        Some(LdapAuthenticator::new(
            ldap_address,
            options
                .ldap_search_dn
                .and_then(|lsd| options.ldap_search_password.map(|lsp| (lsd, lsp))),
            ldap_base_dn,
            options.ldap_attribute,
        ))
    } else {
        None
    };

    let router = router(
        options.uploads_dir,
        RealIp::new(options.behind_proxy),
        limiters,
        determiner,
        pool,
        Assets::new(options.theme),
        Authenticator::new(access, options.credentials, ldap),
    );

    let address = SocketAddr::new(options.address, options.port);
    let service = RouterService::new(router)
        .unwrap_or_else(|e| exit_error!("Cannot create HTTP service: {}", e));

    log::info!("App is running on: {}", address);
    Server::bind(&address)
        .serve(service)
        .await
        .unwrap_or_else(|e| exit_error!("Server stopped: {}", e))
}
