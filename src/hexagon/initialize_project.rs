use chrono::NaiveDate;

use super::{
    document::{Document, Kind, NewDocument, Status, ensure_valid, slugify},
    error::{MakespecError, Result},
    ports::driven::{
        for_obtaining_skill_files::ForObtainingSkillFiles,
        for_storing_documents::{ForStoringDocuments, ProjectFile},
    },
};

const SEED_DECISION_TITLE: &str = "Use Makespec to manage decisions and specifications";

pub(super) fn initialize_project(
    store: &impl ForStoringDocuments,
    skill: &impl ForObtainingSkillFiles,
    date: &str,
) -> Result<()> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| MakespecError::Invalid(format!("invalid initialization date `{date}`")))?;

    let seed = NewDocument {
        kind: Kind::Decision,
        title: SEED_DECISION_TITLE.into(),
        slug: slugify(SEED_DECISION_TITLE)?,
        date: date.into(),
        body: DECISION_SEED_BODY.into(),
    }
    .build(1, Status::Accepted)?;

    ensure_valid(
        &seed,
        &format!("built-in seed {} is invalid", seed.metadata.id),
    )?;

    let existing = store.get_existing_documents()?;
    let existing_seed = existing
        .iter()
        .find(|document| document.metadata.id == seed.metadata.id);
    let mut files = Vec::new();

    match existing_seed {
        Some(document) => validate_existing_seed(document, &seed)?,
        None => files.push(ProjectFile {
            relative_path: seed.path,
            content: seed.raw,
        }),
    }

    files.extend(skill.get_skill_files().iter().map(|file| ProjectFile {
        relative_path: format!(".agents/skills/makespec/{}", file.relative_path),
        content: file.content.to_string(),
    }));

    if !existing.iter().any(|document| {
        document
            .metadata
            .id
            .starts_with(Kind::Specification.id_prefix())
    }) {
        files.push(ProjectFile {
            relative_path: "docs/specifications/.gitkeep".into(),
            content: String::new(),
        });
    }

    store.create_files(&files)
}

fn validate_existing_seed(existing: &Document, seed: &Document) -> Result<()> {
    if existing.path != seed.path
        || existing.metadata.title != seed.metadata.title
        || existing.metadata.status != Status::Accepted
    {
        return Err(MakespecError::Invalid(format!(
            "initialization conflict: existing {} does not match the accepted seed record",
            existing.path
        )));
    }

    ensure_valid(
        existing,
        &format!("existing seed {} is invalid", seed.metadata.id),
    )
}

const DECISION_SEED_BODY: &str = r#"## Context

Contributors need a shared way to document proposed functionality and preserve the reasoning behind focused choices. Humans and coding agents need to find and review these records in the repository.

## Decision

Use Makespec to create, validate, and manage the lifecycle of decisions and specifications stored as Markdown. Makespec relies on the architecture decision record (ADR) format for focused decisions and the request for comments (RFC) format for functionality specifications, including product requirements documents (PRDs).

Keep decisions in `docs/decisions/` and specifications in `docs/specifications/`. Contributors may write and edit content with ordinary editors or agent tools.

## Consequences

Contributors gain stable references and a shared lifecycle. They must keep records current and validate changes with `makespec check`. Makespec becomes a contributor tool, but reading the records does not require the binary. Contributors can review and version records in Git.

Follow the installed skill, templates, and command reference for authoring and review rather than maintaining a separate workflow specification.

## References

- [Contribution workflow](../../.agents/skills/makespec/SKILL.md#workflow) defines authoring, planning, and approval boundaries.
- [Decision template](../../.agents/skills/makespec/references/decision-template.md) and [specification template](../../.agents/skills/makespec/references/specification-template.md) define record structure.
- [Command reference](../../.agents/skills/makespec/references/cli.md) defines syntax and lifecycle transitions."#;

#[cfg(test)]
mod tests {
    use crate::{
        hexagon::ports::driver::for_managing_documents::ForManagingDocuments,
        startup::test::{decision_file, makespec},
    };

    const SEED_PATH: &str =
        "docs/decisions/0001-use-makespec-to-manage-decisions-and-specifications.md";

    #[test]
    fn initialization_installs_the_seed_skill_and_placeholder_idempotently() {
        let (app, documents) = makespec();

        app.initialize_project("2026-11-09").unwrap();

        let seed = documents.read_file(SEED_PATH).unwrap();

        assert!(seed.contains("status: accepted"));
        assert_eq!(
            documents.read_file(".agents/skills/makespec/SKILL.md"),
            Some("# Stub skill\n".into())
        );
        assert_eq!(
            documents.read_file("docs/specifications/.gitkeep"),
            Some(String::new())
        );

        app.initialize_project("2026-11-10").unwrap();

        assert_eq!(documents.read_file(SEED_PATH).unwrap(), seed);
        assert!(app.check_documents().unwrap().is_empty());
    }

    #[test]
    fn initialization_keeps_an_edited_seed_but_rejects_a_conflicting_one() {
        let (app, documents) = makespec();

        app.initialize_project("2026-11-09").unwrap();

        let original = documents.read_file(SEED_PATH).unwrap();
        let edited = original.replace("date: 2026-11-09", "date: 2026-11-10");

        documents.write_file(SEED_PATH, &edited);
        app.initialize_project("2026-11-09").unwrap();

        assert_eq!(documents.read_file(SEED_PATH).unwrap(), edited);

        documents.write_file(
            SEED_PATH,
            &original.replace("status: accepted", "status: proposed"),
        );

        let error = app.initialize_project("2026-11-09").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("does not match the accepted seed record")
        );
    }

    #[test]
    fn initialization_rejects_a_seed_id_at_another_path() {
        let (app, documents) = makespec();

        documents.write_file(
            "docs/decisions/0001-other.md",
            &decision_file(
                "ADR-0001",
                "Use Makespec to manage decisions and specifications",
                "accepted",
                "",
            ),
        );

        assert!(app.initialize_project("2026-11-09").is_err());
    }

    #[test]
    fn initialization_skips_the_placeholder_when_specifications_exist() {
        let (app, documents) = makespec();
        let specification = decision_file("ADR-0001", "Placeholder", "proposed", "")
            .replace("ADR-0001", "RFC-0001");

        documents.write_file("docs/specifications/0001-placeholder.md", &specification);
        app.initialize_project("2026-11-09").unwrap();

        assert_eq!(documents.read_file("docs/specifications/.gitkeep"), None);
    }

    #[test]
    fn initialization_rejects_invalid_dates_without_writing() {
        let (app, documents) = makespec();

        assert!(app.initialize_project("2026-13-01").is_err());
        assert_eq!(documents.read_file(SEED_PATH), None);
    }
}
