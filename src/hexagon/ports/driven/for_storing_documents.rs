use crate::hexagon::{
    document::{Document, DocumentSummary, NewDocument},
    error::Result,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectFile {
    pub relative_path: String,
    pub content: String,
}

/// Driven port for reading and publishing project documents.
pub(crate) trait ForStoringDocuments {
    /// Publishes missing files without replacing different existing content.
    fn create_files(&self, files: &[ProjectFile]) -> Result<()>;
    /// Gets documents from the document directories that already exist.
    fn get_existing_documents(&self) -> Result<Vec<Document>>;
    /// Gets every document, failing when the project is not initialized.
    fn get_documents(&self) -> Result<Vec<Document>>;
    /// Gets document metadata without reading complete bodies.
    fn get_summaries(&self) -> Result<Vec<DocumentSummary>>;
    fn get_document(&self, id: &str) -> Result<Document>;
    /// Allocates the next ID for the document kind and publishes the proposal.
    fn create_proposal(&self, new_document: NewDocument) -> Result<Document>;
    /// Replaces a document, failing when it changed since `previous` was read.
    fn update_document(&self, previous: &Document, replacement: &Document) -> Result<()>;
}
