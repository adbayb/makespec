use std::collections::{BTreeMap, BTreeSet};

use super::{
    document::{Document, Status, slugify, validate_document},
    error::Result,
    ports::driven::for_storing_documents::ForStoringDocuments,
};

pub(super) fn check_documents(store: &impl ForStoringDocuments) -> Result<Vec<String>> {
    let documents = store.get_documents()?;
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

#[cfg(test)]
mod tests {
    use crate::{
        hexagon::{
            error::MakespecError, ports::driver::for_managing_documents::ForManagingDocuments,
        },
        startup::test::{decision_file, makespec},
    };

    fn assert_reports(errors: &[String], expected: &str) {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "expected {expected:?}, got {errors:?}"
        );
    }

    #[test]
    fn checking_requires_an_initialized_project() {
        let (app, _) = makespec();

        assert!(matches!(
            app.check_documents(),
            Err(MakespecError::NotInitialized)
        ));
    }

    #[test]
    fn checking_reports_filenames_and_duplicate_ids() {
        let (app, documents) = makespec();

        app.initialize_project("2026-11-09").unwrap();
        documents.write_file(
            "docs/decisions/0002-Second.md",
            &decision_file("ADR-0002", "Second", "proposed", ""),
        );
        documents.write_file(
            "docs/decisions/0002-second-copy.md",
            &decision_file("ADR-0002", "Second", "proposed", ""),
        );

        let errors = app.check_documents().unwrap();

        assert_reports(&errors, "0002-Second.md: filename must start with");
        assert_reports(&errors, "duplicate decision identifier number 0002");
    }

    #[test]
    fn checking_reports_missing_and_cyclic_supersession_targets() {
        let (app, documents) = makespec();

        app.initialize_project("2026-11-09").unwrap();
        documents.write_file(
            "docs/decisions/0002-second.md",
            &decision_file(
                "ADR-0002",
                "Second",
                "superseded",
                "superseded_by: ADR-0003\n",
            ),
        );
        documents.write_file(
            "docs/decisions/0003-third.md",
            &decision_file(
                "ADR-0003",
                "Third",
                "superseded",
                "superseded_by: ADR-0002\n",
            ),
        );
        documents.write_file(
            "docs/decisions/0004-fourth.md",
            &decision_file(
                "ADR-0004",
                "Fourth",
                "superseded",
                "superseded_by: ADR-0009\n",
            ),
        );

        let errors = app.check_documents().unwrap();

        assert_reports(
            &errors,
            "0002-second.md: supersession chain contains a cycle",
        );
        assert_reports(&errors, "supersession target `ADR-0009` does not exist");
    }
}
