use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashSet;
use std::convert::TryInto;
use regex::Regex;
use std::str;

lazy_static! {
    static ref CHARS: &'static[u8; 55] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    static ref REGEX: Regex = Regex::new(
        &format!("^[{}]{{6}}$", str::from_utf8(*CHARS).unwrap())
    ).unwrap();
}

pub fn is_match(alias: &str) -> bool {
    REGEX.is_match(alias)
}

pub fn random() -> Option<String> {
    let mut rng = thread_rng();
    let mut alias = String::with_capacity(6);
    for _ in 0..6 {
        alias.push(CHARS.choose(&mut rng).map(|&c| c as char)?);
    }
    Some(alias)
}

#[cfg(test)]
mod tests {
    #[test]
    fn random() {
        [
            "nXL4fq",
            "hT8cFn",
            "bEC9v8",
            "aBvyRK"
        ]
            .iter()
            .for_each(|a| assert!(super::REGEX.is_match(a)));

        [
            "AAAAA",
            "AAAAAAA",
            "iAAAAAA",
            "0AAAAAA"
        ]
            .iter()
            .for_each(|a| assert!(!super::REGEX.is_match(a)));

        for _ in 0..1000 {
            let alias = super::random();
            assert!(alias.is_some());
            assert!(super::REGEX.is_match(&alias.unwrap()));
        }
    }
}