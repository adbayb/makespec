use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use super::{
    document::{
        Document, DocumentSummary, Kind, MAX_DOCUMENT_BYTES, Metadata, NewDocument, Status,
        parse_document, parse_id, render_document, slugify, update_metadata, validate_acceptance,
        validate_structure,
    },
    error::{MakespecError, Result},
};

const SEED_ADR_TITLE: &str = "Use ADRs for decisions and RFCs for changes";
const SEED_RFC_TITLE: &str = "Specify the RFC format";

#[derive(Clone, Debug)]
pub(crate) struct ProjectFile {
    pub relative_path: String,
    pub content: String,
    pub seed: Option<SeedIdentity>,
}

#[derive(Clone, Debug)]
pub(crate) struct SeedIdentity {
    pub id: String,
    pub title: String,
}

pub(crate) trait ProjectStore {
    fn initialize(&self, files: &[ProjectFile]) -> Result<()>;
    fn existing_documents(&self) -> Result<Vec<Document>>;
    fn documents(&self) -> Result<Vec<Document>>;
    fn catalog(&self) -> Result<Vec<DocumentSummary>>;
    fn document(&self, id: &str) -> Result<Document>;
    fn create_proposed(&self, new_document: NewDocument) -> Result<Document>;
    fn replace(&self, previous: &Document, replacement: &Document) -> Result<Document>;
}

pub(crate) fn initialize(
    store: &impl ProjectStore,
    date: &str,
    skill_files: &[(&str, &str)],
) -> Result<()> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| MakespecError::Invalid(format!("invalid initialization date `{date}`")))?;

    let adr = seed_document(Kind::Adr, SEED_ADR_TITLE, date, adr_seed_body())?;
    let rfc = seed_document(Kind::Rfc, SEED_RFC_TITLE, date, rfc_seed_body())?;

    for seed in [&adr, &rfc] {
        let errors = validate_acceptance(seed);

        if !errors.is_empty() {
            return Err(structure_error(
                &format!("built-in seed {} is invalid", seed.metadata.id),
                &errors,
            ));
        }
    }

    let mut files = vec![
        ProjectFile {
            relative_path: adr.path.clone(),
            content: adr.raw.clone(),
            seed: Some(seed_identity(&adr)),
        },
        ProjectFile {
            relative_path: rfc.path.clone(),
            content: rfc.raw.clone(),
            seed: Some(seed_identity(&rfc)),
        },
    ];

    for (relative_path, content) in skill_files {
        files.push(ProjectFile {
            relative_path: format!(".agents/skills/makespec/{relative_path}"),
            content: (*content).to_string(),
            seed: None,
        });
    }

    let existing = store.existing_documents()?;

    for seed in [&adr, &rfc] {
        if let Some(document) = existing
            .iter()
            .find(|document| document.metadata.id == seed.metadata.id)
        {
            if document.path != seed.path
                || document.metadata.title != seed.metadata.title
                || document.metadata.status != Status::Accepted
            {
                return Err(MakespecError::Invalid(format!(
                    "initialization conflict: existing {} does not match the accepted seed record",
                    document.path
                )));
            }

            let errors = validate_acceptance(document);

            if !errors.is_empty() {
                return Err(structure_error(
                    &format!("existing seed {} is invalid", seed.metadata.id),
                    &errors,
                ));
            }

            files.retain(|file| file.relative_path != document.path);
        }
    }

    store.initialize(&files)
}

fn seed_identity(document: &Document) -> SeedIdentity {
    SeedIdentity {
        id: document.metadata.id.clone(),
        title: document.metadata.title.clone(),
    }
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

    let slug = slugify(&title)?;
    let new_document = NewDocument {
        kind,
        title,
        slug,
        date: date.to_string(),
        body,
    };
    let validation_document = build_document(
        &format!("{}-0001", new_document.kind.id_prefix()),
        &new_document,
        Status::Proposed,
        "",
    )?;

    if validation_document.raw.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(MakespecError::Invalid(format!(
            "complete document exceeds {} bytes",
            MAX_DOCUMENT_BYTES
        )));
    }

    let errors = validate_acceptance(&validation_document);

    if !errors.is_empty() {
        return Err(structure_error(
            "document is incomplete or invalid",
            &errors,
        ));
    }

    store.create_proposed(new_document)
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
    if next_status != Status::Superseded && superseded_by.is_some() {
        return Err(MakespecError::Invalid(
            "`--by` is only valid when setting status to `superseded`".into(),
        ));
    }

    if next_status == Status::Superseded && superseded_by.is_none() {
        return Err(MakespecError::Invalid(
            "superseding a document requires `--by <ID>`".into(),
        ));
    }

    let documents = store.catalog()?;
    documents
        .iter()
        .find(|document| document.metadata.id == id)
        .ok_or_else(|| MakespecError::NotFound(id.to_string()))?;

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
    metadata.superseded_by = None;

    if next_status == Status::Accepted {
        let errors = validate_acceptance(&current);

        if !errors.is_empty() {
            return Err(structure_error(&format!("cannot accept {id}"), &errors));
        }
    } else if next_status == Status::Superseded {
        let target_id = superseded_by.expect("checked above");
        let (target_kind, _) = parse_id(target_id)?;
        let (current_kind, _, _) = current.metadata.validate()?;

        if target_kind != current_kind.id_prefix() || target_id == id {
            return Err(MakespecError::Invalid(format!(
                "`{target_id}` must be a different {} document",
                current_kind.as_str()
            )));
        }

        let target = documents
            .iter()
            .find(|document| document.metadata.id == target_id)
            .ok_or_else(|| MakespecError::NotFound(target_id.to_string()))?;

        if target.metadata.status != Status::Accepted {
            return Err(MakespecError::Invalid(format!(
                "replacement `{target_id}` must be accepted first"
            )));
        }

        metadata.superseded_by = Some(target_id.to_string());
    }

    let replacement = update_metadata(&current, metadata)?;

    store.replace(&current, &replacement)
}

pub(crate) fn check(store: &impl ProjectStore) -> Result<Vec<String>> {
    let documents = store.documents()?;
    let mut errors = Vec::new();
    let mut by_id = BTreeMap::new();
    let mut numbers = BTreeSet::new();

    for document in &documents {
        let path = &document.path;

        match document.metadata.validate() {
            Ok((kind, status, number)) => {
                let expected_prefix = format!("docs/{}/{number:04}-", kind.directory());
                let valid_filename = path
                    .strip_prefix(&expected_prefix)
                    .and_then(|filename| filename.strip_suffix(".md"))
                    .is_some_and(|slug| {
                        slugify(slug).is_ok_and(|normalized_slug| normalized_slug == slug)
                    });

                if !valid_filename {
                    errors.push(format!(
                        "{path}: filename must start with `{expected_prefix}` and end in a kebab-case `.md` slug"
                    ));
                }

                if !numbers.insert((kind, number)) {
                    errors.push(format!(
                        "{path}: duplicate {} identifier number {number:04}",
                        kind.as_str()
                    ));
                }

                by_id.insert(document.metadata.id.as_str(), document);
                let document_errors = if status == Status::Accepted {
                    validate_acceptance(document)
                } else {
                    validate_structure(document)
                };

                errors.extend(
                    document_errors
                        .into_iter()
                        .map(|error| format!("{path}: {error}")),
                );
            }
            Err(error) => errors.push(format!("{path}: {error}")),
        }
    }

    for document in &documents {
        let metadata = &document.metadata;
        let Some(target_id) = metadata.superseded_by.as_deref() else {
            continue;
        };

        match by_id.get(target_id) {
            Some(target)
                if target.metadata.validate().is_ok_and(|(target_kind, _, _)| {
                    metadata
                        .validate()
                        .is_ok_and(|(kind, _, _)| target_kind == kind)
                }) && matches!(
                    target.metadata.status,
                    Status::Accepted | Status::Deprecated | Status::Superseded
                ) => {}
            Some(_) => errors.push(format!(
                "{}: supersession target `{target_id}` has a different kind or invalid status",
                document.path
            )),
            None => errors.push(format!(
                "{}: supersession target `{target_id}` does not exist",
                document.path
            )),
        }

        if supersession_reaches(target_id, &metadata.id, &by_id) {
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

fn seed_document(kind: Kind, title: &str, date: &str, body: &str) -> Result<Document> {
    let slug = slugify(title)?;
    let metadata = Metadata {
        id: format!("{}-0001", kind.id_prefix()),
        title: title.to_string(),
        date: date.to_string(),
        status: Status::Accepted,
        superseded_by: None,
        extra: BTreeMap::new(),
    };
    let path = format!("docs/{}/{:04}-{slug}.md", kind.directory(), 1);
    let raw = render_document(&metadata, body)?;

    parse_document(&raw, path)
}

fn build_document(
    id: &str,
    new_document: &NewDocument,
    status: Status,
    path: &str,
) -> Result<Document> {
    let metadata = Metadata {
        id: id.to_string(),
        title: new_document.title.clone(),
        date: new_document.date.clone(),
        status,
        superseded_by: None,
        extra: BTreeMap::new(),
    };
    let raw = render_document(&metadata, &new_document.body)?;

    parse_document(&raw, path.to_string())
}

fn structure_error(context: &str, errors: &[String]) -> MakespecError {
    MakespecError::Invalid(format!("{context}: {}", errors.join("; ")))
}

fn adr_seed_body() -> &'static str {
    "## Context\n\nProject changes need two levels of design records: focused decisions and broader functionality proposals. A single format makes small choices hard to find and forces feature proposals into an unsuitable structure.\n\n## Decision\n\nUse ADRs for small, focused decisions and RFCs to specify functionality or other substantial changes. Keep both as Markdown files in `docs/adrs` and `docs/rfcs`, respectively. Give each kind its own incrementing ID, creation date, and front-matter status. Use an RFC when a change needs requirements, constraints, alternatives, and an adoption plan; record a contained choice as an ADR.\n\n## Consequences\n\nReviewers can trace proposals and decisions in Git. Authors must keep status and structure valid, and RFC authors must compare every option against the same criteria. The repository gains a small amount of documentation overhead, but does not need a service or database."
}

fn rfc_seed_body() -> &'static str {
    r#"## Summary

Define a consistent RFC format for proposals that specify functionality or substantial changes.

## Motivation

A shared format makes proposals easier to review and compare. It records user needs, measurable requirements, constraints, alternatives, and an adoption approach without requiring a hosted service.

## Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | An RFC specifies functionality requirements, constraints, and qualitative alternatives. |
| NFR-001 | SHOULD | An RFC remains readable and reviewable as plain Markdown in Git. |

## Constraints

| ID | Constraint |
| --- | --- |
| C-001 | The format must work with repository files and require no service or database. |

## Detailed design

Every RFC has YAML front matter containing `id`, `title`, `date`, and `status`. The ID prefix determines the document kind, and the filename retains its creation-time slug if the title later changes. The CLI supplies this metadata and the title heading. RFC bodies use the Summary, Motivation, Requirements, Constraints, Detailed design, Drawbacks, Alternatives, and Adoption strategy sections in this order. A Basic example and Unresolved questions section may be added when useful.

Requirements use `FR-001` and `NFR-001` IDs and priority (MUST, SHOULD, or MAY). Constraints use `C-001` IDs. In the comparison matrix, requirement cells use `Does not meet`, `Partially meets`, or `Fully meets`, followed by a brief explanation. Constraint cells use `PASS` or `FAIL`, followed by an explanation. The chosen option must fully meet each MUST requirement and pass each constraint. Explain the decision using priorities and evidence rather than an aggregate score.

## Drawbacks

Authors must provide concise evidence for each assessment, and hard constraints cannot be traded against requirement fit.

## Alternatives

| Criterion ID | Do nothing | Free-form Markdown | ADR-only records | Makespec format |
| --- | --- | --- | --- | --- |
| FR-001 | Does not meet - Proposals need not capture the required areas. | Does not meet - No structure requires all required areas. | Does not meet - ADRs do not specify functionality proposals. | Fully meets - The format requires functionality criteria, constraints, and comparisons. |
| NFR-001 | Partially meets - Markdown stays readable, but authors lack a shared structure. | Fully meets - Free-form Markdown remains easy to read and diff. | Fully meets - ADRs remain plain Markdown in Git. | Fully meets - The format uses reviewable Markdown tables. |
| C-001 | PASS - It requires no service or database. | PASS - It uses repository files only. | PASS - It uses repository files only. | PASS - The CLI reads and writes local Markdown files. |

**Chosen option:** Makespec format

The chosen format fully meets FR-001 and satisfies the hard constraint. It keeps records portable while making comparison criteria explicit.

## Adoption strategy

Initialize a repository with the CLI, which creates the accepted format RFC and adoption ADR. Use the template for new proposals, run `makespec check` before review and status changes, and revise the RFC in Git if the format changes."#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_seed_passes_acceptance_and_chosen_must_requirement_is_enforced() {
        let valid =
            seed_document(Kind::Rfc, SEED_RFC_TITLE, "2026-11-09", rfc_seed_body()).unwrap();

        assert!(validate_acceptance(&valid).is_empty());

        let body = rfc_seed_body().replace(
            "Fully meets - The format requires functionality criteria, constraints, and comparisons.",
            "Does not meet - The format requires functionality criteria, constraints, and comparisons.",
        );
        let invalid = seed_document(Kind::Rfc, SEED_RFC_TITLE, "2026-11-09", &body).unwrap();

        assert!(validate_structure(&invalid).iter().any(|error| {
            error.contains("chosen option does not fully meet a MUST requirement")
        }));
    }

    #[test]
    fn chosen_option_must_pass_every_constraint() {
        let body = rfc_seed_body().replace(
            "PASS - The CLI reads and writes local Markdown files.",
            "FAIL - The CLI reads and writes local Markdown files.",
        );
        let document = seed_document(Kind::Rfc, SEED_RFC_TITLE, "2026-11-09", &body).unwrap();

        assert!(
            validate_structure(&document)
                .iter()
                .any(|error| error == "chosen option fails a constraint")
        );
    }

    #[test]
    fn rfc_comparison_requires_labels_instead_of_numeric_scores() {
        let body = rfc_seed_body().replace(
            "Fully meets - The format requires functionality criteria, constraints, and comparisons.",
            "3 - The format requires functionality criteria, constraints, and comparisons.",
        );
        let document = seed_document(Kind::Rfc, SEED_RFC_TITLE, "2026-11-09", &body).unwrap();

        assert!(
            validate_structure(&document)
                .iter()
                .any(|error| { error.contains("invalid requirement label `3` for `FR-001`") })
        );
    }

    #[test]
    fn rfc_rejects_empty_requirement_descriptions() {
        let body = rfc_seed_body().replace(
            "| FR-001 | MUST | An RFC specifies functionality requirements, constraints, and qualitative alternatives. |",
            "| FR-001 | MUST | |",
        );
        let document = seed_document(Kind::Rfc, SEED_RFC_TITLE, "2026-11-09", &body).unwrap();

        assert!(
            validate_structure(&document)
                .iter()
                .any(|error| error.contains("needs a description"))
        );
    }

    #[test]
    fn rfc_reports_requirement_and_constraint_errors_together() {
        let body = rfc_seed_body()
            .replace(
                "| FR-001 | MUST | An RFC specifies functionality requirements, constraints, and qualitative alternatives. |",
                "| FR-001 | MUST | |",
            )
            .replace(
                "PASS - The CLI reads and writes local Markdown files.",
                "FAIL - The CLI reads and writes local Markdown files.",
            );
        let document = seed_document(Kind::Rfc, SEED_RFC_TITLE, "2026-11-09", &body).unwrap();
        let errors = validate_structure(&document);

        assert!(
            errors
                .iter()
                .any(|error| error == "criterion `FR-001` needs a description")
        );
        assert!(
            errors
                .iter()
                .any(|error| error == "chosen option fails a constraint")
        );
    }
}
