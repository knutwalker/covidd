use crate::{
    data::{ApiResponse, CachedData, CachingData, Data, DataPoint},
    Result,
};
use directories::ProjectDirs;
use std::{
    convert::TryFrom,
    fmt::Debug,
    fs::File,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tracing::instrument;

static APPLICATION: &str = env!("CARGO_PKG_NAME");
static CACHE_FILE: &str = "cached_data.json";

#[instrument(err)]
pub fn get() -> Result<Data> {
    let cached = cached_data()?;
    if let Some(data) = cached {
        // TODO: check for stale data
        info!("Using data from cache from {}", data.created_at);
        return Ok(data.attributes);
    }
    let data = read_from_api()?;
    cache_data(&data)?;
    Ok(data)
}

#[instrument(err)]
pub fn cached_data() -> Result<Option<CachedData>> {
    let cache_file = match cache_file() {
        None => return Ok(None),
        Some(file) => file,
    };

    match read_from_file(&cache_file) {
        Ok(file) => Ok(Some(file)),
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

#[instrument(skip(data), err)]
pub fn cache_data(data: &Data) -> Result<()> {
    let cache_file = match cache_file() {
        None => return Ok(()),
        Some(file) => file,
    };

    match write_to_file(&cache_file, &data) {
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
pub fn cache_file() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("de", "knutwalker", APPLICATION)?;
    let mut file = dirs.cache_dir().to_path_buf();
    file.push(CACHE_FILE);
    Some(file)
}

#[instrument(err)]
fn read_test_file() -> Result<Data> {
    info!("Reading test file");
    static TEST_FILE: &str = "data.json";
    read_from_file(TEST_FILE).map(|x| x.attributes)
}

#[instrument(err)]
fn read_from_file(file: impl AsRef<Path> + Debug) -> Result<CachedData> {
    let file = file.as_ref();
    let file = File::open(file)?;
    let file = try_lock_file_for_reading(file)?;
    read_from_open_file(file)
}

#[instrument(err)]
fn read_from_open_file(file: File) -> Result<CachedData> {
    let data: CachedData = serde_json::from_reader(file)?;
    Ok(data)
}

#[instrument(skip(data), err)]
fn write_to_file(file: impl AsRef<Path> + Debug, data: &Data) -> Result<()> {
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
        attributes: &data[..],
    };
    write_to_open_file(file, data)
}

#[instrument(skip(data), err)]
fn write_to_open_file(file: File, data: CachingData) -> Result<()> {
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}

#[instrument(err)]
fn try_lock_file_for_writing(file: File) -> Result<File> {
    use fs2::FileExt;
    file.try_lock_exclusive()?;
    Ok(file)
}

#[instrument(err)]
fn try_lock_file_for_reading(file: File) -> Result<File> {
    use fs2::FileExt;
    file.try_lock_shared()?;
    Ok(file)
}

#[instrument(err)]
fn read_from_api() -> Result<Data> {
    static URL: &str = "https://services.arcgis.com/ORpvigFPJUhb8RDF/arcgis/rest/services/corona_DD_7_Sicht/FeatureServer/0/query?f=pjson&where=ObjectId%3E=0&outFields=*";

    info!("Reading from API");

    let resp = ureq::get(URL).call();
    info!("Response sc={}", resp.status_line());
    if resp.error() {
        let err = resp.into_string()?;
        error!("Could not read API: {}", err);
        bail!("Could not read API: {}", err);
    }
    let data = resp.into_json_deserialize::<ApiResponse>()?;
    let attributes = data
        .features
        .into_iter()
        .filter_map(|f| DataPoint::try_from(f).ok())
        .collect::<Vec<_>>();

    Ok(attributes)
}
