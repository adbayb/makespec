use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;

use super::{
    document::{
        Document, DocumentSummary, Kind, NewDocument, Status, parse_id, slugify, update_metadata,
        validate_document,
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

impl SeedIdentity {
    pub(crate) fn validate(&self, document: &Document, expected_path: &str) -> Result<()> {
        if document.path != expected_path
            || document.metadata.id != self.id
            || document.metadata.title != self.title
            || document.metadata.status != Status::Accepted
        {
            return Err(MakespecError::Invalid(format!(
                "initialization conflict: existing {} does not match the accepted seed record",
                document.path
            )));
        }

        let errors = validate_document(document);

        if !errors.is_empty() {
            return Err(structure_error(
                &format!("existing seed {} is invalid", self.id),
                &errors,
            ));
        }

        Ok(())
    }
}

pub(crate) trait ProjectStore {
    fn initialize(&self, files: &[ProjectFile]) -> Result<()>;
    fn existing_documents(&self) -> Result<Vec<Document>>;
    fn documents(&self) -> Result<Vec<Document>>;
    fn catalog(&self) -> Result<Vec<DocumentSummary>>;
    fn document(&self, id: &str) -> Result<Document>;
    fn create_draft(&self, new_document: NewDocument) -> Result<Document>;
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
        let errors = validate_document(seed);

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
            seed_identity(seed).validate(document, &seed.path)?;
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
    let validation_document = new_document.build(1, Status::Draft)?;

    let errors = validate_document(&validation_document);

    if !errors.is_empty() {
        return Err(structure_error("document structure is invalid", &errors));
    }

    store.create_draft(new_document)
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
            "a replacement ID is only valid when setting status to `superseded`".into(),
        ));
    }

    if next_status == Status::Superseded && superseded_by.is_none() {
        return Err(MakespecError::Invalid(
            "superseding a document requires a replacement ID".into(),
        ));
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
    metadata.superseded_by = None;

    if next_status == Status::Superseded {
        let target_id = superseded_by.expect("checked above");
        let (target_kind, _) = parse_id(target_id)?;
        let (current_kind, _) = current.metadata.validate()?;

        if target_kind != current_kind.id_prefix() || target_id == id {
            return Err(MakespecError::Invalid(format!(
                "`{target_id}` must be a different {} document",
                current_kind.as_str()
            )));
        }

        let documents = store.catalog()?;
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

    if matches!(next_status, Status::Proposed | Status::Accepted) {
        let errors = validate_document(&replacement);

        if !errors.is_empty() {
            return Err(structure_error(
                &format!("cannot set {id} to {next_status}"),
                &errors,
            ));
        }
    }

    store.replace(&current, &replacement)
}

pub(crate) fn check(store: &impl ProjectStore) -> Result<Vec<String>> {
    let documents = store.documents()?;
    let mut errors = Vec::new();
    let mut by_id = BTreeMap::new();

    for document in &documents {
        let path = &document.path;

        match document.metadata.validate() {
            Ok((kind, number)) => {
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

                if by_id
                    .insert(document.metadata.id.as_str(), document)
                    .is_some()
                {
                    errors.push(format!(
                        "{path}: duplicate {} identifier number {number:04}",
                        kind.as_str()
                    ));
                }

                errors.extend(
                    validate_document(document)
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
    NewDocument {
        kind,
        title: title.to_string(),
        slug: slugify(title)?,
        date: date.to_string(),
        body: body.to_string(),
    }
    .build(1, Status::Accepted)
}

fn structure_error(context: &str, errors: &[String]) -> MakespecError {
    MakespecError::Invalid(format!("{context}: {}", errors.join("; ")))
}

fn adr_seed_body() -> &'static str {
    "## Context\n\nProject changes need two levels of design records: focused decisions and broader functionality proposals. A single format makes small choices hard to find and forces feature proposals into an unsuitable structure.\n\n## Decision\n\nUse ADRs for small, focused decisions and RFCs to specify functionality or other substantial changes. Keep both as Markdown files in `docs/adrs` and `docs/rfcs`, respectively. Give each kind its own incrementing ID, creation date, and front-matter status. Use an RFC when a change needs requirements, constraints, alternatives, and an adoption plan; record a contained choice as an ADR.\n\n## Consequences\n\nReviewers can trace proposals and decisions in Git. Authors must keep status and structure valid, and RFC authors must compare every option against the same criteria. The repository gains a small amount of documentation overhead, but does not need a service or database."
}

fn rfc_seed_body() -> &'static str {
    r#"## Context

Define a consistent RFC format for functionality proposals. Authors need a reviewable record of the problem, evaluation criteria, options, chosen design, and adoption approach without a hosted service.

## Requirements and constraints

### Requirements

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-001 | MUST | An RFC specifies functionality requirements, constraints, and qualitative alternatives. |
| NFR-001 | SHOULD | An RFC remains readable and reviewable as plain Markdown in Git. |

### Constraints

| ID | Constraint |
| --- | --- |
| C-001 | The format must work with repository files and require no service or database. |

## Options

- Do nothing: leave proposal formats to individual authors.
- Free-form Markdown: record proposals without required sections or comparison rules.
- ADR-only records: reuse the focused decision format for functionality proposals.
- Makespec format: use shared criteria, an options comparison, and a design for the chosen option.

| Criterion ID | Do nothing | Free-form Markdown | ADR-only records | Makespec format |
| --- | --- | --- | --- | --- |
| FR-001 | Does not meet - Proposals need not capture the required areas. | Does not meet - No structure requires all required areas. | Does not meet - ADRs do not specify functionality proposals. | Fully meets - The format requires functionality criteria, constraints, and comparisons. |
| NFR-001 | Partially meets - Markdown stays readable, but authors lack a shared structure. | Fully meets - Free-form Markdown remains easy to read and diff. | Fully meets - ADRs remain plain Markdown in Git. | Fully meets - The format uses reviewable Markdown tables. |
| C-001 | PASS - It requires no service or database. | PASS - It uses repository files only. | PASS - It uses repository files only. | PASS - The CLI reads and writes local Markdown files. |

**Chosen option:** Makespec format

The chosen format fully meets FR-001 and satisfies the hard constraint. It keeps records portable while making comparison criteria explicit.

## Design

Every RFC has YAML front matter containing `id`, `title`, `date`, and `status`. The CLI supplies this metadata and the title heading. The ID prefix determines the document kind, and the filename retains its creation-time slug if the title later changes.

RFC bodies use Context, Requirements and constraints, Options, Design, and Adoption in that order. Context starts with a short proposal summary and explains the problem and scope. Put functional requirements in a `### Requirements` table, adding non-functional requirements only when relevant. Add a separate `### Constraints` table only for genuine hard boundaries. An optional Open questions section may follow Adoption.

Functional requirements use unique `FR-001`-style IDs, and non-functional requirements use `NFR-001`-style IDs. Both use priority (MUST, SHOULD, or MAY). Constraints use unique `C-001` IDs. Options includes candidate descriptions and one comparison table with criteria in rows and options in columns, including Do nothing. Requirement cells use `Does not meet`, `Partially meets`, or `Fully meets`, followed by ` - ` and evidence. Constraint cells use `PASS` or `FAIL`, followed by ` - ` and evidence. The chosen option must fully meet each MUST requirement and pass every constraint. Explain the choice using criterion IDs rather than an aggregate score.

New records start as drafts. Draft RFCs require Context, Requirements and constraints, and Options, including at least one functional requirement and a comparison table. Draft comparison cells and the chosen option may remain blank, and Design and Adoption may be omitted. Before promotion to proposed, complete the five sections, assess every criterion for every option, select an eligible option, and replace template placeholders. Proposed records may return to draft for rework or proceed to accepted or rejected. Drafts may also be rejected. Accepted records may become deprecated or superseded.

Design describes only the chosen option, including behavior, examples, and failure cases. Add a Risks and mitigations subsection when material risks need explanation. Adoption covers rollout, compatibility, and verification, with migration or rollback when relevant.

### Risks and mitigations

Comparison labels require supporting evidence; the CLI checks structure and eligibility but cannot judge whether an assessment is true. Reviewers must examine the evidence before accepting a proposal.

## Adoption

Initialize a repository with the CLI, which creates the accepted format RFC and adoption ADR. Create drafts from the bundled template and run `makespec check` after edits. Complete a draft before promoting it to proposed, then request acceptance after review. Verify adoption by checking the seed records and exercising the draft-to-proposed-to-accepted workflow."#
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRAFT_RFC_BODY: &str = "## Context\n\nLet users search local records.\n\n## Requirements and constraints\n\n### Requirements\n\n| ID | Priority | Requirement |\n| --- | --- | --- |\n| FR-001 | MUST | Search records by title. |\n\n## Options\n\n| Criterion ID | Do nothing | Local search |\n| --- | --- | --- |\n| FR-001 | | |\n\n**Chosen option:**";

    #[test]
    fn rfc_drafts_can_be_unfinished_and_need_only_functional_criteria() {
        let draft = rfc(DRAFT_RFC_BODY, Status::Draft);

        assert!(validate_document(&draft).is_empty());
        assert!(validate_document(&rfc(DRAFT_RFC_BODY, Status::Rejected)).is_empty());

        let errors = validate_document(&rfc(DRAFT_RFC_BODY, Status::Proposed));

        for expected in [
            "`## Design`",
            "`## Adoption`",
            "chosen option",
            "label and explanation",
        ] {
            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "{errors:?}"
            );
        }

        let body = DRAFT_RFC_BODY
            .replace(
                "| FR-001 | | |",
                "| FR-001 | Does not meet - No search exists. | Fully meets - Search matches titles. |",
            )
            .replace("**Chosen option:**", "**Chosen option:** Local search")
            + "\n\n## Design\n\nMatch a query against record titles.\n\n## Adoption\n\nShip the search command and verify title matches.";
        let errors = validate_document(&rfc(&body, Status::Proposed));

        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn rfc_drafts_still_reject_inconsistent_comparison_structure() {
        for (original, replacement, expected) in [
            (
                "Do nothing",
                "Keep current behavior",
                "include a `Do nothing`",
            ),
            (
                "| FR-001 | MUST | Search records by title. |",
                "",
                "at least one functional requirement",
            ),
            ("| FR-001 | | |", "| FR-002 | | |", "unknown criterion"),
            (
                "| FR-001 | | |",
                "| FR-001 | | |\n| FR-001 | | |",
                "repeats criterion",
            ),
            (
                "| FR-001 | | |",
                "| FR-001 | PASS - Confused labels. | |",
                "invalid requirement label",
            ),
            (
                "**Chosen option:**",
                "**Chosen option:** Missing",
                "chosen option",
            ),
            (
                "### Requirements",
                "### Requirements\n\n### Requirements",
                "duplicate `### Requirements`",
            ),
            (
                "## Options",
                "## Open questions\n\nPending.\n\n## Options",
                "template order",
            ),
        ] {
            let body = DRAFT_RFC_BODY.replace(original, replacement);
            let errors = validate_document(&rfc(&body, Status::Draft));

            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn template_drafts_are_valid_but_require_completion_before_review() {
        for (kind, body) in [
            (
                Kind::Adr,
                include_str!("../../.agents/skills/makespec/references/adr-template.md"),
            ),
            (
                Kind::Rfc,
                include_str!("../../.agents/skills/makespec/references/rfc-template.md"),
            ),
        ] {
            let new_document = NewDocument {
                kind,
                title: "Template record".into(),
                slug: "template-record".into(),
                date: "2026-11-09".into(),
                body: body.into(),
            };
            let errors = validate_document(&new_document.build(1, Status::Draft).unwrap());

            assert!(errors.is_empty(), "{errors:?}");
            assert!(
                validate_document(&new_document.build(1, Status::Proposed).unwrap())
                    .iter()
                    .any(|error| error.contains("unresolved template placeholder"))
            );
        }
    }

    #[test]
    fn rfc_comparisons_validate_descriptions_labels_and_chosen_option_eligibility() {
        assert!(validate_document(&rfc(rfc_seed_body(), Status::Proposed)).is_empty());

        for (original, replacement, expected, valid_draft) in [
            (
                "Fully meets - The format requires functionality criteria, constraints, and comparisons.",
                "Does not meet - The format requires functionality criteria, constraints, and comparisons.",
                "chosen option does not fully meet a MUST requirement",
                true,
            ),
            (
                "PASS - The CLI reads and writes local Markdown files.",
                "FAIL - The CLI reads and writes local Markdown files.",
                "chosen option fails a constraint",
                true,
            ),
            (
                "Fully meets - The format requires functionality criteria, constraints, and comparisons.",
                "3 - The format requires functionality criteria, constraints, and comparisons.",
                "invalid requirement label `3` for `FR-001`",
                false,
            ),
            (
                "| FR-001 | MUST | An RFC specifies functionality requirements, constraints, and qualitative alternatives. |",
                "| FR-001 | MUST | |",
                "needs a description",
                false,
            ),
        ] {
            let body = rfc_seed_body().replace(original, replacement);
            let errors = validate_document(&rfc(&body, Status::Proposed));

            assert!(
                errors.iter().any(|error| error.contains(expected)),
                "{errors:?}"
            );
            assert_eq!(
                validate_document(&rfc(&body, Status::Draft)).is_empty(),
                valid_draft
            );
        }
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
        let errors = validate_document(&rfc(&body, Status::Proposed));

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

    fn rfc(body: &str, status: Status) -> Document {
        NewDocument {
            kind: Kind::Rfc,
            title: "Local search".into(),
            slug: "local-search".into(),
            date: "2026-11-09".into(),
            body: body.into(),
        }
        .build(1, status)
        .unwrap()
    }
}
