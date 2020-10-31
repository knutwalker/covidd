#[macro_use]
extern crate eyre;
#[macro_use]
extern crate tracing;

use color_eyre::Result;
use data::AttributeData;
use tracing::{info, instrument};

mod api;
mod data;
mod ui;

#[instrument]
fn main() -> Result<()> {
    install_tracing();
    install_eyre()?;

    let data = api::get()?;

    for data in data.iter().filter(|x| x.show) {
        info!("Got data: {:#?}", data);
    }

    let mut dates = Vec::with_capacity(data.len());
    let mut cases = Vec::with_capacity(data.len());
    let mut deaths = Vec::with_capacity(data.len());
    let mut recoveries = Vec::with_capacity(data.len());
    let mut hospitalisations = Vec::with_capacity(data.len());

    let items = data.into_iter().filter_map(
        |AttributeData {
             dates,
             cases,
             deaths,
             recoveries,
             hospitalisations,
             ..
         }| {
            let date = dates.date?;
            let cases = cases.total();
            let deaths = deaths.total();
            let recoveries = recoveries.total();
            let hospitalisations = hospitalisations.total();

            Some((date, cases, deaths, recoveries, hospitalisations))
        },
    );

    for (date, infected, died, recovered, hospitalised) in items {
        dates.push(date);
        cases.push(infected);
        deaths.push(died);
        recoveries.push(recovered);
        hospitalisations.push(hospitalised);
    }

    ui::draw(&dates, &cases, &recoveries, &deaths, &hospitalisations)?;
    Ok(())
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

fn install_eyre() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .issue_filter(|kind| match kind {
            color_eyre::ErrorKind::NonRecoverable(_) => false,
            color_eyre::ErrorKind::Recoverable(_) => true,
        })
        .install()
}
