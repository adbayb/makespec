use crate::hexagon::{
    document::{Document, DocumentSummary, Kind, Status},
    error::Result,
};

pub(crate) struct CreateDocumentInput {
    pub kind: Kind,
    pub title: String,
    pub body: String,
    /// Creation date in `YYYY-MM-DD` form.
    pub date: String,
}

pub(crate) struct ChangeDocumentStatusInput<'a> {
    pub id: &'a str,
    pub status: Status,
    /// Accepted document that replaces a superseded one.
    pub replacement_id: Option<&'a str>,
}

/// Driver port for the Makespec document workflows.
pub(crate) trait ForManagingDocuments {
    /// Installs the adoption decision, agent skill, and document directories.
    fn initialize_project(&self, date: &str) -> Result<()>;
    /// Creates a proposed document with the next ID of its kind.
    fn create_document(&self, input: CreateDocumentInput) -> Result<Document>;
    fn get_summaries(&self) -> Result<Vec<DocumentSummary>>;
    fn change_document_status(&self, input: ChangeDocumentStatusInput) -> Result<Document>;
    /// Returns every validation error across the project's documents.
    fn check_documents(&self) -> Result<Vec<String>>;
}
