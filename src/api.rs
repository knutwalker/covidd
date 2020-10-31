use crate::{
    data::{CachedData, Data, ResponseData},
    Result,
};
use std::{fmt::Debug, fs::File, path::Path};
use tracing::instrument;

#[instrument]
pub fn get() -> Result<Data> {
    read_test_file()
}

#[instrument]
fn read_test_file() -> Result<Data> {
    info!("Reading test file");
    static TEST_FILE: &str = "data.json";
    read_from_file(TEST_FILE)
}

#[instrument]
fn read_from_file(file: impl AsRef<Path> + Debug) -> Result<Data> {
    let file = file.as_ref();
    info!("Reading from file: {:?}", file);
    let file = File::open(file)?;
    let data: CachedData = serde_json::from_reader(file)?;
    Ok(data.attributes)
}

#[instrument]
fn read_from_api() -> Result<Data> {
    static URL: &str = "https://services.arcgis.com/ORpvigFPJUhb8RDF/arcgis/rest/services/corona_DD_7_Sicht/FeatureServer/0/query?f=pjson&where=ObjectId%3E=0&outFields=*";

    info!("Reading from API");

    let resp = ureq::get(URL).call();
    debug!("Response sc={}", resp.status_line());
    if resp.error() {
        let err = resp.into_string()?;
        error!("Could not read API: {}", err);
        bail!("Could not read API: {}", err);
    }
    let data = resp.into_json_deserialize::<ResponseData>()?;
    let attributes = data
        .features
        .into_iter()
        .map(|f| f.attributes)
        .collect::<Vec<_>>();

    Ok(attributes)
}
