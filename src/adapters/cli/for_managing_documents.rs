use std::{
    io::{self, IsTerminal, Read},
    path::Path,
    process::ExitCode,
};

use chrono::Local;
use clap::{CommandFactory, Parser, Subcommand};

use crate::hexagon::{
    document::{DocumentSummary, Kind, MAX_DOCUMENT_BYTES, Status},
    error::{MakespecError, Result},
    ports::driver::for_managing_documents::{
        ChangeDocumentStatusInput, CreateDocumentInput, ForManagingDocuments,
    },
};

#[derive(Parser)]
#[command(
    name = "makespec",
    version,
    disable_version_flag = true,
    disable_help_subcommand = true,
    about = "Manage decisions and specifications in your repository"
)]
struct Arguments {
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    version: Option<bool>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Init,
    New {
        #[arg(value_name = "decision|specification")]
        kind: Kind,
        title: String,
    },
    List,
    Status {
        id: String,
        status: Status,
        replacement_id: Option<String>,
    },
    Check,
}

/// Driver adapter that runs one terminal command against the driver port.
pub(crate) struct CommandLine<'a, App> {
    app: &'a App,
    project: &'a Path,
}

impl<'a, App: ForManagingDocuments> CommandLine<'a, App> {
    pub(crate) fn new(app: &'a App, project: &'a Path) -> Self {
        Self { app, project }
    }

    pub(crate) fn run(&self) -> ExitCode {
        match self.execute(Arguments::parse()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => report_failure(&error),
        }
    }

    fn execute(&self, arguments: Arguments) -> Result<()> {
        let Some(command) = arguments.command else {
            Arguments::command().print_help()?;
            println!();

            return Ok(());
        };

        match command {
            Command::Init => {
                self.app.initialize_project(&today())?;

                println!(
                    "Initialized Makespec in {}",
                    sanitize_terminal_text(
                        &std::fs::canonicalize(self.project)?.display().to_string()
                    )
                );
            }
            Command::New { kind, title } => {
                if io::stdin().is_terminal() {
                    return Err(MakespecError::Invalid(
                        "new reads a Markdown body from stdin; pipe a Markdown body into the command"
                            .into(),
                    ));
                }

                let document = self.app.create_document(CreateDocumentInput {
                    kind,
                    title,
                    body: read_stdin_bounded()?,
                    date: today(),
                })?;

                println!("Created {} ({})", document.metadata.id, document.path);
            }
            Command::List => {
                let documents = self.app.get_summaries()?;

                print_document_list(&documents);
            }
            Command::Status {
                id,
                status,
                replacement_id,
            } => {
                let document = self.app.change_document_status(ChangeDocumentStatusInput {
                    id: &id,
                    status,
                    replacement_id: replacement_id.as_deref(),
                })?;

                println!("{} is {}", document.metadata.id, document.metadata.status);
            }
            Command::Check => {
                let errors = self.app.check_documents()?;

                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("error: {}", sanitize_terminal_text(error));
                    }

                    return Err(MakespecError::Invalid(format!(
                        "{} validation error(s)",
                        errors.len()
                    )));
                }

                println!("Makespec documents are valid");
            }
        }

        Ok(())
    }
}

/// Prints a failure the way every command reports errors.
pub(crate) fn report_failure(error: &MakespecError) -> ExitCode {
    eprintln!("makespec: {}", sanitize_terminal_text(&error.to_string()));

    ExitCode::FAILURE
}

fn print_document_list(documents: &[DocumentSummary]) {
    if documents.is_empty() {
        println!("No records found.");

        return;
    }

    let mut has_printed_group = false;

    for (heading, kind) in [
        ("Decisions", Kind::Decision),
        ("Specifications", Kind::Specification),
    ] {
        let group: Vec<_> = documents
            .iter()
            .filter(|document| document.metadata.id.starts_with(kind.id_prefix()))
            .collect();

        if group.is_empty() {
            continue;
        }

        if has_printed_group {
            println!();
        }

        println!("{heading} ({})", group.len());
        has_printed_group = true;

        for (index, document) in group.iter().enumerate() {
            if index > 0 {
                println!();
            }

            println!(
                "  {}  {}",
                sanitize_terminal_text(&document.metadata.id),
                sanitize_terminal_text(&document.metadata.title)
            );
            println!(
                "    Status: {}  Date: {}",
                document.metadata.status,
                sanitize_terminal_text(&document.metadata.date)
            );
            println!("    Path: {}", sanitize_terminal_text(&document.path));
        }
    }
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

fn today() -> String {
    Local::now().date_naive().to_string()
}

fn sanitize_terminal_text(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\n' {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect()
}
