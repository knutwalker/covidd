use crate::Result;
use color_eyre::{Report, Section};
use fluent::{FluentArgs, FluentBundle, FluentError, FluentResource, FluentValue};
use std::fmt::{Debug, Display};
use unic_langid::{langid, LanguageIdentifier};

pub struct Messages {
    bundle: FluentBundle<FluentResource>,
}

impl Messages {
    pub fn user_default() -> Result<Self> {
        let bundle = match user_lang() {
            Some(lang) if lang.matches(&langid!("de"), false, true) => setup_de(),
            _ => setup_en(),
        };
        let bundle = bundle?;
        Ok(Self { bundle })
    }

    pub fn get(&self, msg: MsgId, count: impl Into<f64>) -> Result<String> {
        let msg = self
            .bundle
            .get_message(msg.ident())
            .ok_or_else(|| eyre!("The message {:?} is not part of the message bundle", msg))?;
        let pat = msg
            .value
            .ok_or_else(|| eyre!("The message {:?} does not have a pattern", msg))?;

        let count = count.into();
        let count = FluentValue::from(count);
        let mut args = FluentArgs::with_capacity(1);
        args.add("count", count);

        let mut errors = Vec::new();
        let value = self.bundle.format_pattern(pat, Some(&args), &mut errors);
        if !errors.is_empty() {
            return chain(
                eyre!("The message {:?} could not be formatted", msg),
                errors.into_iter().map(FluentErrorAsStdErr),
            );
        }

        Ok(value.into_owned())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MsgId {
    Recovered,
    Hospitalised,
    Deaths,
    Cases,
    Incidence,
}

impl MsgId {
    pub fn ident(&self) -> &str {
        match self {
            MsgId::Recovered => "recovered",
            MsgId::Hospitalised => "hospitalised",
            MsgId::Deaths => "deaths",
            MsgId::Cases => "cases",
            MsgId::Incidence => "incidence",
        }
    }
}

fn setup_de() -> Result<FluentBundle<FluentResource>> {
    setup(
        langid!("de"),
        "
recovered = { $count } Genesungsfälle
hospitalised = { $count } Krankenhausfälle
deaths = { $count } Todesfälle
cases = { $count } Infektionsfälle
incidence = { $count } Inzidenz
",
    )
}

fn setup_en() -> Result<FluentBundle<FluentResource>> {
    setup(
        langid!("en"),
        "
recovered = { $count } recovered
hospitalised = { $count } hospitalised
deaths = { $count } deaths
cases = { $count } cases
incidence = { $count } incidence
",
    )
}

fn setup(id: LanguageIdentifier, messages: &str) -> Result<FluentBundle<FluentResource>> {
    let ftl_string = String::from(messages);

    let res = match FluentResource::try_new(ftl_string) {
        Ok(res) => res,
        Err((_, errors)) => {
            return chain(eyre!("Could not parse an FTL string"), errors);
        }
    };

    let mut bundle = FluentBundle::new(&[id]);
    if let Err(errors) = bundle.add_resource(res) {
        return chain(
            eyre!("Failed to add FTL resources to the bundle"),
            errors.into_iter().map(FluentErrorAsStdErr),
        );
    }

    Ok(bundle)
}

fn chain<T, I, E>(main: Report, errors: I) -> Result<T>
where
    I: IntoIterator<Item = E>,
    E: std::error::Error + Send + Sync + 'static,
{
    let err = Err(main);
    let err = errors.into_iter().fold(err, |report, e| report.error(e));
    err
}

struct FluentErrorAsStdErr(FluentError);

impl std::error::Error for FluentErrorAsStdErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            FluentError::ParserError(e) => e.source(),
            _ => None,
        }
    }
}

impl Display for FluentErrorAsStdErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            FluentError::ParserError(p) => Display::fmt(&p, f),
            FluentError::Overriding { kind, id } => {
                let s = format!("Overriding {} with id {}", kind, id);
                f.pad(&s)
            }
            FluentError::ResolverError(r) => {
                let s = match r {
                    fluent::resolver::ResolverError::Reference(s) => {
                        format!("Could not resolve refernce {}", s)
                    }
                    fluent::resolver::ResolverError::MissingDefault => format!("Missing default"),
                    fluent::resolver::ResolverError::Cyclic => format!("Cyclic"),
                    fluent::resolver::ResolverError::TooManyPlaceables => {
                        format!("Too many placebales")
                    }
                };
                f.pad(&s)
            }
        }
    }
}

impl Debug for FluentErrorAsStdErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

fn user_lang() -> Option<LanguageIdentifier> {
    locale_config::Locale::current()
        .tags_for("messages")
        .flat_map(|t| t.as_ref().parse().ok().into_iter())
        .next()
}
