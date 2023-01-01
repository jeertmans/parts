use crate::config::{merge_globs_and_regexes, Config};
use std::io::Write;
use std::path::Path;
use termcolor::BufferWriter;

#[cfg(unix)]
fn write_path<W: Write>(mut wtr: W, path: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    wtr.write_all(path.as_os_str().as_bytes())?;
    wtr.write_all(b"\n")
}

#[cfg(not(unix))]
fn write_path<W: Write>(mut wtr: W, path: &Path) -> std::io::Result<()> {
    wtr.write_all(path.to_string_lossy().as_bytes())?;
    wtr.write_all(b"\n")
}

pub struct Walker {
    directory: String,
    ignore_hidden: bool,
    use_gitignore: bool,
    include: regex::bytes::RegexSet,
    exclude: regex::bytes::RegexSet,
}

impl From<Config> for Walker {
    fn from(config: Config) -> Self {
        let directory = config.directory;
        let ignore_hidden = config.ignore_hidden;
        let use_gitignore = config.use_gitignore;
        let include = merge_globs_and_regexes(config.globs, config.regexes);
        let exclude = merge_globs_and_regexes(config.exclude_globs, config.exclude_regexes);
        Walker {
            directory,
            ignore_hidden,
            use_gitignore,
            include,
            exclude,
        }
    }
}

impl Walker {
    pub fn walk(&self, buffer_writer: &BufferWriter) {
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
            let mut stdout = buffer_writer.buffer();
            move || {
                for path_buf in rx.iter().filter_map(|de| {
                    let path = if de.path().starts_with("./") {
                        de.path().strip_prefix("./").unwrap()
                    } else {
                        de.path()
                    };
                    let strl = path.to_string_lossy();
                    let utf8 = strl.as_bytes();
                    if path.is_file() && include.is_match(utf8) && !exclude.is_match(utf8) {
                        Some(path.to_path_buf())
                    } else {
                        None
                    }
                }) {
                    write_path(&mut stdout, path_buf.as_path()).unwrap();
                }
                stdout
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
        buffer_writer.print(&stdout_thread.join().unwrap()).unwrap();
    }
}
