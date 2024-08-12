pub mod gen_test_data_interp1d;

// -----------------------------------------------------------------------------
// Cmd
// -----------------------------------------------------------------------------
pub trait Cmd {
    fn run(&self) -> anyhow::Result<()>;
}

// -----------------------------------------------------------------------------
// Commands
// -----------------------------------------------------------------------------
#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    GenTestData(GenTestDataArgs),
}

impl Cmd for Commands {
    fn run(&self) -> anyhow::Result<()> {
        match self {
            Commands::GenTestData(args) => args.run(),
        }
    }
}

// -----------------------------------------------------------------------------
// GenTestDataArgs
// GenTestData
// -----------------------------------------------------------------------------
#[derive(Debug, clap::Args)]
pub struct GenTestDataArgs {
    #[clap(subcommand)]
    subcmd: GenTestData,
}

#[derive(Debug, clap::Subcommand)]
#[clap(rename_all = "snake_case")]
enum GenTestData {
    Interp1d(gen_test_data_interp1d::Args),
}

impl Cmd for GenTestDataArgs {
    fn run(&self) -> anyhow::Result<()> {
        match &self.subcmd {
            GenTestData::Interp1d(args) => args.run(),
        }
    }
}
