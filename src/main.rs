#[cfg(feature = "clap_complete")]
use clap_complete::{generate, shells};

use clap::CommandFactory;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use termcolor::{ColorChoice, StandardStream};

mod config;
//mod walk;
use anyhow::Result;

#[derive(Parser)]
#[command(about)]
#[command(author)]
#[command(version)]
struct Cli {
    #[clap(short, long, value_parser = config::validate_config_file_value)]
    /// Config file path, with optional keys. Must be an existing .toml file.
    ///
    /// If the config is included in some parts of a bigger config, indicate
    /// it with keys.
    ///
    /// The expected format is "<path>:(<keys>)+", where keys are separated
    /// with a dot `.` (dot not trailing dot at the end).
    config: Option<String>,
    #[clap(flatten)]
    verbose: Verbosity,
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Parser)]
/// List all parts specified in a given config file.
struct List {}

#[cfg(feature = "clap_complete")]
#[derive(clap::Parser)]
#[command(arg_required_else_help(true))]
#[command(after_help = "Use --help for installation help.")]
#[command(after_long_help = r"DISCUSSION:
    Enable tab completion for Bash, Fish, Zsh, or PowerShell
    Elvish shell completion is currently supported, but not documented below.
    The script is output on `stdout`, allowing one to re-direct the
    output to the file of their choosing. Where you place the file
    will depend on which shell, and which operating system you are
    using. Your particular configuration may also determine where
    these scripts need to be placed.
    Here are some common set ups for the three supported shells under
    Unix and similar operating systems (such as GNU/Linux).
    BASH:
    Completion files are commonly stored in `/etc/bash_completion.d/` for
    system-wide commands, but can be stored in
    `~/.local/share/bash-completion/completions` for user-specific commands.
    Run the command:
        $ mkdir -p ~/.local/share/bash-completion/completions
        $ parts completions bash >> ~/.local/share/bash-completion/completions/parts
    This installs the completion script. You may have to log out and
    log back in to your shell session for the changes to take effect.
    BASH (macOS/Homebrew):
    Homebrew stores bash completion files within the Homebrew directory.
    With the `bash-completion` brew formula installed, run the command:
        $ mkdir -p $(brew --prefix)/etc/bash_completion.d
        $ parts completions bash > $(brew --prefix)/etc/bash_completion.d/parts.bash-completion
    FISH:
    Fish completion files are commonly stored in
    `$HOME/.config/fish/completions`. Run the command:
        $ mkdir -p ~/.config/fish/completions
        $ parts completions fish > ~/.config/fish/completions/parts.fish
    This installs the completion script. You may have to log out and
    log back in to your shell session for the changes to take effect.
    ZSH:
    ZSH completions are commonly stored in any directory listed in
    your `$fpath` variable. To use these completions, you must either
    add the generated script to one of those directories, or add your
    own to this list.
    Adding a custom directory is often the safest bet if you are
    unsure of which directory to use. First create the directory; for
    this example we'll create a hidden directory inside our `$HOME`
    directory:
        $ mkdir ~/.zfunc
    Then add the following lines to your `.zshrc` just before
    `compinit`:
        fpath+=~/.zfunc
    Now you can install the completions script using the following
    command:
        $ parts completions zsh > ~/.zfunc/_parts
    You must then either log out and log back in, or simply run
        $ exec zsh
    for the new completions to take effect.
    CUSTOM LOCATIONS:
    Alternatively, you could save these files to the place of your
    choosing, such as a custom directory inside your $HOME. Doing so
    will require you to add the proper directives, such as `source`ing
    inside your login script. Consult your shells documentation for
    how to add such directives.
    POWERSHELL:
    The powershell completion scripts require PowerShell v5.0+ (which
    comes with Windows 10, but can be downloaded separately for windows 7
    or 8.1).
    First, check if a profile has already been set
        PS C:\> Test-Path $profile
    If the above command returns `False` run the following
        PS C:\> New-Item -path $profile -type file -force
    Now open the file provided by `$profile` (if you used the
    `New-Item` command it will be
    `${env:USERPROFILE}\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`
    Next, we either save the completions file into our profile, or
    into a separate file and source it inside our profile. To save the
    completions into our profile simply use
        PS C:\> parts completions powershell >> ${env:USERPROFILE}\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1
    SOURCE:
        This documentation is directly taken from: https://github.com/rust-lang/rustup/blob/8f6b53628ad996ad86f9c6225fa500cddf860905/src/cli/help.rs#L157")]
/// Generate tab-completion scripts for supported shells.
struct CompleteCommand {
    #[clap(ignore_case = true, value_parser = ["bash", "elvish", "fish", "powershell", "zsh"])]
    shell: String,
}

#[derive(Parser)]
/// List all parts specified in a given config file.
struct ListCommand {}

#[derive(Parser)]
/// Walk through all files in given part, and print them.
///
/// As the traversal is performed in parallel, the output
/// order is not deterministic.
struct WalkCommand {
    /// Part name, as defined in the config file.
    part: String,

    /// If true, will sort files by names.
    ///
    /// This may dramatically decrease the performances.
    #[clap(short = 's', long, default_value = "false")]
    sorted: bool,
}

#[derive(clap::Subcommand)]
enum Action {
    //Show(ShowCommand),
    #[cfg(feature = "clap_complete")]
    Complete(CompleteCommand),
    List(ListCommand),
    Walk(WalkCommand),
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(2);
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();

    pretty_env_logger::formatted_builder()
        .filter_level(cli.verbose.log_level_filter())
        .init();

    let config_file = match &cli.config {
        Some(config_file) => {
            let (path, keys) = config::split_path_and_keys(config_file);
            config::try_parse_config_file(path, keys)?
        }
        None => config::try_find_config_file()?,
    };

    let choice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut stdout = StandardStream::stdout(choice);

    match cli.action {
        Action::List(_) => {
            config_file.write_list(&mut stdout)?;
        }
        Action::Walk(walk) => {
            let config = config_file.get(Some(&walk.part)).unwrap();
            //let walker: walk::Walker = config.clone().try_into()?;
        }
        #[cfg(feature = "clap_complete")]
        Action::Complete(complete) => match complete.shell.as_str() {
            "bash" => generate(
                shells::Bash,
                &mut Cli::command(),
                env!("CARGO_BIN_NAME"),
                &mut stdout,
            ),
            "elvish" => generate(
                shells::Elvish,
                &mut Cli::command(),
                env!("CARGO_BIN_NAME"),
                &mut stdout,
            ),
            "fish" => generate(
                shells::Fish,
                &mut Cli::command(),
                env!("CARGO_BIN_NAME"),
                &mut stdout,
            ),
            "powershell" => generate(
                shells::PowerShell,
                &mut Cli::command(),
                env!("CARGO_BIN_NAME"),
                &mut stdout,
            ),
            "zsh" => generate(
                shells::Zsh,
                &mut Cli::command(),
                env!("CARGO_BIN_NAME"),
                &mut stdout,
            ),
            _ => unreachable!(),
        },
    }

    Ok(())
}
