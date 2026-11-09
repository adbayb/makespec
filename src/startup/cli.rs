use std::{
    io::{self, IsTerminal, Read},
    path::PathBuf,
    process::Command,
};

use clap::{CommandFactory, Parser, Subcommand};

use crate::{
    adapters::{embedded, filesystem::FilesystemStore},
    core::{
        document::{Kind, Status, today},
        error::{MakespecError, Result},
        service::{self},
    },
};

const MAX_STDIN_BYTES: u64 = crate::core::document::MAX_DOCUMENT_BYTES;

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
        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            return super::tui::run();
        }

        Cli::command().print_help()?;
        println!();

        return Ok(());
    };

    match command {
        CommandLine::Init { path, no_skills } => {
            let store = FilesystemStore::new(&path)?;
            let skills = if no_skills {
                Vec::new()
            } else {
                embedded::skill_files().into_iter().collect::<Vec<_>>()
            };

            service::initialize(&store, &today(), &skills)?;

            println!(
                "Initialized Makespec in {}",
                sanitize_terminal_text(&std::fs::canonicalize(path)?.display().to_string())
            );
        }
        CommandLine::New { kind, title } => {
            if io::stdin().is_terminal() {
                return Err(MakespecError::Invalid(
                    "new reads a Markdown body from stdin; pipe the body or use the TUI".into(),
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
                    document.metadata.title,
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

pub(crate) fn edit_with_editor(document: &crate::core::document::Document) -> Result<String> {
    edit_text_with_editor(&document.raw)
}

pub(crate) fn edit_text_with_editor(initial: &str) -> Result<String> {
    let editor = configured_editor()?;
    let mut temporary = tempfile::NamedTempFile::new()?;
    use std::io::Write;

    temporary.write_all(initial.as_bytes())?;
    temporary.as_file().sync_all()?;
    let path = temporary.path().to_path_buf();

    let status = {
        let _guard = super::tui::suspend_terminal()?;

        Command::new(&editor.program)
            .args(&editor.arguments)
            .arg(&path)
            .status()?
    };

    if !status.success() {
        return Err(MakespecError::Invalid(format!(
            "editor exited with status {status}"
        )));
    }

    let mut file = std::fs::File::open(path)?;
    let mut contents = Vec::new();
    Read::take(&mut file, MAX_STDIN_BYTES + 1).read_to_end(&mut contents)?;

    if contents.len() as u64 > MAX_STDIN_BYTES {
        return Err(MakespecError::Invalid(format!(
            "edited document exceeds {MAX_STDIN_BYTES} bytes"
        )));
    }

    String::from_utf8(contents)
        .map_err(|_| MakespecError::Invalid("edited document is not UTF-8".into()))
}

struct Editor {
    program: String,
    arguments: Vec<String>,
}

fn configured_editor() -> Result<Editor> {
    let configured = std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| {
            MakespecError::Invalid(
                "no editor configured; set VISUAL or EDITOR to an executable path".into(),
            )
        })?;
    let words = split_editor_words(&configured)?;

    let Some((program, arguments)) = words.split_first() else {
        return Err(MakespecError::Invalid(
            "editor configuration is empty".into(),
        ));
    };

    Ok(Editor {
        program: program.clone(),
        arguments: arguments.to_vec(),
    })
}

fn split_editor_words(value: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            word.push(character);
            escaped = false;
            continue;
        }

        match (quote, character) {
            (Some('"'), '\\') | (None, '\\') => escaped = true,
            (Some(current), character) if current == character => quote = None,
            (None, '"' | '\'') => quote = Some(character),
            (None, character) if character.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            _ => word.push(character),
        }
    }

    if escaped || quote.is_some() {
        return Err(MakespecError::Invalid(
            "editor configuration has an unclosed quote or escape".into(),
        ));
    }

    if !word.is_empty() {
        words.push(word);
    }

    Ok(words)
}

fn read_stdin_bounded() -> Result<String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(MAX_STDIN_BYTES + 1)
        .read_to_end(&mut bytes)?;

    if bytes.len() as u64 > MAX_STDIN_BYTES {
        return Err(MakespecError::Invalid(format!(
            "stdin body exceeds {MAX_STDIN_BYTES} bytes"
        )));
    }

    String::from_utf8(bytes)
        .map_err(|_| MakespecError::Invalid("stdin body must be valid UTF-8".into()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_arguments_support_quotes_without_shell_expansion() {
        assert_eq!(
            split_editor_words("code --wait 'file with spaces'").unwrap(),
            ["code", "--wait", "file with spaces"]
        );
        assert!(split_editor_words("editor 'unfinished").is_err());
    }
}
