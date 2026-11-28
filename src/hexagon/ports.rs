use super::{
    document::{Document, DocumentSummary, NewDocument},
    error::Result,
};

#[derive(Clone, Debug)]
pub(crate) struct ProjectFile {
    pub relative_path: String,
    pub content: String,
}

/// Driven port for reading and publishing project records.
pub(crate) trait ProjectStore {
    /// Publishes missing files without replacing different existing content.
    fn initialize(&self, files: &[ProjectFile]) -> Result<()>;
    /// Loads records from the document directories that already exist.
    fn existing_documents(&self) -> Result<Vec<Document>>;
    /// Loads every record, failing when the project is not initialized.
    fn documents(&self) -> Result<Vec<Document>>;
    /// Loads record metadata without reading complete bodies.
    fn catalog(&self) -> Result<Vec<DocumentSummary>>;
    fn document(&self, id: &str) -> Result<Document>;
    /// Allocates the next ID for the document kind and publishes the proposal.
    fn create_proposal(&self, new_document: NewDocument) -> Result<Document>;
    /// Replaces a record, failing when it changed since `previous` was read.
    fn replace(&self, previous: &Document, replacement: &Document) -> Result<()>;
}
