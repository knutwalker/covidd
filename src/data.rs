#![allow(dead_code)]

use crate::Result;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::instrument;

pub type Data = Vec<AttributeData>;

#[derive(Debug, Deserialize)]
pub struct ResponseData {
    pub features: Vec<FeatureData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedData {
    pub created_at: u64,
    pub attributes: Vec<AttributeData>,
}

#[derive(Debug, Deserialize)]
pub struct FeatureData {
    pub attributes: AttributeData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dates {
    #[serde(rename = "Datum", with = "date_format")]
    pub date: Option<Date<Utc>>,

    #[serde(rename = "Datum_neu", with = "chrono::serde::ts_milliseconds_option")]
    pub date_ts: Option<DateTime<Utc>>,

    #[serde(rename = "Zeitraum")]
    pub date_range: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Cases {
    #[serde(rename = "Fallzahl")]
    pub total: Option<u32>,

    #[serde(rename = "Zuwachs_Fallzahl")]
    pub increase: Option<u32>,

    #[serde(rename = "Fälle_Meldedatum")]
    pub reported: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Deaths {
    #[serde(rename = "Sterbefall")]
    pub total: Option<u32>,

    #[serde(rename = "Zuwachs_Sterbefall")]
    pub increase: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Recoveries {
    #[serde(rename = "Genesungsfall")]
    pub total: Option<u32>,

    #[serde(rename = "Zuwachs_Genesung")]
    pub increase: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Hospitalisations {
    #[serde(rename = "Hospitalisierung")]
    pub total: Option<u32>,

    #[serde(rename = "Zuwachs_Krankenhauseinweisung")]
    pub increase: Option<u32>,

    #[serde(rename = "BelegteBetten")]
    pub beds_in_use: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttributeData {
    #[serde(rename = "ObjectId")]
    pub object_id: u32,

    #[serde(flatten)]
    pub dates: Dates,

    #[serde(rename = "Anzeige_Indikator", with = "show_indicator_format")]
    pub show: bool,

    #[serde(rename = "Inzidenz")]
    pub indication: Option<f64>,

    #[serde(flatten)]
    pub cases: Cases,

    #[serde(flatten)]
    pub deaths: Deaths,

    #[serde(flatten)]
    pub recoveries: Recoveries,

    #[serde(flatten)]
    pub hospitalisations: Hospitalisations,
}

impl AttributeData {
    pub fn indication(&self) -> f64 {
        self.indication.unwrap_or_default()
    }
}

impl Cases {
    pub fn total(&self) -> u32 {
        self.total.unwrap_or_default()
    }

    pub fn increase(&self) -> u32 {
        self.increase.unwrap_or_default()
    }
}

impl Deaths {
    pub fn total(&self) -> u32 {
        self.total.unwrap_or_default()
    }

    pub fn increase(&self) -> u32 {
        self.increase.unwrap_or_default()
    }
}

impl Recoveries {
    pub fn total(&self) -> u32 {
        self.total.unwrap_or_default()
    }

    pub fn increase(&self) -> u32 {
        self.increase.unwrap_or_default()
    }
}

impl Hospitalisations {
    pub fn total(&self) -> u32 {
        self.total.unwrap_or_default()
    }

    pub fn increase(&self) -> u32 {
        self.increase.unwrap_or_default()
    }
}

mod date_format {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%d.%m.%Y";

    #[instrument(err, skip(serializer))]
    pub(crate) fn serialize<S>(date: &Option<Date<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => {
                let date = format!("{}", date.format(FORMAT));
                serializer.serialize_str(&date)
            }
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
