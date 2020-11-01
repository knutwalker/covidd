use crate::{
    data::{ApiResponse, Data, DataPoint},
    Result,
};
use std::convert::TryFrom;

pub fn read_from_api() -> Result<Data> {
    static URL: &str = "https://services.arcgis.com/ORpvigFPJUhb8RDF/arcgis/rest/services/corona_DD_7_Sicht/FeatureServer/0/query?f=pjson&where=ObjectId%3E=0&outFields=*";

    debug!("Reading from API");

    let resp = ureq::get(URL).call();
    debug!("Response sc={}", resp.status_line());
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
