#![allow(dead_code)]

use crate::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt::Debug};
use tracing::instrument;

pub type Data = Vec<DataPoint>;

#[derive(Debug, Deserialize)]
pub struct CachedData {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub attributes: Vec<DataPoint>,
}
#[derive(Debug, Serialize)]
pub struct CachingData<'a> {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub attributes: &'a [DataPoint],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataPoint {
    #[serde(rename = "ObjectId")]
    pub object_id: u32,

    #[serde(flatten)]
    pub dates: Dates,

    #[serde(rename = "Anzeige_Indikator", with = "show_indicator_format")]
    pub show: bool,

    #[serde(rename = "Inzidenz")]
    pub incidence: f64,

    #[serde(flatten)]
    pub cases: Cases,

    #[serde(flatten)]
    pub deaths: Deaths,

    #[serde(flatten)]
    pub recoveries: Recoveries,

    #[serde(flatten)]
    pub hospitalisations: Hospitalisations,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dates {
    #[serde(rename = "Datum", with = "date_format")]
    pub date: Date<Utc>,

    #[serde(rename = "Datum_neu", with = "chrono::serde::ts_milliseconds")]
    pub date_ts: DateTime<Utc>,

    #[serde(rename = "Zeitraum")]
    pub date_range: String,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Cases {
    #[serde(rename = "Fallzahl")]
    pub total: u32,

    #[serde(rename = "Zuwachs_Fallzahl")]
    pub increase: u32,

    #[serde(rename = "Fälle_Meldedatum")]
    pub reported: u32,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Deaths {
    #[serde(rename = "Sterbefall")]
    pub total: u32,

    #[serde(rename = "Zuwachs_Sterbefall")]
    pub increase: u32,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Recoveries {
    #[serde(rename = "Genesungsfall")]
    pub total: u32,

    #[serde(rename = "Zuwachs_Genesung")]
    pub increase: u32,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Hospitalisations {
    #[serde(rename = "Hospitalisierung")]
    pub total: u32,

    #[serde(rename = "Zuwachs_Krankenhauseinweisung")]
    pub increase: u32,

    #[serde(rename = "BelegteBetten")]
    pub beds_in_use: u32,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub features: Vec<ApiFeatures>,
}

#[derive(Debug, Deserialize)]
pub struct ApiFeatures {
    pub attributes: ApiAttributes,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiAttributes {
    #[serde(rename = "ObjectId")]
    pub object_id: u32,

    #[serde(rename = "Datum", with = "date_format_or_null")]
    pub date: Option<Date<Utc>>,

    #[serde(rename = "Datum_neu", with = "chrono::serde::ts_milliseconds_option")]
    pub date_ts: Option<DateTime<Utc>>,

    #[serde(rename = "Zeitraum")]
    pub date_range: Option<String>,

    #[serde(rename = "Anzeige_Indikator", with = "show_indicator_format")]
    pub show: bool,

    #[serde(rename = "Inzidenz")]
    pub incidence: Option<f64>,

    #[serde(rename = "Fallzahl")]
    pub cases_total: Option<u32>,

    #[serde(rename = "Zuwachs_Fallzahl")]
    pub cases_increase: Option<u32>,

    #[serde(rename = "Fälle_Meldedatum")]
    pub cases_reported: Option<u32>,

    #[serde(rename = "Sterbefall")]
    pub deaths_total: Option<u32>,

    #[serde(rename = "Zuwachs_Sterbefall")]
    pub deaths_increase: Option<u32>,

    #[serde(rename = "Genesungsfall")]
    pub recoveries_total: Option<u32>,

    #[serde(rename = "Zuwachs_Genesung")]
    pub recoveries_increase: Option<u32>,

    #[serde(rename = "Hospitalisierung")]
    pub hospitalisations_total: Option<u32>,

    #[serde(rename = "Zuwachs_Krankenhauseinweisung")]
    pub hospitalisations_increase: Option<u32>,

    #[serde(rename = "BelegteBetten")]
    pub hospitalisations_beds_in_use: Option<u32>,
}

impl TryFrom<ApiFeatures> for DataPoint {
    type Error = ();

    fn try_from(value: ApiFeatures) -> Result<Self, Self::Error> {
        let ApiAttributes {
            object_id,
            date,
            date_ts,
            date_range,
            show,
            incidence,
            cases_total,
            cases_increase,
            cases_reported,
            deaths_total,
            deaths_increase,
            recoveries_total,
            recoveries_increase,
            hospitalisations_total,
            hospitalisations_increase,
            hospitalisations_beds_in_use,
        } = value.attributes;
        let date = match date {
            Some(date) => date,
            None => return Err(()),
        };
        let data_point = DataPoint {
            object_id,
            dates: Dates {
                date,
                date_ts: date_ts.unwrap_or_else(|| date.and_hms(0, 0, 0)),
                date_range: date_range.unwrap_or_default(),
            },
            show,
            incidence: incidence.unwrap_or_default(),
            cases: Cases {
                total: cases_total.unwrap_or_default(),
                increase: cases_increase.unwrap_or_default(),
                reported: cases_reported.unwrap_or_default(),
            },
            deaths: Deaths {
                total: deaths_total.unwrap_or_default(),
                increase: deaths_increase.unwrap_or_default(),
            },
            recoveries: Recoveries {
                total: recoveries_total.unwrap_or_default(),
                increase: recoveries_increase.unwrap_or_default(),
            },
            hospitalisations: Hospitalisations {
                total: hospitalisations_total.unwrap_or_default(),

                increase: hospitalisations_increase.unwrap_or_default(),

                beds_in_use: hospitalisations_beds_in_use.unwrap_or_default(),
            },
        };
        Ok(data_point)
    }
}

mod date_format {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%d.%m.%Y";

    #[instrument(err, skip(serializer))]
    pub(crate) fn serialize<S>(date: &Date<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format(FORMAT).to_string())
    }

    #[instrument(err, skip(deserializer))]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Date<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }

    #[instrument(err)]
    fn parse(date: &str) -> Result<Date<Utc>, chrono::ParseError> {
        let date = NaiveDate::parse_from_str(&date, FORMAT)?;
        let date = Utc.from_utc_date(&date);
        Ok(date)
    }
}

mod date_format_or_null {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%d.%m.%Y";

    #[instrument(err, skip(serializer))]
    pub(crate) fn serialize<S>(date: &Option<Date<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => serializer.serialize_str(&date.format(FORMAT).to_string()),
            None => serializer.serialize_none(),
        }
    }

    #[instrument(err, skip(deserializer))]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date = Option::<String>::deserialize(deserializer)?;
        parse(date.as_deref()).map_err(serde::de::Error::custom)
    }

    #[instrument(err)]
    fn parse(date: Option<&str>) -> Result<Option<Date<Utc>>, chrono::ParseError> {
        let date = match date {
            None => return Ok(None),
            Some(date) => date,
        };
        let date = NaiveDate::parse_from_str(&date, FORMAT)?;
        let date = Utc.from_utc_date(&date);
        Ok(Some(date))
    }
}

mod show_indicator_format {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    #[instrument(err, skip(serializer))]
    pub(crate) fn serialize<S>(indicator: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if *indicator {
            serializer.serialize_char('x')
        } else {
            serializer.serialize_none()
        }
    }

    #[instrument(err, skip(deserializer))]
    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        let indicator = Option::<String>::deserialize(deserializer)?;
        Ok(indicator.is_some())
    }
}
