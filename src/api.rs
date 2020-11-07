use crate::{
    data::{ApiResponse, Data, DataPoint},
    Result,
};
use std::{convert::TryFrom, time::Duration};

static UA: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[instrument(err)]
pub fn call(timeout: Duration) -> Result<Data> {
    static URL: &str = "https://services.arcgis.com/ORpvigFPJUhb8RDF/arcgis/rest/services/corona_DD_7_Sicht/FeatureServer/0/query?f=pjson&where=ObjectId%3E=0&outFields=*";

    debug!("Reading from API");

    let data = minreq::get(URL)
        .with_header("User-Agent", UA)
        .with_timeout(timeout.as_secs())
        .send()?
        .json::<ApiResponse>()?;

    let population = f64::from(populace(timeout, true)?);

    let attributes = data
        .features
        .into_iter()
        .filter_map(|f| DataPoint::try_from(f).ok())
        .scan(Counts::default(), |counts, mut d| {
            let sum_of_increase = counts
                .rolling_increase
                .iter()
                .copied()
                .map(u64::from)
                .sum::<u64>();
            let incidence = sum_of_increase as f64 * 100_000.0 / population;
            d.incidence_calculated = incidence;

            counts.rolling_increase.copy_within(1..7, 0);
            counts.rolling_increase[6] = d.cases.reported;

            macro_rules! inc {
                ($($value:ident),+) => {{
                    $(
                        if d.$value.total > 0 {
                            d.$value.increase = d.$value.total.saturating_sub(counts.$value);
                            counts.$value = d.$value.total;
                        } else {
                            counts.$value += d.$value.increase;
                            d.$value.total = counts.$value;
                        }
                    )+
                }};
            }

            inc!(cases, deaths, recoveries, hospitalisations);

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
    rolling_increase: [u32; 7],
}

#[instrument(err)]
pub fn populace(timeout: Duration, strict: bool) -> Result<u32> {
    static POP_URL: &str = "https://opendata.dresden.de/duva2ckan/files/de-sn-dresden-einwohner___md_34e_2020_-_3006_od_bevoelkerung_ab_stadtteil_hauptwohner_geschlecht_deutsche__auslaender/content";

    debug!("Reading population info from API");

    let data = minreq::get(POP_URL)
        .with_header("User-Agent", UA)
        .with_timeout(10)
        .send()?
        .into_bytes();
    let data = String::from_utf8_lossy(&data);
    if strict {
        strict_pop(data)
    } else {
        Ok(lossy_pop(data))
    }
}

fn strict_pop(data: impl AsRef<str>) -> Result<u32> {
    let total = data
        .as_ref()
        .lines()
        .skip(1)
        .flat_map(|l| l.split(';').last())
        .map(|d| d.parse::<u32>())
        .try_fold(0_u32, |total, parsed| parsed.map(|p| total + p))?;
    Ok(total)
}

fn lossy_pop(data: impl AsRef<str>) -> u32 {
    data.as_ref()
        .lines()
        .skip(1)
        .flat_map(|l| l.split(';').last())
        .filter_map(|d| d.parse::<u32>().ok())
        .sum::<u32>()
}
