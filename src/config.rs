use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

const SUPPORTED_CONFIG_VERSION: u64 = 1;

#[derive(Debug)]
pub struct Config {
    claude_executable: String,
    wikis: BTreeMap<String, WikiConfig>,
}

#[derive(Debug)]
pub struct WikiConfig {
    path: PathBuf,
    entrypoint: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    config_version: u64,
    #[serde(default = "default_claude_executable")]
    claude_executable: String,
    wikis: BTreeMap<String, RawWikiConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWikiConfig {
    path: PathBuf,
    entrypoint: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config '{}': {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid config: config_version must equal 1")]
    UnsupportedVersion,
    #[error("invalid config: at least one wiki must be configured")]
    EmptyWikiRegistry,
    #[error("invalid config: wiki '{wiki}' path must be an absolute existing directory")]
    InvalidWikiPath { wiki: String },
    #[error("invalid config: wiki '{wiki}' entrypoint must be non-empty")]
    EmptyEntrypoint { wiki: String },
    #[error("unknown wiki '{0}'")]
    UnknownWiki(String),
    #[error("could not determine default config path: {0}")]
    DefaultPath(&'static str),
}

impl Config {
    pub fn parse(input: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(input)?;
        if raw.config_version != SUPPORTED_CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion);
        }
        if raw.wikis.is_empty() {
            return Err(ConfigError::EmptyWikiRegistry);
        }

        let mut wikis = BTreeMap::new();
        for (id, wiki) in raw.wikis {
            if !wiki.path.is_absolute() || !wiki.path.is_dir() {
                return Err(ConfigError::InvalidWikiPath { wiki: id });
            }
            if wiki.entrypoint.trim().is_empty() {
                return Err(ConfigError::EmptyEntrypoint { wiki: id });
            }
            wikis.insert(
                id,
                WikiConfig {
                    path: wiki.path,
                    entrypoint: wiki.entrypoint,
                },
            );
        }

        Ok(Self {
            claude_executable: raw.claude_executable,
            wikis,
        })
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let input = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&input)
    }

    pub fn claude_executable(&self) -> &str {
        &self.claude_executable
    }

    pub fn wiki(&self, id: &str) -> Result<&WikiConfig, ConfigError> {
        self.wikis
            .get(id)
            .ok_or_else(|| ConfigError::UnknownWiki(id.to_owned()))
    }
}

impl WikiConfig {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entrypoint(&self) -> &str {
        &self.entrypoint
    }
}

fn default_claude_executable() -> String {
    "claude".to_owned()
}

#[derive(Clone, Copy, Debug)]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }
}

#[derive(Debug, Default)]
pub struct Environment {
    pub appdata: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

impl Environment {
    pub fn current() -> Self {
        Self {
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            home: std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from),
        }
    }
}

pub fn default_config_path(
    platform: Platform,
    environment: &Environment,
) -> Result<PathBuf, ConfigError> {
    match platform {
        Platform::Windows => environment
            .appdata
            .as_ref()
            .map(|path| path.join("llm-wikis").join("config.toml"))
            .ok_or(ConfigError::DefaultPath("APPDATA is not set")),
        Platform::Macos => environment
            .home
            .as_ref()
            .map(|path| {
                path.join("Library")
                    .join("Application Support")
                    .join("llm-wikis")
                    .join("config.toml")
            })
            .ok_or(ConfigError::DefaultPath("HOME is not set")),
        Platform::Linux => environment
            .xdg_config_home
            .as_ref()
            .map(|path| path.join("llm-wikis").join("config.toml"))
            .or_else(|| {
                environment
                    .home
                    .as_ref()
                    .map(|path| path.join(".config").join("llm-wikis").join("config.toml"))
            })
            .ok_or(ConfigError::DefaultPath(
                "XDG_CONFIG_HOME and HOME are not set",
            )),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{Config, Environment, Platform, default_config_path};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "llm-wikis-config-test-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("test directory should be removed");
        }
    }

    fn valid_config(wiki_path: &Path) -> String {
        format!(
            r#"
config_version = 1

[wikis.agents]
path = {}
entrypoint = "/wiki-query"
"#,
            toml::Value::String(wiki_path.display().to_string())
        )
    }

    #[test]
    fn parses_valid_config_and_defaults_claude_executable() {
        let wiki = TestDir::new();

        let config = Config::parse(&valid_config(wiki.path())).expect("config should parse");

        assert_eq!(config.claude_executable(), "claude");
        assert_eq!(
            config.wiki("agents").expect("wiki should exist").path(),
            wiki.path()
        );
        assert_eq!(
            config
                .wiki("agents")
                .expect("wiki should exist")
                .entrypoint(),
            "/wiki-query"
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        let wiki = TestDir::new();
        let input = valid_config(wiki.path()).replacen(
            "config_version = 1",
            "config_version = 1\nextra = true",
            1,
        );

        let error = Config::parse(&input).expect_err("unknown field should fail");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_unknown_wiki_fields() {
        let wiki = TestDir::new();
        let input = format!("{}\nunexpected = true\n", valid_config(wiki.path()));

        let error = Config::parse(&input).expect_err("unknown wiki field should fail");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_empty_wiki_registry() {
        let input = r#"
config_version = 1
wikis = {}
"#;

        let error = Config::parse(input).expect_err("empty wiki registry should fail");

        assert!(error.to_string().contains("at least one wiki"));
    }

    #[test]
    fn rejects_unsupported_config_version() {
        let wiki = TestDir::new();
        let input =
            valid_config(wiki.path()).replacen("config_version = 1", "config_version = 2", 1);

        let error = Config::parse(&input).expect_err("wrong version should fail");

        assert!(error.to_string().contains("config_version must equal 1"));
    }

    #[test]
    fn rejects_relative_wiki_path() {
        let input = r#"
config_version = 1

[wikis.agents]
path = "relative/wiki"
entrypoint = "/wiki-query"
"#;

        let error = Config::parse(input).expect_err("relative path should fail");

        assert!(error.to_string().contains("absolute"));
    }

    #[test]
    fn rejects_missing_wiki_directory() {
        let wiki = TestDir::new();
        let missing = wiki.path().join("missing");

        let error = Config::parse(&valid_config(&missing)).expect_err("missing path should fail");

        assert!(error.to_string().contains("existing directory"));
    }

    #[test]
    fn rejects_file_as_wiki_directory() {
        let temp = TestDir::new();
        let file = temp.path().join("wiki.txt");
        fs::write(&file, "not a directory").expect("fixture should be written");

        let error = Config::parse(&valid_config(&file)).expect_err("file path should fail");

        assert!(error.to_string().contains("existing directory"));
    }

    #[test]
    fn rejects_empty_entrypoint() {
        let wiki = TestDir::new();
        let input = valid_config(wiki.path()).replace("/wiki-query", "   ");

        let error = Config::parse(&input).expect_err("empty entrypoint should fail");

        assert!(error.to_string().contains("entrypoint"));
    }

    #[test]
    fn rejects_duplicate_wiki_ids() {
        let wiki = TestDir::new();
        let section = format!(
            r#"
[wikis.agents]
path = {}
entrypoint = "/other"
"#,
            toml::Value::String(wiki.path().display().to_string())
        );
        let input = format!("{}{}", valid_config(wiki.path()), section);

        let error = Config::parse(&input).expect_err("duplicate wiki ID should fail");

        assert!(error.to_string().to_lowercase().contains("duplicate"));
    }

    #[test]
    fn reports_unknown_wiki() {
        let wiki = TestDir::new();
        let config = Config::parse(&valid_config(wiki.path())).expect("config should parse");

        let error = config
            .wiki("missing")
            .expect_err("unknown wiki should fail");

        assert_eq!(error.to_string(), "unknown wiki 'missing'");
    }

    #[test]
    fn loads_config_from_explicit_path() {
        let temp = TestDir::new();
        let wiki = TestDir::new();
        let config_path = temp.path().join("chosen.toml");
        fs::write(&config_path, valid_config(wiki.path())).expect("fixture should be written");

        let config = Config::load(&config_path).expect("explicit config should load");

        assert_eq!(
            config.wiki("agents").expect("wiki should exist").path(),
            wiki.path()
        );
    }

    #[test]
    fn resolves_windows_default_path_from_appdata() {
        let appdata = PathBuf::from(r"C:\Users\me\AppData\Roaming");
        let environment = Environment {
            appdata: Some(appdata.clone()),
            ..Environment::default()
        };

        let path = default_config_path(Platform::Windows, &environment)
            .expect("APPDATA should resolve config");

        assert_eq!(path, appdata.join("llm-wikis").join("config.toml"));
    }

    #[test]
    fn resolves_macos_default_path_from_home() {
        let environment = Environment {
            home: Some(PathBuf::from("/Users/me")),
            ..Environment::default()
        };

        let path =
            default_config_path(Platform::Macos, &environment).expect("HOME should resolve config");

        assert_eq!(
            path,
            PathBuf::from("/Users/me/Library/Application Support/llm-wikis/config.toml")
        );
    }

    #[test]
    fn resolves_linux_default_path_from_xdg_config_home() {
        let environment = Environment {
            xdg_config_home: Some(PathBuf::from("/custom/config")),
            home: Some(PathBuf::from("/home/me")),
            ..Environment::default()
        };

        let path =
            default_config_path(Platform::Linux, &environment).expect("XDG should resolve config");

        assert_eq!(path, PathBuf::from("/custom/config/llm-wikis/config.toml"));
    }

    #[test]
    fn resolves_linux_default_path_from_home_when_xdg_is_absent() {
        let environment = Environment {
            home: Some(PathBuf::from("/home/me")),
            ..Environment::default()
        };

        let path =
            default_config_path(Platform::Linux, &environment).expect("HOME should resolve config");

        assert_eq!(
            path,
            PathBuf::from("/home/me/.config/llm-wikis/config.toml")
        );
    }

    #[test]
    fn reports_missing_environment_for_default_path() {
        let error = default_config_path(Platform::Linux, &Environment::default())
            .expect_err("missing HOME and XDG should fail");

        assert!(error.to_string().contains("default config path"));
    }
}
