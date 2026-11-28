use super::{
    document::{
        Document, NewDocument, Status, slugify, validate_document_structure, validation_error,
    },
    error::{MakespecError, Result},
    ports::{
        driven::for_storing_documents::ForStoringDocuments,
        driver::for_managing_documents::CreateDocumentInput,
    },
};

pub(super) fn create_document(
    store: &impl ForStoringDocuments,
    CreateDocumentInput {
        kind,
        title,
        body,
        date,
    }: CreateDocumentInput,
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
        date,
        body,
    };
    let errors = validate_document_structure(&new_document.build(1, Status::Proposed)?);

    if !errors.is_empty() {
        return Err(validation_error("document structure is invalid", &errors));
    }

    store.create_proposal(new_document)
}

#[cfg(test)]
mod tests {
    use crate::{
        hexagon::{
            document::Kind,
            ports::driver::for_managing_documents::{CreateDocumentInput, ForManagingDocuments},
        },
        startup::test::{DECISION_BODY, makespec},
    };

    fn input(kind: Kind, title: &str, body: &str) -> CreateDocumentInput {
        CreateDocumentInput {
            kind,
            title: title.into(),
            body: body.into(),
            date: "2026-11-09".into(),
        }
    }

    #[test]
    fn creation_allocates_the_next_id_of_each_kind() {
        let (app, _) = makespec();

        app.initialize_project("2026-11-09").unwrap();

        let first = app
            .create_document(input(Kind::Decision, "Second decision", DECISION_BODY))
            .unwrap();
        let second = app
            .create_document(input(Kind::Decision, "Third decision", DECISION_BODY))
            .unwrap();

        assert_eq!(first.metadata.id, "ADR-0002");
        assert_eq!(first.path, "docs/decisions/0002-second-decision.md");
        assert_eq!(second.metadata.id, "ADR-0003");
    }

    #[test]
    fn creation_rejects_front_matter_and_invalid_structure_without_writing() {
        let (app, documents) = makespec();

        app.initialize_project("2026-11-09").unwrap();

        for (body, expected) in [
            (
                format!("---\nid: ADR-0009\n---\n{DECISION_BODY}"),
                "omit YAML",
            ),
            (
                DECISION_BODY.replace("## Decision", "## Choice"),
                "missing required `## Decision`",
            ),
        ] {
            let error = app
                .create_document(input(Kind::Decision, "Rejected", &body))
                .unwrap_err();

            assert!(error.to_string().contains(expected), "{error}");
        }

        assert_eq!(documents.read_file("docs/decisions/0002-rejected.md"), None);
    }

    #[test]
    fn creation_rejects_titles_without_a_slug() {
        let (app, _) = makespec();

        app.initialize_project("2026-11-09").unwrap();

        assert!(
            app.create_document(input(Kind::Decision, "../escape", DECISION_BODY))
                .is_err()
        );
    }
}
