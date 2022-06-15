use axum::response::{Html, IntoResponse, Response};
use byte_unit::Byte;

use crate::download::FileInfo;

const BOTS: &[&str] = &["discord", "facebook", "twitter"];

pub(super) fn proxy_request(user_agent: String, files_info: &[FileInfo]) -> Option<Response> {
    if BOTS.iter().all(|bot| !user_agent.contains(bot)) {
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
                Byte::from_bytes(info.size as u64).get_appropriate_unit(false)
            )
        })
        .collect::<Vec<_>>();
    let page = include_str!("redirect.html")
        .replacen("$TITLE", title, 1)
        .replacen("$DESCRIPTION", &description.join("&#10;&#13;"), 1);

    Some(Html(page).into_response())
}
