use std::str::FromStr;
use crate::alias::Alias::{Short, Long};

pub mod short;
pub mod long;

#[derive(Clone, Debug, PartialEq)]
pub enum Alias {
    Short(String),
    Long(String),
}

impl Alias {
    pub fn inner(&self) -> &str {
        match self {
            Short(a) => &a,
            Long(a) => &a,
        }
    }
}

impl FromStr for Alias {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if short::is_match(s) {
            Ok(Short(s.to_owned()))
        } else if long::is_match(s) {
            Ok(Long(s.to_owned()))
        } else {
            Err(())
        }
    }
}

pub fn random_aliases() -> Option<(String, String)> {
    short::random().and_then(|s| long::random().map(|l| (s, l)))
}