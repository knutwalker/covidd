use clap::{
    AppSettings::{
        ColoredHelp, DeriveDisplayOrder, GlobalVersion, InferSubcommands, VersionlessSubcommands,
    },
    Clap,
};
use humantime::Duration;

impl Command {
    pub fn get() -> Self {
        let args = Args::parse();
        match args.cmd {
            Some(cmd) => cmd,
            None => Command::Run(args.run),
        }
    }

    pub fn verbosity(&self) -> i8 {
        match self {
            Command::Cache(_) => 2,
            Command::Run(r) => (r.verbose as i8) - (r.quiet as i8),
        }
    }
}

/// Download and render latest COVID-19 statistics for Dresden
#[derive(Clap, Debug)]
#[clap(version, about, author = "@knutwalker", global_setting = ColoredHelp, global_setting = DeriveDisplayOrder, global_setting = GlobalVersion, global_setting = InferSubcommands, global_setting = VersionlessSubcommands)]
pub struct Args {
    #[clap(flatten)]
    run: Run,

    #[clap(subcommand)]
    cmd: Option<Command>,
}

/// Default run command
#[derive(Clap, Debug)]
pub struct Run {
    /// Print more logs, can be used multiple times
    #[clap(short, long, parse(from_occurrences), conflicts_with = "quiet")]
    pub verbose: u8,

    /// Print less logs, can be used multiple times
    #[clap(short, long, parse(from_occurrences), conflicts_with = "verbose")]
    pub quiet: u8,

    /// Force download of new data before running
    #[clap(short, long, visible_alias = "download", conflicts_with = "cache")]
    pub force: bool,

    /// Force the use of cached data, never download
    #[clap(short, long, visible_alias = "offline", conflicts_with = "force")]
    pub cache: bool,

    /// Consider cached data stale after this duration
    #[clap(short, long, conflicts_with = "cache", default_value = "1 hour")]
    pub stale_after: Duration,

    /// Timeout for the API call if new data needs to be fetched
    #[clap(short, long, conflicts_with = "cache", default_value = "10 seconds")]
    pub timeout: Duration,

    /// Skip the rendering of the UI
    #[clap(long, hidden = true)]
    pub no_ui: bool,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            verbose: 0,
            quiet: 0,
            force: false,
            cache: false,
            stale_after: Duration::from(std::time::Duration::from_secs(3600)),
            timeout: Duration::from(std::time::Duration::from_secs(10)),
            no_ui: false,
        }
    }
}

#[derive(Clap, Debug)]
pub enum Command {
    Cache(Cache),
    Run(Run),
}

/// Operation on the cache for the data downloads
#[derive(Clap, Debug)]
pub struct Cache {
    #[clap(subcommand)]
    pub cmd: CacheCommand,
}

#[derive(Clap, Debug)]
pub enum CacheCommand {
    /// Lists the file(s) currently in the cache
    List,
    /// Flushes the cache (deletes all cached files)
    Flush,
    /// Refreshes the cache. Download a new file regard less of age.
    Refresh,
}
