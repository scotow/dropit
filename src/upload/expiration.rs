use std::time::Duration;
use std::str::FromStr;
use std::convert::TryInto;
use byte_unit::Byte;

#[derive(Clone, Debug)]
pub struct Threshold {
    pub size: u64,
    pub duration: Duration,
}

impl FromStr for Threshold {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [size, duration]: [&str; 2] = s.splitn(2, ':')
            .collect::<Vec<_>>()
            .try_into().map_err(|_| "invalid format (should be SIZE:DURATION_SEC)")?;
        
        Ok(Threshold {
            size: size.parse::<Byte>().map_err(|_| "invalid size")?.get_bytes(),
            duration: Duration::from_secs(
                duration.parse::<u64>()
                    .map_err(|_| "invalid duration")?
            ),
        })
    }
}

pub struct Determiner(Vec<Threshold>);

impl Determiner {
    pub fn new(thresholds: Vec<Threshold>) -> Option<Self> {
        if thresholds.is_empty() {
            return None
        }
        for couple in thresholds.windows(2) {
            if couple[0].size > couple[1].size || couple[0].duration < couple[1].duration {
                return None;
            }
        }
        Some(Self(thresholds))
    }

    pub fn determine(&self, size: u64) -> Option<Duration> {
        self.0.iter()
            .find(|t| size <= t.size)
            .map(|t| t.duration)
    }
}

#[cfg(test)]
mod tests {
    use crate::upload::expiration::{Threshold, Determiner};
    use std::time::Duration;

    #[test]
    fn determiner() {
        let determiner = Determiner::new(
            vec![
                Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) }
            ]
        );
        assert!(determiner.is_some());

        let determiner = Determiner::new(
            vec![
                Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) },
                Threshold { size: 256 * 1024 * 1024, duration: Duration::from_secs(6 * 60 * 60) }
            ]
        );
        assert!(determiner.is_some());

        let determiner = Determiner::new(
            vec![
                Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) },
                Threshold { size: 256 * 1024 * 1024, duration: Duration::from_secs(48 * 60 * 60) }
            ]
        );
        assert!(determiner.is_none());

        let determiner = Determiner::new(Vec::new());
        assert!(determiner.is_none());

        let determiner = Determiner::new(
            vec![
                Threshold { size: 256 * 1024 * 1024, duration: Duration::from_secs(6 * 60 * 60) },
                Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) }
            ]
        );
        assert!(determiner.is_none());
    }

    #[test]
    fn determine() {
        let determiner = Determiner::new(
            vec![
                Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) },
            ]
        ).unwrap();
        assert_eq!(determiner.determine(100 * 1024), Some(Duration::from_secs(24 * 60 * 60)));
        assert_eq!(determiner.determine(64 * 1024 * 1024), Some(Duration::from_secs(24 * 60 * 60))); // Exactly on the threshold.
        assert_eq!(determiner.determine(100 * 1024 * 1024), None);

        let determiner = Determiner::new(
            vec![
                Threshold { size: 64 * 1024 * 1024, duration: Duration::from_secs(24 * 60 * 60) },
                Threshold { size: 256 * 1024 * 1024, duration: Duration::from_secs(6 * 60 * 60) },
            ]
        ).unwrap();
        assert_eq!(determiner.determine(100 * 1024), Some(Duration::from_secs(24 * 60 * 60)));
        assert_eq!(determiner.determine(64 * 1024 * 1024), Some(Duration::from_secs(24 * 60 * 60))); // Exactly on the threshold.
        assert_eq!(determiner.determine(100 * 1024 * 1024), Some(Duration::from_secs(6 * 60 * 60)));
        assert_eq!(determiner.determine(300 * 1024 * 1024), None);
    }
}