use super::{
    document::{Document, Status, ensure_valid, parse_id, update_metadata},
    error::{MakespecError, Result},
    ports::{
        driven::for_storing_documents::ForStoringDocuments,
        driver::for_managing_documents::ChangeDocumentStatusInput,
    },
};

pub(super) fn change_document_status(
    store: &impl ForStoringDocuments,
    ChangeDocumentStatusInput {
        id,
        status: next_status,
        replacement_id: superseded_by,
    }: ChangeDocumentStatusInput,
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

    let current = store.get_document(id)?;

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

    store.update_document(&current, &replacement)?;

    Ok(replacement)
}

fn accepted_replacement(
    store: &impl ForStoringDocuments,
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
        .get_summaries()?
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

#[cfg(test)]
mod tests {
    use crate::{
        adapters::fake::for_storing_documents::InMemoryDocuments,
        hexagon::{
            document::Status,
            ports::driver::for_managing_documents::{
                ChangeDocumentStatusInput, ForManagingDocuments,
            },
        },
        startup::test::{TestMakespec, decision_file, makespec},
    };

    fn change<'a>(
        id: &'a str,
        status: Status,
        replacement_id: Option<&'a str>,
    ) -> ChangeDocumentStatusInput<'a> {
        ChangeDocumentStatusInput {
            id,
            status,
            replacement_id,
        }
    }

    fn initialized_with_proposal() -> (TestMakespec, InMemoryDocuments) {
        let (app, documents) = makespec();

        app.initialize_project("2026-11-09").unwrap();
        documents.write_file(
            "docs/decisions/0002-replacement.md",
            &decision_file("ADR-0002", "Replacement", "proposed", ""),
        );

        (app, documents)
    }

    #[test]
    fn replacement_ids_are_required_only_for_supersession() {
        let (app, _) = initialized_with_proposal();

        for (input, expected) in [
            (
                change("ADR-0001", Status::Superseded, None),
                "requires a replacement ID",
            ),
            (
                change("ADR-0002", Status::Accepted, Some("ADR-0001")),
                "only valid when setting status to `superseded`",
            ),
        ] {
            let error = app.change_document_status(input).unwrap_err();

            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn superseding_requires_an_accepted_different_same_kind_replacement() {
        let (app, _) = initialized_with_proposal();

        for (replacement, expected) in [
            ("ADR-0002", "must be accepted first"),
            ("ADR-0001", "must be a different decision document"),
            ("RFC-0001", "must be a different decision document"),
            ("ADR-0009", "document not found"),
        ] {
            let error = app
                .change_document_status(change("ADR-0001", Status::Superseded, Some(replacement)))
                .unwrap_err();

            assert!(error.to_string().contains(expected), "{error}");
        }

        app.change_document_status(change("ADR-0002", Status::Accepted, None))
            .unwrap();

        let superseded = app
            .change_document_status(change("ADR-0001", Status::Superseded, Some("ADR-0002")))
            .unwrap();

        assert_eq!(
            superseded.metadata.superseded_by.as_deref(),
            Some("ADR-0002")
        );
        assert!(app.check_documents().unwrap().is_empty());
    }

    #[test]
    fn transitions_follow_the_lifecycle_and_repeat_as_no_ops() {
        let (app, documents) = initialized_with_proposal();
        let before = documents
            .read_file("docs/decisions/0002-replacement.md")
            .unwrap();

        app.change_document_status(change("ADR-0002", Status::Proposed, None))
            .unwrap();

        assert_eq!(
            documents.read_file("docs/decisions/0002-replacement.md"),
            Some(before)
        );

        app.change_document_status(change("ADR-0002", Status::Rejected, None))
            .unwrap();

        let error = app
            .change_document_status(change("ADR-0002", Status::Accepted, None))
            .unwrap_err();

        assert!(error.to_string().contains("cannot change"), "{error}");
    }

    #[test]
    fn incomplete_proposals_cannot_be_accepted() {
        let (app, documents) = initialized_with_proposal();

        documents.write_file(
            "docs/decisions/0003-incomplete.md",
            &decision_file("ADR-0003", "Incomplete", "proposed", "")
                .replace("Use the proposed approach.", "[TODO: decide]"),
        );

        let error = app
            .change_document_status(change("ADR-0003", Status::Accepted, None))
            .unwrap_err();

        assert!(error.to_string().contains("placeholder"), "{error}");
    }
}
