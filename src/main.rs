/*!

Render COVID-19 case data for Dresden in the terminal

# Installation

## Prerequisites

This tool is build with Rust so you need to have a rust toolchain and cargo installed.
If you don't, please visit [https://rustup.rs/](https://rustup.rs/) and follow their instructions.

## Building

The preferred way is to run:

```
make install
```

If you do not have a fairly recent `make` (on macOS, homebrew can install a newer version),
or don't want to use make, you can also run `cargo install --path .`.

## Already built binaries

If you don't want to compile on your own, you can find binaries at [the Github release page](https://github.com/knutwalker/covidd/releases).

# Usage

Run `covidd`.

- Press Up/Right to zoom in
- Press Down/Left to zoom out
- Press Home/End to fully zoom in/out
- Press 1 through 9 to zoom to the latest <n> weeks
- Press q to quit

Run `covidd --help` for an overview of more available options.

### Screenshot

![have a look at doc/screenshot.png](https://knutwalker.s3.eu-central-1.amazonaws.com/covidd/doc/screenshot.png)

*/

#[macro_use]
extern crate eyre;
#[macro_use]
extern crate tracing;

use args::{CacheCommand, Command, Run};
use chrono::{DateTime, Duration, Utc};
use color_eyre::{Help, Result};
use data::{CachedData, Data};

mod api;
mod args;
mod cache;
mod data;
mod messages;
mod ui;

#[instrument]
fn main() -> Result<()> {
    let cmd = Command::get();

    install_tracing(cmd.verbosity());
    install_eyre()?;

    let data_for_ui = match cmd {
        Command::Cache(c) => cache_command(c.cmd)?,
        Command::Run(r) => run_command(r)?,
    };

    if let Some(data) = data_for_ui {
        let msg = messages::Messages::user_default();
        ui::draw(&data, msg)?;
    }

    Ok(())
}

#[instrument(err)]
fn run_command(r: Run) -> Result<Option<Data>> {
    let show_ui = !r.no_ui;
    let data = current_data_with_updated_cache(r)?;
    if show_ui {
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

#[instrument(err)]
fn cache_command(c: CacheCommand) -> Result<Option<Data>> {
    match c {
        CacheCommand::List => {
            if let Some((file, data)) = cache::get_cached()? {
                println!("{}\t{}", file.display(), data.created_at);
            }
        }
        CacheCommand::Flush => cache::remove_cache()?,
        CacheCommand::Refresh => {
            let _ = current_data_with_updated_cache(Run {
                force: true,
                ..Run::default()
            })?;
        }
    };
    Ok(None)
}

fn current_data_with_updated_cache(r: Run) -> Result<Data> {
    let cached_data = cached_data_if_current(r.force, r.cache, r.stale_after)?;

    let data = if let Some(data) = cached_data {
        debug!("Using data from cache from {}", data.created_at);
        data.attributes
    } else {
        debug!("Calling API for new data");
        let data = api::call(r.timeout.into())?;
        cache::store_data(&data)?;
        data
    };

    Ok(data)
}

fn cached_data_if_current(
    ignore_cache: bool,
    force_cache: bool,
    stale_after: humantime::Duration,
) -> Result<Option<CachedData>> {
    let data = if ignore_cache {
        debug!("Ignoring cache since --force was given");
        None
    } else {
        let data = data_from_cache(force_cache)?;
        trace!("Found some data in cache: {}", data.is_some());
        match data {
            Some(data) => {
                if cache_is_stale(data.created_at, stale_after)? {
                    None
                } else {
                    Some(data)
                }
            }
            _ => None,
        }
    };
    trace!("Found current data in cache: {}", data.is_some());
    Ok(data)
}

fn data_from_cache(force: bool) -> Result<Option<CachedData>> {
    let cached = cache::get_cached_data()?;
    if force && cached.is_none() {
        Err(eyre!("--cache is defined, but there is not cached data available").suggestion(
            "Run the `cache refresh` subcommand to set a new cache. Treat any warnings as errors.",
        ))?;
    }
    Ok(cached)
}

fn cache_is_stale(created: DateTime<Utc>, stale_after: humantime::Duration) -> Result<bool> {
    let stale_after = Duration::from_std(stale_after.into())?;
    let now = Utc::now();
    let age = now - created;
    let is_current = age < stale_after;
    trace!(
        "Cached data: created={}, age={}, current={}",
        created,
        humantime::format_duration(age.to_std()?),
        is_current
    );
    Ok(!is_current)
}

fn install_tracing(verbosity: i8) {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(true);
    let filter_layer = EnvFilter::try_from_env("COVIDD_LOG")
        .or_else(|_| EnvFilter::try_new(verbosity_to_level(verbosity)))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

fn verbosity_to_level(verbosity: i8) -> &'static str {
    match verbosity {
        i8::MIN..=-2 => "off",
        -1 => "error",
        0 => "warn",
        1 => concat!(env!("CARGO_PKG_NAME"), "=info"),
        2 => concat!(env!("CARGO_PKG_NAME"), "=debug"),
        3 => concat!(env!("CARGO_PKG_NAME"), "=trace"),
        4 => concat!(env!("CARGO_PKG_NAME"), "=trace,info"),
        5 => concat!(env!("CARGO_PKG_NAME"), "=trace,debug"),
        6..=i8::MAX => "trace",
    }
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
