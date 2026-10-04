use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::Level;
use tracing_subscriber::fmt;

pub struct LogFile {
    path: PathBuf,
    backup: PathBuf,
    file: File,
    written: u64,
}

impl LogFile {
    const DIR: &'static str = "logs";
    const NAME: &'static str = "wire.log";
    const BACKUP: &'static str = "wire.log.1";
    const CAP: u64 = 4 * 1024 * 1024;

    pub fn open(data_dir: &Path) -> Result<Self, io::Error> {
        let dir = data_dir.join(Self::DIR);
        DirBuilder::new().recursive(true).mode(0o700).create(&dir)?;
        let path = dir.join(Self::NAME);
        let file = Self::append(&path)?;
        let written = file.metadata()?.len();
        Ok(Self {
            path,
            backup: dir.join(Self::BACKUP),
            file,
            written,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn append(path: &Path) -> Result<File, io::Error> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)
    }

    fn rotate(&mut self) -> Result<(), io::Error> {
        self.file.flush()?;
        fs::rename(&self.path, &self.backup)?;
        self.file = Self::append(&self.path)?;
        self.written = 0;
        Ok(())
    }
}

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        let len = u64::try_from(buf.len())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        if self.written + len > Self::CAP {
            self.rotate()?;
        }
        self.file.write_all(buf)?;
        self.written += len;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.file.flush()
    }
}

pub fn install(data_dir: &Path) -> Result<PathBuf, io::Error> {
    let log = LogFile::open(data_dir)?;
    let path = log.path().to_path_buf();
    let subscriber = fmt()
        .with_ansi(false)
        .with_max_level(Level::INFO)
        .with_writer(Mutex::new(log))
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error))?;
    Ok(path)
}
