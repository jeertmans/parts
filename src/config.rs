use crate::error::{Error, Result};
use globset::Glob;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::warn;
use regex::bytes::{RegexSet, RegexSetBuilder};
use serde::{Deserialize, Deserializer};
use termcolor::{ColorSpec, WriteColor};

lazy_static! {
    static ref POSSIBLE_CONFIG_PATHS: Vec<&'static str> = vec![
        "parts.toml",
        ".parts.toml",
        "Cargo.toml:metadata.parts",
        "pyproject.toml:tool.parts",
    ];
}

const SPLIT_PATH: char = ':';
const SPLIT_KEYS: char = '.';

/// Split a string into a path and a list of keys.
///
/// Path and keys must be separated with a colon `':'`.
/// Keys must be separated with a dot `'.'`.
///
/// # Examples
///
/// ```
/// # use crate::config::split_path_and_keys;
/// let (path, keys) = split_path_and_keys(".parts.toml");
/// assert_eq!(path, ".parts.toml");
/// assert_eq!(keys, vec![]);
///
/// let (path, keys) = split_path_and_keys("Cargo.toml:metadata.parts");
/// assert_eq!(path, "Cargo.toml");
/// assert_eq!(keys, vec!["metadata", "parts"]);
/// ```
pub fn split_path_and_keys(s: &str) -> (&str, Vec<&str>) {
    match s.split_once(SPLIT_PATH) {
        Some((path, keys)) => (path, keys.split(SPLIT_KEYS).collect()),
        None => (s, vec![]),
    }
}

pub fn validate_config_file_value(value: &str) -> Result<String> {
    let (path, _) = split_path_and_keys(value);

    if std::path::Path::new(path).exists() {
        Ok(value.to_string())
    } else {
        Err(Error::ConfigFileDoesNotExist {
            value: value.to_string(),
        })
    }
}


fn deserialize_globs<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<Glob>, D::Error> {
    use serde::de::Error;
    use std::borrow::Cow;
    let globs = <Vec<Cow<str>> as Deserialize>::deserialize(deserializer)?;
    
    globs.iter().map(|glob| Glob::new(glob).map_err(D::Error::custom)).collect()
}

/// Try to parse a config file into a [`ConfigFile`] struct.
///
/// If `keys` is not empty, it will first index the file as if it was
/// a plain TOML document, then parse the appropriate nested table
/// into a [`ConfigFile`].
pub fn try_parse_config_file(path: &str, keys: Vec<&str>) -> Result<ConfigFile> {
    let content = std::fs::read_to_string(&path)?;

    if keys.is_empty() {
        let config_file = toml::from_str(&content)?;
        return Ok(config_file);
    }
    // If keys is not empty, we traverse the inner tables
    let mut toml_document = content.parse::<toml::Value>()?;

    for key in keys.into_iter() {
        match toml_document {
            toml::Value::Table(mut table) => {
                toml_document = table.remove(key).ok_or(Error::KeysNotFound {
                    keys: key.to_string(),
                    path: path.to_string(),
                })?;
            }
            _ => {
                return Err(Error::ValueIsNotTable {
                    path: path.to_string(),
                })
            }
        }
    }

    println!("{}", toml_document);
    let toml = toml_document.try_into()?;
    Ok(toml)
}

pub fn try_find_config_file() -> Result<ConfigFile> {
    for s in POSSIBLE_CONFIG_PATHS.iter() {
        let (path, keys) = split_path_and_keys(s);

        match try_parse_config_file(path, keys) {
            Ok(config_file) => {
                return Ok(ConfigFile {
                    config_file: s.to_string(),
                    ..config_file
                })
            }
            Err(e) => match e {
                Error::TomlDecode(_) => return Err(e),
                _ => warn!("some error occured with parsing {:?}: {}\n", path, e),
            },
        }
    }
    return Err(Error::NoConfigFileFound);
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(skip)]
    pub config_file: String,
    pub default: Option<String>,
    #[serde(flatten)]
    pub configs: std::collections::HashMap<String, Config>,
}

impl ConfigFile {
    pub fn get(&self, key: Option<&str>) -> Option<&Config> {
        if let Some(key) = key {
            self.configs.get(key)
        } else {
            self.get_default_config()
        }
    }

    pub fn get_default_config(&self) -> Option<&Config> {
        match self.default {
            Some(ref name) => self.configs.get(name),
            None => None,
        }
    }

    pub fn matches_default(&self, key: &str) -> bool {
        if let Some(default) = &self.default {
            default.eq(key)
        } else {
            false
        }
    }
    
    /*
    pub fn get_closest_match(&self, key: &str) -> Option<String> {
        ngrammatic::CorpusBuilder::new().fill(self.configs.keys()).finish().search(key, 0.5).first()
    }*/

    pub fn write_list<T: WriteColor>(&self, stdout: &mut T) -> Result<()> {
        let mut filename_color = ColorSpec::new();
        filename_color.set_underline(true);
        let mut key_color = ColorSpec::new();
        key_color.set_bold(true);
        let (path, keys) = split_path_and_keys(&self.config_file);

        match self.configs.len() {
            0 => stdout.write_all(b"Found not part in file: ")?,
            1 => stdout.write_all(b"Found 1 part in file: ")?,
            n => stdout.write_all(format!("Found {n} part in file: ").as_bytes())?,
        };

        stdout.set_color(&filename_color)?;
        stdout.write_all(path.as_bytes())?;
        stdout.reset()?;

        for key in keys.iter() {
            stdout.write_all(b" -> ")?;
            stdout.set_color(&key_color)?;
            stdout.write_all(key.as_bytes())?;
            stdout.reset()?;
        }

        stdout.write_all(b"\n")?;

        if self.configs.is_empty() {
            return Ok(());
        }

        stdout.write_all(b"\n")?;

        for config_name in self.configs.keys().sorted() {
            stdout.write_all(b"- ")?;
            if self.matches_default(&config_name) {
                stdout.set_color(&key_color)?;
                stdout.write_all(format!("{config_name} (default)\n").as_bytes())?;
                stdout.reset()?;
            } else {
                stdout.write_all(config_name.as_bytes())?;
                stdout.write_all(b"\n")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_directory")]
    pub directory: String,
    #[serde(default = "default_true")]
    pub ignore_hidden: bool,
    #[serde(default = "default_true")]
    pub use_gitignore: bool,
    #[serde(default = "default_regexset")]
    #[serde(with = "serde_regex")]
    pub regexes: RegexSet,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_globs")]
    pub globs: Vec<Glob>,
    #[serde(default = "default_regexset")]
    #[serde(with = "serde_regex")]
    pub exclude_regexes: RegexSet,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_globs")]
    pub exclude_globs: Vec<Glob>,
}

fn default_directory() -> String {
    "./".to_string()
}

const fn default_true() -> bool {
    true
}


fn default_regexset() -> RegexSet {
    RegexSet::empty()
}

pub fn merge_globs_and_regexes(globs: Vec<Glob>, regexes: RegexSet) -> RegexSet {
    RegexSetBuilder::new(
        regexes
            .patterns()
            .iter()
            .map(|pattern| pattern.as_str())
            .chain(globs.iter().map(|glob| glob.regex())),
    )
    .build()
    .expect("This cannot fail")
}
