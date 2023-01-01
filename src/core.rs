const POSSIBLE_DIRECT_CONFIG_PATHS: [&str] = ["status.toml", ".status.toml"];

struct Config {
    default: String,
}

struct UndirectConfig {
    tool_status: Config,
}

fn find_config() -> Result<Config> {
    for path in POSSIBLE_DIRECT_CONFIG_PATHS.iter() {}
}

#[derive(clap::Parser)]
pub struct StatusCommand {
    path: String,
}

struct FilesFilter {
    gitignore: bool,
    globs: Vec<Glob>,
    regexes: Vec<Regex>,
    globs_ignore: Vec<Glob>,
    regexes_ignore: Vec<Regex>,
}
