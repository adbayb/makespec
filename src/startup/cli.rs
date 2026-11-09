use std::{
    io::{self, IsTerminal, Read},
    path::PathBuf,
};

use clap::{CommandFactory, Parser, Subcommand};

use crate::{
    adapters::{embedded, filesystem::FilesystemStore},
    core::{
        document::{Kind, MAX_DOCUMENT_BYTES, Status, today},
        error::{MakespecError, Result},
        service,
    },
    sanitize_terminal_text,
};

#[derive(Parser)]
#[command(
    name = "makespec",
    version,
    about = "Manage ADRs and RFCs in your repository"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CommandLine>,
}

#[derive(Subcommand)]
enum CommandLine {
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        no_skills: bool,
    },
    New {
        kind: Kind,
        #[arg(long)]
        title: String,
    },
    List {
        kind: Option<Kind>,
        #[arg(long)]
        status: Option<Status>,
    },
    Status {
        id: String,
        status: Status,
        #[arg(long)]
        by: Option<String>,
    },
    Check,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();

        return Ok(());
    };

    match command {
        CommandLine::Init { path, no_skills } => {
            let store = FilesystemStore::new(&path)?;
            let skills = if no_skills {
                &[]
            } else {
                embedded::SKILL_FILES
            };

            service::initialize(&store, &today(), skills)?;

            println!(
                "Initialized Makespec in {}",
                sanitize_terminal_text(&std::fs::canonicalize(path)?.display().to_string())
            );
        }
        CommandLine::New { kind, title } => {
            if io::stdin().is_terminal() {
                return Err(MakespecError::Invalid(
                    "new reads a Markdown body from stdin; pipe a Markdown body into the command"
                        .into(),
                ));
            }

            let body = read_stdin_bounded()?;
            let store = FilesystemStore::new(&std::env::current_dir()?)?;
            let document = service::create(&store, kind, title, body, &today())?;

            println!("Created {} ({})", document.metadata.id, document.path);
        }
        CommandLine::List { kind, status } => {
            let store = FilesystemStore::new(&std::env::current_dir()?)?;
            let documents = service::list(&store)?;

            for document in documents.into_iter().filter(|document| {
                kind.is_none_or(|filter| document.kind == filter)
                    && status.is_none_or(|filter| document.metadata.status == filter)
            }) {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    document.metadata.id,
                    document.metadata.status,
                    document.metadata.date,
                    sanitize_terminal_text(&document.metadata.title),
                    sanitize_terminal_text(&document.path)
                );
            }
        }
        CommandLine::Status { id, status, by } => {
            let store = FilesystemStore::new(&std::env::current_dir()?)?;
            let document = service::change_status(&store, &id, status, by.as_deref())?;

            println!("{} is {}", document.metadata.id, document.metadata.status);
        }
        CommandLine::Check => {
            let store = FilesystemStore::new(&std::env::current_dir()?)?;
            let errors = service::check(&store)?;

            if errors.is_empty() {
                println!("Makespec documents are valid");
            } else {
                for error in &errors {
                    eprintln!("error: {}", sanitize_terminal_text(error));
                }

                return Err(MakespecError::Invalid(format!(
                    "{} validation error(s)",
                    errors.len()
                )));
            }
        }
    }

    Ok(())
}

fn read_stdin_bounded() -> Result<String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)?;

    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(MakespecError::Invalid(format!(
            "stdin body exceeds {MAX_DOCUMENT_BYTES} bytes"
        )));
    }

    String::from_utf8(bytes)
        .map_err(|_| MakespecError::Invalid("stdin body must be valid UTF-8".into()))
}
