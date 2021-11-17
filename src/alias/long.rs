use std::convert::TryInto;

use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;

lazy_static! {
    static ref WORDS: [&'static str; 144] = {
        include_str!("words.txt")
            .lines()
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    };
    static ref REGEX: Regex = Regex::new("^[a-z]{3,}(?:-[a-z]{3,}){2}$").unwrap();
}

pub fn is_match(alias: &str) -> bool {
    REGEX.is_match(alias)
}

pub fn random() -> Option<String> {
    let chosen = WORDS
        .choose_multiple(&mut thread_rng(), 3)
        .copied()
        .collect::<Vec<_>>();

    if chosen.len() == 3 {
        Some(chosen.join("-"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    fn has_no_repetition(alias: &str) -> bool {
        let parts = alias.split('-').collect::<Vec<_>>();
        if parts.len() != 3 {
            return false;
        }
        if parts[0] == parts[1] || parts[0] == parts[2] || parts[1] == parts[2] {
            return false;
        }
        true
    }

    #[test]
    fn random() {
        [
            "boat-surface-soon",
            "way-finish-then",
            "one-dark-these",
            "slow-while-stand",
        ]
        .iter()
        .for_each(|a| assert!(super::REGEX.is_match(a)));

        ["hello", "hello-world", "hi-world-home"]
            .iter()
            .for_each(|a| assert!(!super::REGEX.is_match(a)));

        for _ in 0..1000 {
            let alias = super::random();
            assert!(alias.is_some());
            assert!(super::REGEX.is_match(&alias.unwrap()));
            assert!(super::random()
                .map(|a| has_no_repetition(&a))
                .unwrap_or(false));
        }
    }
}
