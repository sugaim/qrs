mod gen_doc_schema;
mod gen_test_data_interp1d;

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
    ShortCut(ShortCutArgs),
    GenTestData(GenTestDataArgs),
    GenDocs(GenDocsArgs),
}

impl Cmd for Commands {
    fn run(&self) -> anyhow::Result<()> {
        match self {
            Commands::ShortCut(args) => args.run(),
            Commands::GenTestData(args) => args.run(),
            Commands::GenDocs(args) => args.run(),
        }
    }
}

// -----------------------------------------------------------------------------
// ShortCutArgs
// ShortCut
// -----------------------------------------------------------------------------
#[derive(Debug, clap::Args)]
pub struct ShortCutArgs {
    #[clap(subcommand)]
    subcmd: ShortCut,
}

#[derive(Debug, clap::Subcommand)]
pub enum ShortCut {
    GenInterp1dTestData,
    GenSchema,
}

impl Cmd for ShortCutArgs {
    fn run(&self) -> anyhow::Result<()> {
        match &self.subcmd {
            ShortCut::GenInterp1dTestData => {
                let args = GenTestDataArgs {
                    subcmd: GenTestData::Interp1d(gen_test_data_interp1d::Args {
                        format: gen_test_data_interp1d::Format::Csv,
                        indir: None,
                        outdir: None,
                        auto_clean: Some(true),
                    }),
                };
                args.run()
            }
            ShortCut::GenSchema => {
                let args = GenDocsArgs {
                    subcmd: GenDocs::Schema(gen_doc_schema::Args {
                        outdir: None,
                        auto_clean: Some(true),
                    }),
                };
                args.run()
            }
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

// -----------------------------------------------------------------------------
// GenDocsArgs
// GenDocs
// -----------------------------------------------------------------------------
#[derive(Debug, clap::Args)]
pub struct GenDocsArgs {
    #[clap(subcommand)]
    subcmd: GenDocs,
}

#[derive(Debug, clap::Subcommand)]
#[clap(rename_all = "snake_case")]
enum GenDocs {
    Schema(gen_doc_schema::Args),
}

impl Cmd for GenDocsArgs {
    fn run(&self) -> anyhow::Result<()> {
        match &self.subcmd {
            GenDocs::Schema(args) => args.run(),
        }
    }
}
