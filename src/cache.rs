use crate::{
    data::{CachedData, CachingData, DataRef},
    Result,
};
use directories::ProjectDirs;
use std::{
    fmt::Debug,
    fs::File,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tracing::{instrument, trace, warn};

static APPLICATION: &str = env!("CARGO_PKG_NAME");
static CACHE_FILE: &str = "cached_data.json";

pub fn get_cached_data() -> Result<Option<CachedData>> {
    get_cached().map(|c| c.map(|(_, d)| d))
}

#[instrument]
pub fn get_cached() -> Result<Option<(PathBuf, CachedData)>> {
    let cache_file = match cache_file() {
        None => return Ok(None),
        Some(file) => file,
    };
    trace!("cache file {}", cache_file.display());

    match read_from_file(&cache_file) {
        Ok(data) => Ok(Some((cache_file, data))),
        Err(e) => {
            if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
                match ioe.kind() {
                    ErrorKind::NotFound => {
                        // cache doesn't exist, we're fine
                        Ok(None)
                    }
                    ErrorKind::PermissionDenied => {
                        warn!("Could not get permission to read the cached data at [{0}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. It is most likely a file permission issue, please make sure that your current user can read [{0}].", cache_file.display());
                        Ok(None)
                    }
                    ErrorKind::Other => {
                        warn!("Could not read the cached data at [{}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. Error: [{}].", cache_file.display(), ioe);
                        Ok(None)
                    }
                    ErrorKind::WouldBlock => {
                        warn!("Did not read the cached data at [{}] as another processed was currently accessing that file.", cache_file.display());
                        Ok(None)
                    }
                    _ => Err(e),
                }
            } else if let Some(sje) = e.downcast_ref::<serde_json::error::Error>() {
                warn!("Could not parse the cached data at [{}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. Error: [{}].", cache_file.display(), sje);
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

#[instrument(skip(data))]
pub fn store_data(data: DataRef<'_>) -> Result<()> {
    let cache_file = match cache_file() {
        None => return Ok(()),
        Some(file) => file,
    };
    trace!("cache file {}", cache_file.display());

    match write_to_file(&cache_file, data) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
                match ioe.kind() {
                    ErrorKind::PermissionDenied => {
                        warn!("Could not get permission to write the cached data to [{0}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. It is most likely a file permission issue, please make sure that your current user can write [{0}].", cache_file.display());
                        Ok(())
                    }
                    ErrorKind::Other => {
                        warn!("Could not write the cached data to [{}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. Error: [{}].", cache_file.display(), ioe);
                        Ok(())
                    }
                    ErrorKind::WouldBlock => {
                        warn!("Did not write the cached data to [{}] as another processed was currently accessing that file.", cache_file.display());
                        Ok(())
                    }
                    _ => Err(e),
                }
            } else if let Some(sje) = e.downcast_ref::<serde_json::error::Error>() {
                warn!("Could not write the cached data to [{}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. Error: [{}].", cache_file.display(), sje);
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

#[instrument]
pub fn remove_cache() -> Result<()> {
    let cache_file = match cache_file() {
        None => return Ok(()),
        Some(file) => file,
    };
    trace!("cache file {}", cache_file.display());

    match remove_file(&cache_file) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
                match ioe.kind() {
                    ErrorKind::NotFound => {
                        // already gone or never existed
                        Ok(())
                    }
                    ErrorKind::PermissionDenied => {
                        warn!("Could not get permission to remove the cached data to [{0}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. It is most likely a file permission issue, please make sure that your current user can write [{0}].", cache_file.display());
                        Ok(())
                    }
                    ErrorKind::Other => {
                        warn!("Could not delete the cached data to [{}]. While this is not an error, it is recommended to investigate the reason, as the cache could otherwise not be used. Error: [{}].", cache_file.display(), ioe);
                        Ok(())
                    }
                    ErrorKind::WouldBlock => {
                        warn!("Did not delete the cached data to [{}] as another processed was currently accessing that file.", cache_file.display());
                        Ok(())
                    }
                    _ => Err(e),
                }
            } else {
                Err(e)
            }
        }
    }
}

fn cache_file() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("de", "knutwalker", APPLICATION)?;
    let mut file = dirs.cache_dir().to_path_buf();
    file.push(CACHE_FILE);
    Some(file)
}

#[instrument(err)]
fn read_from_file(file: impl AsRef<Path> + Debug) -> Result<CachedData> {
    let file = file.as_ref();
    let file = File::open(file)?;
    let file = try_lock_file_for_reading(file)?;
    read_from_open_file(file)
}

fn read_from_open_file(file: File) -> Result<CachedData> {
    let data: CachedData = serde_json::from_reader(file)?;
    Ok(data)
}

#[instrument(skip(data), err)]
fn write_to_file(file: impl AsRef<Path> + Debug, data: DataRef<'_>) -> Result<()> {
    let file = file.as_ref();
    if let Some(parent) = file.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let file = File::create(file)?;
    let file = try_lock_file_for_writing(file)?;
    let data = CachingData {
        created_at: chrono::Utc::now(),
        attributes: data,
    };
    write_to_open_file(file, data)
}

#[instrument(err)]
fn remove_file(file: impl AsRef<Path> + Debug) -> Result<()> {
    Ok(std::fs::remove_file(file)?)
}

fn write_to_open_file(file: File, data: CachingData<'_>) -> Result<()> {
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}

fn try_lock_file_for_writing(file: File) -> Result<File> {
    use fs2::FileExt;
    file.try_lock_exclusive()?;
    Ok(file)
}

fn try_lock_file_for_reading(file: File) -> Result<File> {
    use fs2::FileExt;
    file.try_lock_shared()?;
    Ok(file)
}
