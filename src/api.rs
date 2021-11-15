use crate::{
    data::{ApiAttributes, ApiResponse, Data, DataPoint},
    Result,
};
use chrono::{NaiveDate, TimeZone, Utc};
use std::{convert::TryFrom, io::Cursor, time::Duration};

static UA: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[instrument(err)]
pub fn call(timeout: Duration) -> Result<Data> {
    let population = f64::from(populace(timeout)?);

    let initial_data = get_full_data(timeout, population)?;
    let current_data = get_current_data(timeout, population, initial_data.len())?;
    let data = correct_data(vec![initial_data, current_data], population);
    Ok(data)
}

#[instrument(err)]
pub fn populace(timeout: Duration) -> Result<u32> {
    static POP_URL: &str = "https://opendata.dresden.de/duva2ckan/files/de-sn-dresden-einwohner___md_34e_2020_-_3006_od_bevoelkerung_ab_stadtteil_hauptwohner_geschlecht_deutsche__auslaender/content";

    debug!("Reading population info from API");

    let data = minreq::get(POP_URL)
        .with_header("User-Agent", UA)
        .with_timeout(10)
        .send()?
        .into_bytes();

    let rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(Cursor::new(data));

    let total = rdr
        .into_byte_records()
        .map(|record| -> Result<_> {
            let record = record?;
            let record = record.iter().last().ok_or_else(|| eyre!("empty line"))?;
            let record = std::str::from_utf8(record)?;
            let record = record.parse::<u32>()?;
            Ok(record)
        })
        .try_fold(0_u32, |total, parsed| parsed.map(|p| total + p))?;

    Ok(total)
}

#[instrument(err)]
pub fn get_current_data(timeout: Duration, population: f64, skip: usize) -> Result<Data> {
    let url = format!("https://services.arcgis.com/ORpvigFPJUhb8RDF/arcgis/rest/services/corona_DD_7_Sicht/FeatureServer/0/query?f=pjson&where=ObjectId%3E{}&outFields=*", skip);

    debug!("Reading from API: {}", url);

    let data = minreq::get(url)
        .with_header("User-Agent", UA)
        .with_timeout(timeout.as_secs())
        .send()?
        .json::<ApiResponse>()?;

    let data = data
        .features
        .into_iter()
        .map(|f| DataPoint::try_from(f.attributes))
        .collect::<Result<Vec<_>>>()?;

    Ok(data)
}

#[instrument(err)]
pub fn get_full_data(timeout: Duration, population: f64) -> Result<Vec<DataPoint>> {
    static CSV_URL: &str = "https://opendata.dresden.de/duva2ckan/files/de-sn-dresden-corona_-_covid-19_-_fallzahlen_md1_dresden_2020ff/content";

    debug!("Reading CSV from data portal");

    let data = minreq::get(CSV_URL)
        .with_header("User-Agent", UA)
        .with_timeout(timeout.as_secs())
        .send()?
        .into_bytes();

    const FORMAT: &str = "%Y-%m-%d";

    let rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(Cursor::new(data));

    let attributes = rdr
        .into_records()
        .enumerate()
        .map(|(id, result)| -> Result<ApiAttributes> {
            let record = result?;
            let date = NaiveDate::parse_from_str(&record[0], FORMAT)?;
            let date = Utc.from_utc_date(&date);
            let attributes = ApiAttributes {
                object_id: id as u32 + 1,
                date: Some(date),
                date_ts: None,
                date_range: None,
                show: false,
                incidence: None,
                cases_total: Some(record[5].parse()?),
                cases_increase: None,
                cases_reported: Some(record[4].parse()?),
                deaths_total: Some(record[10].parse()?),
                deaths_increase: Some(record[9].parse()?),
                recoveries_total: Some(record[12].parse()?),
                recoveries_increase: Some(record[11].parse()?),
                hospitalisations_total: Some(record[8].parse()?),
                hospitalisations_increase: Some(record[7].parse()?),
                hospitalisations_beds_in_use: None,
            };
            Ok(attributes)
        })
        .map(|att| -> Result<DataPoint> {
            let att = att?;
            DataPoint::try_from(att)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(attributes)
}

#[instrument]
pub fn correct_data<I, II>(data_points: I, population: f64) -> Vec<DataPoint>
where
    I: IntoIterator<Item = II> + std::fmt::Debug,
    II: IntoIterator<Item = DataPoint>,
{
    fn merge(pop: f64, counts: &mut Counts, mut data_point: DataPoint) -> Option<DataPoint> {
        let sum_of_increase = counts
            .rolling_increase
            .iter()
            .copied()
            .map(u64::from)
            .sum::<u64>();
        let incidence = sum_of_increase as f64 * 100_000.0 / pop;
        data_point.incidence_calculated = incidence;

        counts.rolling_increase.copy_within(1..7, 0);
        counts.rolling_increase[6] = data_point.cases.reported;

        macro_rules! inc {
            ($($value:ident),+) => {{
                $(
                    if data_point.$value.total > 0 {
                        data_point.$value.increase = data_point.$value.total as i32 - counts.$value as i32;
                        counts.$value = data_point.$value.total;
                    } else {
                        counts.$value = (counts.$value as i32 + data_point.$value.increase) as _;
                        data_point.$value.total = counts.$value;
                    }
                )+
            }};
        }

        inc!(cases, deaths, recoveries, hospitalisations);

        Some(data_point)
    }

    data_points
        .into_iter()
        .flatten()
        .scan(Counts::default(), |counts, d| merge(population, counts, d))
        .collect()
}

#[derive(Debug, Default)]
struct Counts {
    cases: u32,
    deaths: u32,
    recoveries: u32,
    hospitalisations: u32,
    rolling_increase: [u32; 7],
}
