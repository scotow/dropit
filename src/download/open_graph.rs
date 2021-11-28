use byte_unit::Byte;
use hyper::{Body, Request, StatusCode};
use hyper::header::{CONTENT_TYPE, USER_AGENT};

use crate::{Error, Response};
use crate::download::FileInfo;

const BOTS: &[&'static str] = &["discord", "facebook", "twitter"];

pub(super) fn proxy_request(
    req: &Request<Body>,
    files_info: &[FileInfo],
) -> Option<Response<Body>> {
    if matches!(
        req.uri().query().map(|q| q.contains("force-download=true")),
        Some(true)
    ) {
        return None;
    }

    if let Some(Ok(user_agent)) = req
        .headers()
        .get(USER_AGENT)
        .map(|h| h.to_str().map(|h| h.to_lowercase()))
    {
        if BOTS.iter().all(|bot| !user_agent.contains(bot)) {
            return None;
        }
    } else {
        return None;
    }

    let title = match files_info.len() {
        0 => return None,
        1 => "Download File",
        _ => "Download Archive",
    };

    let description = files_info
        .iter()
        .map(|info| {
            format!(
                "{} ({})",
                info.name,
                Byte::from_bytes(info.size as u64)
                    .get_appropriate_unit(false)
                    .to_string()
            )
        })
        .collect::<Vec<_>>();
    let page = include_str!("redirect.html")
        .replacen("$TITLE", title, 1)
        .replacen("$DESCRIPTION", &description.join("&#10;&#13;"), 1);

    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/html")
            .body(Body::from(page))
            .ok()?,
    )
}
