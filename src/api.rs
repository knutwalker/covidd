use crate::{
    data::{ApiResponse, Data, DataPoint},
    Result,
};
use std::{convert::TryFrom, time::Duration};

#[instrument(err)]
pub fn call(timeout: Duration) -> Result<Data> {
    static URL: &str = "https://services.arcgis.com/ORpvigFPJUhb8RDF/arcgis/rest/services/corona_DD_7_Sicht/FeatureServer/0/query?f=pjson&where=ObjectId%3E=0&outFields=*";
    static UA: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    debug!("Reading from API");

    let data = minreq::get(URL)
        .with_header("User-Agent", UA)
        .with_timeout(timeout.as_secs())
        .send()?
        .json::<ApiResponse>()?;

    let attributes = data
        .features
        .into_iter()
        .filter_map(|f| DataPoint::try_from(f).ok())
        .scan(Counts::default(), |counts, mut d| {
            counts.cases += d.cases.increase;
            counts.deaths += d.deaths.increase;
            counts.recoveries += d.recoveries.increase;
            counts.hospitalisations += d.hospitalisations.increase;

            if d.cases.total == 0 {
                d.cases.total = counts.cases;
            }
            if d.deaths.total == 0 {
                d.deaths.total = counts.deaths;
            }
            if d.recoveries.total == 0 {
                d.recoveries.total = counts.recoveries;
            }
            if d.hospitalisations.total == 0 {
                d.hospitalisations.total = counts.hospitalisations;
            }

            Some(d)
        })
        .collect::<Vec<_>>();

    Ok(attributes)
}

#[derive(Debug, Default)]
struct Counts {
    cases: u32,
    deaths: u32,
    recoveries: u32,
    hospitalisations: u32,
}
