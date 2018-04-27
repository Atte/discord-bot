use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use toml;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        TomlRead(::toml::de::Error);
        TomlWrite(::toml::ser::Error);
    }

    errors {
        LockPoison {
            description("cache lock has been poisoned")
        }
    }
}

pub struct Cache {
    path: PathBuf,
    content: Mutex<CacheContent>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheContent {
    pub seen: HashSet<String>,
}

impl Cache {
    fn new(path: PathBuf, content: CacheContent) -> Self {
        Self {
            path,
            content: Mutex::new(content),
        }
    }

    pub fn from_file<P>(path: P) -> Result<Self>
    where
        P: Into<PathBuf>,
    {
        let path: PathBuf = path.into();

        let mut source: Vec<u8> = Vec::new();
        match File::open(&path) {
            Ok(mut fh) => {
                fh.read_to_end(&mut source)?;
            }
            Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::new(path, CacheContent::default()));
            }
            Err(err) => {
                return Err(err.into());
            }
        }
        Ok(Self::new(path, toml::from_slice(&source)?))
    }

    pub fn with<F, R>(&self, fun: F) -> Result<R>
    where
        F: FnOnce(&mut CacheContent) -> R,
    {
        if let Ok(mut content) = self.content.lock() {
            let result = fun(&mut content);
            let data = toml::to_string_pretty(&*content)?;
            // TODO: keep file open all the time
            let mut fh = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.path)?;
            fh.write_all(data.as_bytes())?;
            Ok(result)
        } else {
            Err(ErrorKind::LockPoison.into())
        }
    }
}
