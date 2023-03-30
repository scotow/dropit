use std::collections::HashMap;

use itertools::Itertools;
use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::response::{ApiHeader, SingleLine};

pub(super) mod both;
pub(super) mod long;
pub(super) mod short;

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
        let mut aliases = HashMap::with_capacity(2);
        let mut links = HashMap::with_capacity(2);
        if let Some((alias, link)) = &self.short {
            aliases.insert("short".to_owned(), alias.to_owned());
            links.insert("short".to_owned(), link.to_owned());
        }
        if let Some((alias, link)) = &self.long {
            aliases.insert("long".to_owned(), alias.to_owned());
            links.insert("long".to_owned(), link.to_owned());
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
