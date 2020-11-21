pub struct Messages {
    bundle: Box<dyn Bundle>,
}

impl Messages {
    pub fn user_default() -> Self {
        let bundle: Box<dyn Bundle> = match user_lang() {
            Some(Lang::De) => Box::new(BundleDe),
            _ => Box::new(BundleEn),
        };
        Self { bundle }
    }

    pub fn get(
        &self,
        msg: MsgId,
        count: impl Into<f64>,
        increase: Option<impl Into<f64>>,
    ) -> String {
        self.bundle.get(msg, count.into(), increase.map(Into::into))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MsgId {
    Recovered,
    Hospitalised,
    Deaths,
    Active,
    Cases,
    Incidence,
}

pub trait Bundle {
    fn get(&self, msg: MsgId, count: f64, increase: Option<f64>) -> String;
}

struct BundleDe;

impl Bundle for BundleDe {
    #[rustfmt::skip]
    fn get(&self, msg: MsgId, count: f64, increase: Option<f64>) -> String {
        match increase {
            Some(inc) => match msg {
                MsgId::Recovered    => format!(  "{:>6} ({:>+5}) Genesene",                count, inc),
                MsgId::Hospitalised => format!(  "{:>6} ({:>+5}) Krankenhauseinweisungen", count, inc),
                MsgId::Deaths       => format!(  "{:>6} ({:>+5}) Sterbefälle",             count, inc),
                MsgId::Active       => format!(  "{:>6} ({:>+5}) Active Fälle",            count, inc),
                MsgId::Cases        => format!(  "{:>6} ({:>+5}) Fälle",                   count, inc),
                MsgId::Incidence    => format!("{:>6.1} ({:>+5.1}) Inzidenz",              count, inc),
            }
            None => match msg {
                MsgId::Recovered    => format!(  "{:>6} Genesene"               , count),
                MsgId::Hospitalised => format!(  "{:>6} Krankenhauseinweisungen", count),
                MsgId::Deaths       => format!(  "{:>6} Sterbefälle"            , count),
                MsgId::Active       => format!(  "{:>6} Active Fälle"           , count),
                MsgId::Cases        => format!(  "{:>6} Fälle"                  , count),
                MsgId::Incidence    => format!("{:>6.1} Inzidenz"               , count),
            }
        }
    }
}

struct BundleEn;

impl Bundle for BundleEn {
    #[rustfmt::skip]
    fn get(&self, msg: MsgId, count: f64, increase: Option<f64>) -> String {
        match increase {
            Some(inc) => match msg {
                MsgId::Recovered    => format!(  "{:>6} ({:>+5}) recovered",    count, inc),
                MsgId::Hospitalised => format!(  "{:>6} ({:>+5}) hospitalised", count, inc),
                MsgId::Deaths       => format!(  "{:>6} ({:>+5}) deaths",       count, inc),
                MsgId::Active       => format!(  "{:>6} ({:>+5}) active cases", count, inc),
                MsgId::Cases        => format!(  "{:>6} ({:>+5}) total cases",  count, inc),
                MsgId::Incidence    => format!("{:>6.1} ({:>+5.1}) incidence",  count, inc),
            }
            None => match msg {
                MsgId::Recovered    => format!(  "{:>6} recovered"    , count),
                MsgId::Hospitalised => format!(  "{:>6} hospitalised" , count),
                MsgId::Deaths       => format!(  "{:>6} deaths"       , count),
                MsgId::Active       => format!(  "{:>6} active cases" , count),
                MsgId::Cases        => format!(  "{:>6} total cases"  , count),
                MsgId::Incidence    => format!("{:>6.1} incidence"    , count),
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Lang {
    De,
    En,
}

impl Lang {
    fn of(v: &str) -> Option<Self> {
        match v {
            "de" | "DE" => Some(Lang::De),
            "en" | "EN" => Some(Lang::En),
            _ => None,
        }
    }
}

fn user_lang() -> Option<Lang> {
    locale_config::Locale::current()
        .tags_for("messages")
        .flat_map(|t| t.as_ref().get(..2).and_then(Lang::of).into_iter())
        .next()
}
