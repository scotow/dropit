use hyper::{Body, Request, Response, StatusCode};
use itertools::Itertools;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::{json, Map, Value};

use crate::error::alias as AliasError;
use crate::error::Error;
use crate::misc::request_target;
// use crate::response::json_response;
use crate::response::{ApiHeader, SingleLine};
use crate::{alias, include_query};

pub mod both;
pub mod long;
pub mod short;

pub struct AliasChange {
    pub(crate) short: Option<(String, String)>,
    pub(crate) long: Option<(String, String)>,
}

impl ApiHeader for AliasChange {}

impl Serialize for AliasChange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AliasChange", 2)?;
        let mut aliases = Map::with_capacity(2);
        let mut links = Map::with_capacity(2);
        if let Some((alias, link)) = &self.short {
            aliases.insert("short".to_owned(), alias.to_owned().into());
            links.insert("short".to_owned(), link.to_owned().into());
        }
        if let Some((alias, link)) = &self.long {
            aliases.insert("long".to_owned(), alias.to_owned().into());
            links.insert("long".to_owned(), link.to_owned().into());
        }
        state.serialize_field("alias", &aliases)?;
        state.serialize_field("link", &links)?;
        state.end()
    }
}

impl SingleLine for AliasChange {
    fn single_lined(&self) -> String {
        self.short
            .iter()
            .chain(self.long.iter())
            .map(|(_, link)| link)
            .join(" ")
    }
}

// pub async fn handler_long(req: Request<Body>) -> Result<Response<Body>, Error> {
//     Ok(json_response(
//         StatusCode::OK,
//         process_long(req).await.map(|(base, alias)| {
//             json!({
//                 "alias": { "long": &alias },
//                 "link": { "long": format!("{}/{}", base, &alias) }
//             })
//         })?,
//     )?)
// }
//
// pub async fn handler_both(req: Request<Body>) -> Result<Response<Body>, Error> {
//     Ok(json_response(
//         StatusCode::OK,
//         process_both(req).await.map(|(base, short, long)| {
//             json!({
//                 "alias": {
//                     "short": &short,
//                     "long": &long,
//                 },
//                 "link": {
//                     "short": format!("{}/{}", base, &short),
//                     "long": format!("{}/{}", base, &long),
//                 }
//             })
//         })?,
//     )?)
// }

// async fn process_long(req: Request<Body>) -> Result<(String, String), Error> {
//     let (id, _size, mut conn) = super::authorize(&req).await?;
//     let alias = alias::random_unused_long(&mut conn)
//         .await
//         .ok_or(AliasError::AliasGeneration)?;
//
//     let affected = sqlx::query(include_query!("update_file_long_alias"))
//         .bind(&alias)
//         .bind(&id)
//         .execute(&mut conn)
//         .await
//         .map_err(|_| AliasError::Database)?
//         .rows_affected();
//
//     if affected != 1 {
//         return Err(AliasError::UnexpectedFileModification);
//     }
//
//     let base = request_target(req.headers()).ok_or(AliasError::Target)?;
//     Ok((base, alias))
// }
//
// async fn process_both(req: Request<Body>) -> Result<(String, String, String), Error> {
//     let (id, _size, mut conn) = super::authorize(&req).await?;
//     let (short, long) = alias::random_unused_aliases(&mut conn)
//         .await
//         .ok_or(AliasError::AliasGeneration)?;
//
//     let affected = sqlx::query(include_query!("update_file_aliases"))
//         .bind(&short)
//         .bind(&long)
//         .bind(&id)
//         .execute(&mut conn)
//         .await
//         .map_err(|_| AliasError::Database)?
//         .rows_affected();
//
//     if affected != 1 {
//         return Err(AliasError::UnexpectedFileModification);
//     }
//
//     let base = request_target(req.headers()).ok_or(AliasError::Target)?;
//     Ok((base, short, long))
// }
