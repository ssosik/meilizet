use color_eyre::Report;
use glob::{glob, Paths};
use meilisearch_cli::Document;
use std::path::Path;
use structopt::StructOpt;
use url::Url;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "meilisearch-cli",
    about = "CLI interface to Meilisearch to storing and retrieving Zettelkasten-style notes",
    author = "Steve <steve@little-fluffy.cloud>"
)]
struct Opt {
    /// switch on verbosity
    #[structopt(short)]
    verbose: bool,

    #[structopt(short, long, default_value = "http://127.0.0.1:7700")]
    host: String,

    #[structopt(subcommand)]
    import: Subcommands,
}

#[derive(Debug, StructOpt)]
enum Subcommands {
    /// Import frontmatter+markdown formatted files matching the unexpanded glob pattern
    Import { globpath: String },
}

pub fn glob_files(source: &str, verbosity: i8) -> Result<Paths, Box<dyn std::error::Error>> {
    let glob_path = Path::new(&source);
    let glob_str = shellexpand::tilde(glob_path.to_str().unwrap());

    if verbosity > 0 {
        println!("Sourcing Markdown documents matching : {}", glob_str);
    }

    Ok(glob(&glob_str).expect("Failed to read glob pattern"))
}

fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    Ok(())
}

fn main() -> Result<(), Report> {
    setup()?;

    let cli = Opt::clap().get_matches();
    let verbosity = cli.occurrences_of("v");
    let mut url_base = Url::parse(cli.value_of("host").unwrap())?;
    url_base.set_path("indexes/notes/documents");

    if let Some(cli) = cli.subcommand_matches("import") {
        let client = reqwest::blocking::Client::new();

        // Read the markdown files and post them to local Meilisearch
        for entry in glob_files(cli.value_of("globpath").unwrap(), verbosity as i8)
            .expect("Failed to read glob pattern")
        {
            match entry {
                // TODO convert this to iterator style using map/filter
                Ok(path) => {
                    if let Ok(mdfm_doc) = markdown_fm_doc::parse_file(&path) {
                        let doc: Vec<Document> = vec![mdfm_doc.into()];
                        let res = client
                            .post(url_base.as_ref())
                            .body(serde_json::to_string(&doc).unwrap())
                            .send()?;
                        if verbosity > 0 {
                            println!("✅ {:?}", res,);
                        }
                    } else {
                        eprintln!("❌ Failed to load file {}", path.display());
                    }
                }

                Err(e) => eprintln!("❌ {:?}", e),
            }
        }
    }

    Ok(())
}
