#[macro_use]
extern crate eyre;
#[macro_use]
extern crate tracing;

use color_eyre::Result;

mod api;
mod data;
mod ui;

#[instrument]
fn main() -> Result<()> {
    install_tracing();
    install_eyre()?;

    let data = api::get()?;
    ui::draw(&data)?;
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
