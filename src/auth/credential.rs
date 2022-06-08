use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Credential(pub String, pub String);

impl FromStr for Credential {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (username, password) = s
            .split_once(':')
            .ok_or("invalid format (should be USERNAME:PASSWORD)")?;

        Ok(Self(username.to_owned(), password.to_owned()))
    }
}
