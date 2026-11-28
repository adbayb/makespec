use std::process::ExitCode;

use crate::{
    adapters::{
        cli::for_managing_documents::{CommandLine, report_failure},
        embedded::for_obtaining_skill_files::EmbeddedSkillFiles,
        filesystem::for_storing_documents::FilesystemStore,
    },
    hexagon::{Makespec, error::Result},
};

/// Production entry: wires the filesystem and embedded skill into the hexagon,
/// then drives it from the command line.
pub(crate) fn run() -> ExitCode {
    let (project, store) = match driven_adapters() {
        Ok(adapters) => adapters,
        Err(error) => return report_failure(&error),
    };
    let makespec = Makespec::new(store, EmbeddedSkillFiles);

    CommandLine::new(&makespec, &project).run()
}

fn driven_adapters() -> Result<(std::path::PathBuf, FilesystemStore)> {
    let project = std::env::current_dir()?;
    let store = FilesystemStore::new(&project)?;

    Ok((project, store))
}
