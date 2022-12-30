use crate::config::{try_parse_globs_and_regexes, Config};
use anyhow::{Error, Result};
use std::io::Write;
use std::path::Path;

#[macro_export]
macro_rules! path_as_bytes {
    ($path: ident) => {
        $path.to_string_lossy().as_bytes()
    };
}

#[cfg(unix)]
fn write_path<W: Write>(mut wtr: W, path: &Path) {
    use std::os::unix::ffi::OsStrExt;
    wtr.write_all(path.as_os_str().as_bytes()).unwrap();
    wtr.write_all(b"\n").unwrap();
}

#[cfg(not(unix))]
fn write_path<W: Write>(mut wtr: W, path: &Path) {
    wtr.write_all(path.to_string_lossy().as_bytes()).unwrap();
    wtr.write_all(b"\n").unwrap();
}

pub struct Walker {
    directory: String,
    ignore_hidden: bool,
    use_gitignore: bool,
    include: regex::bytes::RegexSet,
    exclude: regex::bytes::RegexSet,
}

impl TryFrom<Config> for Walker {
    type Error = Error;
    fn try_from(config: Config) -> Result<Self> {
        let directory = config.directory;
        let ignore_hidden = config.ignore_hidden;
        let use_gitignore = config.use_gitignore;
        let include = try_parse_globs_and_regexes(config.globs.iter(), config.regexes.iter())?;
        let exclude = try_parse_globs_and_regexes(
            config.exclude_globs.iter(),
            config.exclude_regexes.iter(),
        )?;
        Ok(Walker {
            directory,
            ignore_hidden,
            use_gitignore,
            include,
            exclude,
        })
    }
}

impl Walker {
    fn walk<T: Write + Send + Sync + Clone + 'static>(&self, stdout: &mut T) {
        let directory = self.directory.clone();
        let ignore_hidden = self.ignore_hidden;
        let use_gitignore = self.use_gitignore;
        let include = self.include.clone();
        let exclude = self.exclude.clone();
        let (tx, rx) = crossbeam_channel::unbounded::<ignore::DirEntry>();

        let walker = ignore::WalkBuilder::new(directory)
            .hidden(ignore_hidden)
            .git_ignore(use_gitignore)
            .build_parallel();

        let stdout_thread = std::thread::spawn({
            let mut stdout = stdout.clone();
            move || {
                for de in rx.iter().filter(|de| {
                    let path = de.path();
                    let strl = path.to_string_lossy();
                    let utf8 = strl.as_bytes();
                    path.is_file() && include.is_match(utf8) && !exclude.is_match(utf8)
                }) {
                    write_path(&mut stdout, de.path());
                }
            }
        });

        walker.run(|| {
            let tx = tx.clone();
            Box::new(move |result| {
                tx.send(result.unwrap()).unwrap();
                ignore::WalkState::Continue
            })
        });

        drop(tx);
        stdout_thread.join().unwrap();
    }
}
