use crate::alias::Alias;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

#[derive(PartialEq, Deserialize, Debug)]
struct Command {
    action: Action,
    files: Vec<File>,
}

#[derive(PartialEq, Deserialize, Debug)]
struct File {
    alias: Alias,
    admin: String,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Action {
    Subscribe,
    Unsubscribe,
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Deserialize::deserialize(deserializer)? {
            "subscribe" => Self::Subscribe,
            "unsubscribe" => Self::Unsubscribe,
            other => return Err(Error::unknown_variant(other, &["subscribe", "unsubscribe"])),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Action;
    use super::Command;
    use super::File;
    use crate::alias::Alias;

    #[test]
    fn deserialize_action() {
        assert_eq!(
            serde_json::from_str::<Action>("\"subscribe\"").unwrap(),
            Action::Subscribe
        );
        assert_eq!(
            serde_json::from_str::<Action>("\"unsubscribe\"").unwrap(),
            Action::Unsubscribe
        );
        assert!(serde_json::from_str::<Action>("\"eat\"").is_err());
    }

    #[test]
    fn deserialize_command() {
        assert_eq!(
            serde_json::from_str::<Command>(
                r#"
            {
                "action": "subscribe",
                "files": [
                    {
                        "alias": "abc-def-ghi",
                        "admin": "token"
                    },
                    {
                        "alias": "abcdef",
                        "admin": "token"
                    }
                ]
            }
        "#
            )
            .unwrap(),
            Command {
                action: Action::Subscribe,
                files: vec![
                    File {
                        alias: Alias::Long("abc-def-ghi".to_owned()),
                        admin: "token".to_owned(),
                    },
                    File {
                        alias: Alias::Short("abcdef".to_owned()),
                        admin: "token".to_owned(),
                    }
                ]
            }
        );
    }
}
