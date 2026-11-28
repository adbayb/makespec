use super::{
    document::DocumentSummary, error::Result,
    ports::driven::for_storing_documents::ForStoringDocuments,
};

pub(super) fn get_summaries(store: &impl ForStoringDocuments) -> Result<Vec<DocumentSummary>> {
    store.get_summaries()
}

#[cfg(test)]
mod tests {
    use crate::{
        hexagon::{
            error::MakespecError, ports::driver::for_managing_documents::ForManagingDocuments,
        },
        startup::test::{decision_file, makespec},
    };

    #[test]
    fn summaries_require_initialization_and_sort_by_kind_and_number() {
        let (app, documents) = makespec();

        assert!(matches!(
            app.get_summaries(),
            Err(MakespecError::NotInitialized)
        ));

        app.initialize_project("2026-11-09").unwrap();
        documents.write_file(
            "docs/decisions/0010-tenth.md",
            &decision_file("ADR-0010", "Tenth", "proposed", ""),
        );
        documents.write_file(
            "docs/decisions/0002-second.md",
            &decision_file("ADR-0002", "Second", "proposed", ""),
        );

        let ids: Vec<_> = app
            .get_summaries()
            .unwrap()
            .into_iter()
            .map(|summary| summary.metadata.id)
            .collect();

        assert_eq!(ids, ["ADR-0001", "ADR-0002", "ADR-0010"]);
    }
}
