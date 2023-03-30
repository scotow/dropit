use std::{str::FromStr, time::Duration};

use byte_unit::Byte;

#[derive(Clone, Debug)]
pub struct Threshold {
    pub size: u64,
    pub default: Duration,
    pub allowed: Option<Duration>,
}

impl FromStr for Threshold {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parts = input.split(':');
        let size = parts
            .next()
            .ok_or("missing size")?
            .parse::<Byte>()
            .map_err(|_| "invalid size")?
            .get_bytes();
        let default = parts
            .next()
            .ok_or("missing default duration")?
            .parse::<humantime::Duration>()
            .map_err(|_| "invalid duration")?;
        let allowed = match parts.next() {
            Some(s) => Some(
                s.parse::<humantime::Duration>()
                    .map_err(|_| "invalid duration")?,
            ),
            None => None,
        };
        if parts.next().is_some() {
            return Err("invalid format (must be SIZE:DURATION[:DURATION]");
        }
        Ok(Threshold {
            size,
            default: default.into(),
            allowed: allowed.map(|d| d.into()),
        })
    }
}

#[derive(Debug)]
pub struct Determiner(Vec<Threshold>);

impl Determiner {
    pub fn new(thresholds: Vec<Threshold>) -> Result<Self, &'static str> {
        if thresholds.is_empty() {
            return Err("empty determiner");
        }
        if thresholds.iter().any(|t| t.allowed.is_none())
            && thresholds.iter().any(|t| t.allowed.is_some())
        {
            return Err("either none or either all thresholds must have an extended duration");
        }
        if thresholds[0].allowed.is_some()
            && thresholds.iter().any(|t| t.allowed.unwrap() < t.default)
        {
            // We verified before that all or none at set.
            return Err("maximum duration cannot be shorter than the default one");
        }
        for couple in thresholds.windows(2) {
            if couple[0].size > couple[1].size
                || couple[0].default < couple[1].default
                || couple[0].allowed < couple[1].allowed
            {
                return Err("invalid thresholds order");
            }
        }

        Ok(Self(thresholds))
    }

    pub fn determine(&self, size: u64) -> Option<(Duration, Option<Duration>)> {
        self.0
            .iter()
            .find(|t| size <= t.size)
            .map(|t| (t.default, t.allowed))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::upload::expiration::{Determiner, Threshold};

    #[test]
    fn determiner() {
        let determiner = Determiner::new(vec![Threshold {
            size: 1,
            default: Duration::from_secs(1),
            allowed: None,
        }]);
        assert!(determiner.is_ok());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 1,
                default: Duration::from_secs(2),
                allowed: None,
            },
            Threshold {
                size: 2,
                default: Duration::from_secs(1),
                allowed: None,
            },
        ]);
        assert!(determiner.is_ok());

        let determiner = Determiner::new(Vec::new());
        assert!(determiner.is_err());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 1,
                default: Duration::from_secs(1),
                allowed: None,
            },
            Threshold {
                size: 2,
                default: Duration::from_secs(2),
                allowed: None,
            },
        ]);
        assert!(determiner.is_err());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 2,
                default: Duration::from_secs(2),
                allowed: None,
            },
            Threshold {
                size: 1,
                default: Duration::from_secs(1),
                allowed: None,
            },
        ]);
        assert!(determiner.is_err());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 1,
                default: Duration::from_secs(2),
                allowed: Some(Duration::from_secs(3)),
            },
            Threshold {
                size: 2,
                default: Duration::from_secs(1),
                allowed: Some(Duration::from_secs(2)),
            },
        ]);
        assert!(determiner.is_ok());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 1,
                default: Duration::from_secs(2),
                allowed: Some(Duration::from_secs(3)),
            },
            Threshold {
                size: 2,
                default: Duration::from_secs(1),
                allowed: None,
            },
        ]);
        assert!(determiner.is_err());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 1,
                default: Duration::from_secs(2),
                allowed: None,
            },
            Threshold {
                size: 2,
                default: Duration::from_secs(1),
                allowed: Some(Duration::from_secs(2)),
            },
        ]);
        assert!(determiner.is_err());

        let determiner = Determiner::new(vec![
            Threshold {
                size: 1,
                default: Duration::from_secs(2),
                allowed: Some(Duration::from_secs(3)),
            },
            Threshold {
                size: 2,
                default: Duration::from_secs(1),
                allowed: Some(Duration::from_secs(4)),
            },
        ]);
        assert!(determiner.is_err());

        let determiner = Determiner::new(vec![Threshold {
            size: 1,
            default: Duration::from_secs(2),
            allowed: Some(Duration::from_secs(1)),
        }]);
        assert!(determiner.is_err());
    }

    #[test]
    fn determine() {
        let determiner = Determiner::new(vec![Threshold {
            size: 2,
            default: Duration::from_secs(1),
            allowed: None,
        }])
        .unwrap();
        assert_eq!(
            determiner.determine(1),
            Some((Duration::from_secs(1), None))
        );
        assert_eq!(
            determiner.determine(2),
            Some((Duration::from_secs(1), None))
        ); // Exactly on the threshold.
        assert_eq!(determiner.determine(3), None);

        let determiner = Determiner::new(vec![
            Threshold {
                size: 2,
                default: Duration::from_secs(2),
                allowed: None,
            },
            Threshold {
                size: 4,
                default: Duration::from_secs(1),
                allowed: None,
            },
        ])
        .unwrap();
        assert_eq!(
            determiner.determine(1),
            Some((Duration::from_secs(2), None))
        );
        assert_eq!(
            determiner.determine(2),
            Some((Duration::from_secs(2), None))
        ); // Exactly on the threshold.
        assert_eq!(
            determiner.determine(3),
            Some((Duration::from_secs(1), None))
        );
        assert_eq!(
            determiner.determine(4),
            Some((Duration::from_secs(1), None))
        ); // Exactly on the threshold.
        assert_eq!(determiner.determine(5), None);
    }
}
