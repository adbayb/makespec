use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use super::{
    document::{
        Document, DocumentSummary, Kind, NewDocument, Status, parse_id, slugify, update_metadata,
        validate_document, validate_document_structure,
    },
    error::{MakespecError, Result},
    ports::{ProjectFile, ProjectStore},
};

const SEED_DECISION_TITLE: &str = "Use Makespec to manage decisions and specifications";

pub(crate) fn initialize(
    store: &impl ProjectStore,
    date: &str,
    skill_files: &[(&str, &str)],
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

    let existing = store.existing_documents()?;
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

    files.extend(
        skill_files
            .iter()
            .map(|(relative_path, content)| ProjectFile {
                relative_path: format!(".agents/skills/makespec/{relative_path}"),
                content: (*content).to_string(),
            }),
    );

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

    store.initialize(&files)
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

pub(crate) fn create(
    store: &impl ProjectStore,
    kind: Kind,
    title: String,
    body: String,
    date: &str,
) -> Result<Document> {
    if body.trim_start().starts_with("---") {
        return Err(MakespecError::Invalid(
            "new accepts only the Markdown body; omit YAML front matter".into(),
        ));
    }

    let new_document = NewDocument {
        kind,
        slug: slugify(&title)?,
        title,
        date: date.to_string(),
        body,
    };
    let errors = validate_document_structure(&new_document.build(1, Status::Proposed)?);

    if !errors.is_empty() {
        return Err(structure_error("document structure is invalid", &errors));
    }

    store.create_proposal(new_document)
}

pub(crate) fn list(store: &impl ProjectStore) -> Result<Vec<DocumentSummary>> {
    store.catalog()
}

pub(crate) fn change_status(
    store: &impl ProjectStore,
    id: &str,
    next_status: Status,
    superseded_by: Option<&str>,
) -> Result<Document> {
    match (next_status, superseded_by) {
        (Status::Superseded, None) => {
            return Err(MakespecError::Invalid(
                "superseding a document requires a replacement ID".into(),
            ));
        }
        (Status::Superseded, Some(_)) | (_, None) => {}
        (_, Some(_)) => {
            return Err(MakespecError::Invalid(
                "a replacement ID is only valid when setting status to `superseded`".into(),
            ));
        }
    }

    let current = store.document(id)?;

    if current.metadata.status == next_status {
        if let Some(target) = superseded_by
            && current.metadata.superseded_by.as_deref() != Some(target)
        {
            return Err(MakespecError::Invalid(format!(
                "{id} is already superseded by a different document"
            )));
        }

        return Ok(current);
    }

    if !current.metadata.status.can_transition_to(next_status) {
        return Err(MakespecError::Invalid(format!(
            "cannot change `{id}` from {} to {next_status}",
            current.metadata.status
        )));
    }

    let mut metadata = current.metadata.clone();
    metadata.status = next_status;
    metadata.superseded_by = superseded_by
        .map(|target_id| accepted_replacement(store, &current, target_id))
        .transpose()?;

    let replacement = update_metadata(&current, metadata)?;

    if next_status == Status::Accepted {
        ensure_valid(&replacement, &format!("cannot set {id} to {next_status}"))?;
    }

    store.replace(&current, &replacement)?;

    Ok(replacement)
}

fn accepted_replacement(
    store: &impl ProjectStore,
    current: &Document,
    target_id: &str,
) -> Result<String> {
    let (target_kind, _) = parse_id(target_id)?;
    let (current_kind, _) = parse_id(&current.metadata.id)?;

    if target_kind != current_kind || target_id == current.metadata.id {
        return Err(MakespecError::Invalid(format!(
            "`{target_id}` must be a different {current_kind} document"
        )));
    }

    let target = store
        .catalog()?
        .into_iter()
        .find(|document| document.metadata.id == target_id)
        .ok_or_else(|| MakespecError::NotFound(target_id.to_string()))?;

    if target.metadata.status != Status::Accepted {
        return Err(MakespecError::Invalid(format!(
            "replacement `{target_id}` must be accepted first"
        )));
    }

    Ok(target.metadata.id)
}

pub(crate) fn check(store: &impl ProjectStore) -> Result<Vec<String>> {
    let documents = store.documents()?;
    let mut errors = Vec::new();
    let mut by_id = BTreeMap::new();

    for document in &documents {
        let path = &document.path;
        let (kind, number) = match document.metadata.validate() {
            Ok(identity) => identity,
            Err(error) => {
                errors.push(format!("{path}: {error}"));
                continue;
            }
        };
        let expected_prefix = format!("docs/{}/{number:04}-", kind.directory());
        let valid_filename = path
            .strip_prefix(&expected_prefix)
            .and_then(|filename| filename.strip_suffix(".md"))
            .is_some_and(|slug| slugify(slug).is_ok_and(|normalized_slug| normalized_slug == slug));

        if !valid_filename {
            errors.push(format!(
                "{path}: filename must start with `{expected_prefix}` and end in a kebab-case `.md` slug"
            ));
        }

        if by_id
            .insert(document.metadata.id.as_str(), document)
            .is_some()
        {
            errors.push(format!(
                "{path}: duplicate {kind} identifier number {number:04}"
            ));
        }

        errors.extend(
            validate_document(document)
                .into_iter()
                .map(|error| format!("{path}: {error}")),
        );
    }

    for document in &documents {
        let Some(target_id) = document.metadata.superseded_by.as_deref() else {
            continue;
        };

        match by_id.get(target_id) {
            Some(target)
                if matches!(
                    target.metadata.status,
                    Status::Accepted | Status::Deprecated | Status::Superseded
                ) => {}
            Some(_) => errors.push(format!(
                "{}: supersession target `{target_id}` has an invalid status",
                document.path
            )),
            None => errors.push(format!(
                "{}: supersession target `{target_id}` does not exist",
                document.path
            )),
        }

        if supersession_reaches(target_id, &document.metadata.id, &by_id) {
            errors.push(format!(
                "{}: supersession chain contains a cycle",
                document.path
            ));
        }
    }

    Ok(errors)
}

fn supersession_reaches(start: &str, sought: &str, documents: &BTreeMap<&str, &Document>) -> bool {
    let mut current = start;
    let mut visited = BTreeSet::new();

    while visited.insert(current) {
        if current == sought {
            return true;
        }

        let Some(next) = documents
            .get(current)
            .and_then(|document| document.metadata.superseded_by.as_deref())
        else {
            return false;
        };

        current = next;
    }

    true
}

fn ensure_valid(document: &Document, context: &str) -> Result<()> {
    let errors = validate_document(document);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(structure_error(context, &errors))
    }
}

fn structure_error(context: &str, errors: &[String]) -> MakespecError {
    MakespecError::Invalid(format!("{context}: {}", errors.join("; ")))
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
