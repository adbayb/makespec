use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
};

use crate::hexagon::{
    document::{Document, DocumentSummary, NewDocument, Status, parse_document, parse_id},
    error::{MakespecError, Result},
    ports::driven::for_storing_documents::{ForStoringDocuments, ProjectFile},
};

/// Keeps project files in memory so use cases run without filesystem setup.
/// Clones share the same files, letting tests inspect what the hexagon wrote.
#[derive(Clone, Default)]
pub(crate) struct InMemoryDocuments {
    files: Rc<RefCell<BTreeMap<String, String>>>,
    is_initialized: Rc<Cell<bool>>,
}

impl InMemoryDocuments {
    /// Writes a file directly, as an external editor would.
    pub(crate) fn write_file(&self, relative_path: &str, content: &str) {
        self.files
            .borrow_mut()
            .insert(relative_path.into(), content.into());
    }

    pub(crate) fn read_file(&self, relative_path: &str) -> Option<String> {
        self.files.borrow().get(relative_path).cloned()
    }

    fn documents(&self, require_initialization: bool) -> Result<Vec<Document>> {
        if require_initialization && !self.is_initialized.get() {
            return Err(MakespecError::NotInitialized);
        }

        let mut documents = self
            .files
            .borrow()
            .iter()
            .filter(|(path, _)| path.starts_with("docs/") && path.ends_with(".md"))
            .map(|(path, content)| parse_document(content, path.clone()))
            .collect::<Result<Vec<_>>>()?;

        documents.sort_by_key(|document| parse_id(&document.metadata.id).ok());

        Ok(documents)
    }
}

impl ForStoringDocuments for InMemoryDocuments {
    fn create_files(&self, files: &[ProjectFile]) -> Result<()> {
        let mut stored = self.files.borrow_mut();

        if let Some(file) = files.iter().find(|file| {
            stored
                .get(&file.relative_path)
                .is_some_and(|content| *content != file.content)
        }) {
            return Err(MakespecError::Invalid(format!(
                "initialization conflict at {}",
                file.relative_path
            )));
        }

        for file in files {
            stored.insert(file.relative_path.clone(), file.content.clone());
        }

        self.is_initialized.set(true);

        Ok(())
    }

    fn get_existing_documents(&self) -> Result<Vec<Document>> {
        self.documents(false)
    }

    fn get_documents(&self) -> Result<Vec<Document>> {
        self.documents(true)
    }

    fn get_summaries(&self) -> Result<Vec<DocumentSummary>> {
        Ok(self
            .documents(true)?
            .into_iter()
            .map(|document| DocumentSummary {
                metadata: document.metadata,
                path: document.path,
            })
            .collect())
    }

    fn get_document(&self, id: &str) -> Result<Document> {
        self.documents(true)?
            .into_iter()
            .find(|document| document.metadata.id == id)
            .ok_or_else(|| MakespecError::NotFound(id.into()))
    }

    fn create_proposal(&self, new_document: NewDocument) -> Result<Document> {
        let number = self
            .documents(true)?
            .iter()
            .filter_map(|document| parse_id(&document.metadata.id).ok())
            .filter(|(kind, _)| *kind == new_document.kind)
            .map(|(_, number)| number)
            .max()
            .unwrap_or(0)
            + 1;
        let document = new_document.build(number, Status::Proposed)?;

        if self.read_file(&document.path).is_some() {
            return Err(MakespecError::AlreadyExists(PathBuf::from(&document.path)));
        }

        self.write_file(&document.path, &document.raw);

        Ok(document)
    }

    fn update_document(&self, previous: &Document, replacement: &Document) -> Result<()> {
        if self.read_file(&previous.path).as_deref() != Some(previous.raw.as_str()) {
            return Err(MakespecError::ConcurrentModification(PathBuf::from(
                &previous.path,
            )));
        }

        self.write_file(&replacement.path, &replacement.raw);

        Ok(())
    }
}
