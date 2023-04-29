use std::{
    env,
    fmt::{Display, Formatter},
    str::FromStr,
};

use atty::Stream;
use clap::{parser::ValueSource, CommandFactory, Parser};
use config::{builder::DefaultState, ConfigBuilder};
use itertools::chain;
use serde::{
    de::{Error as DeError, Unexpected},
    Deserialize, Deserializer,
};

#[derive(Deserialize, Default, Debug)]
struct Config {
    server: Option<String>,
    username: Option<String>,
    password: Option<String>,
    #[serde(alias = "progress")]
    progress_bar: Option<DetectOption>,
}

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Options {
    #[arg(short = 'c', long, env = "DROPIT_CONFIG")]
    config: Option<String>,
    #[arg(short = 's', long, env = "DROPIT_SERVER")]
    pub server: String,
    #[arg(short = 'u', long, env = "DROPIT_USERNAME", requires = "password")]
    username: Option<String>,
    #[arg(short = 'p', long, env = "DROPIT_PASSWORD", requires = "username")]
    password: Option<String>,
    #[arg(short = 'P', long, env = "DROPIT_PROGRESS", default_value_t)]
    progress_bar: DetectOption,
    pub paths: Vec<String>,
}

impl Options {
    pub fn parse() -> Self {
        let config_option_pos = env::args()
            .skip(1)
            .position(|arg| arg == "-c" || arg == "--config");

        let (config_path, allow_missing) = if let Some(pos) = config_option_pos {
            if let Some(config_path) = env::args().nth(pos + 2) {
                (config_path, false)
            } else {
                panic!("missing config file path");
            }
        } else {
            (
                format!(
                    "{}/.config/dropit.toml",
                    dirs::home_dir().unwrap().to_str().unwrap()
                ),
                true,
            )
        };

        let from_config = ConfigBuilder::<DefaultState>::default()
            .add_source(config::File::with_name(&config_path))
            .build();

        let mut args = Vec::new();

        match from_config {
            Ok(config) => {
                let from_config = config
                    .try_deserialize::<Config>()
                    .expect("invalid configuration");
                let matches_from_cli = Options::command_for_update().get_matches();
                if from_config.server.is_some() && !matches_from_cli.contains_id("server") {
                    args.extend(["--server".to_owned(), from_config.server.unwrap()]);
                }
                if from_config.username.is_some() && !matches_from_cli.contains_id("username") {
                    args.extend(["--username".to_owned(), from_config.username.unwrap()]);
                }
                if from_config.password.is_some() && !matches_from_cli.contains_id("password") {
                    args.extend(["--password".to_owned(), from_config.password.unwrap()]);
                }
                if from_config.progress_bar.is_some()
                    && matches!(
                        matches_from_cli.value_source("progress_bar"),
                        Some(ValueSource::DefaultValue)
                    )
                {
                    args.extend([
                        "--progress-bar".to_owned(),
                        from_config.progress_bar.unwrap().to_string(),
                    ]);
                }
            }
            Err(err) => {
                if !allow_missing {
                    panic!("couldn't load configuration: {}", err);
                }
            }
        }

        Options::parse_from(chain!(env::args().take(1), args, env::args().skip(1)))
    }

    pub fn credentials(&self) -> Option<Credentials> {
        if let (Some(username), Some(password)) = (self.username.as_ref(), self.password.as_ref()) {
            Some(Credentials {
                username: username.clone(),
                password: password.clone(),
            })
        } else {
            None
        }
    }

    pub fn progress_bar(&self) -> bool {
        match self.progress_bar {
            DetectOption::Auto => atty::is(Stream::Stderr),
            DetectOption::On => true,
            DetectOption::Off => false,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Default, Copy, Clone, Debug)]
pub enum DetectOption {
    #[default]
    Auto,
    On,
    Off,
}

impl FromStr for DetectOption {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Self::Auto),
            "false" | "off" | "no" | "n" | "0" => Ok(Self::Off),
            "true" | "on" | "yes" | "y" | "1" => Ok(Self::On),
            _ => Err("Invalid option variant"),
        }
    }
}

impl<'de> Deserialize<'de> for DetectOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wrapper {
            Bool(bool),
            String(String),
        }

        let wrapper = Wrapper::deserialize(deserializer)?;
        match wrapper {
            Wrapper::Bool(true) => Ok(Self::On),
            Wrapper::Bool(false) => Ok(Self::Off),
            Wrapper::String(s) if s == "auto" => Ok(Self::Auto),
            Wrapper::String(s) => Err(D::Error::invalid_value(Unexpected::Str(&s), &"auto")),
        }
    }
}

impl Display for DetectOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectOption::Auto => f.write_str("auto"),
            DetectOption::On => f.write_str("on"),
            DetectOption::Off => f.write_str("off"),
        }
    }
}
