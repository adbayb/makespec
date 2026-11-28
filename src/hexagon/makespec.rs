use super::{
    change_document_status::change_document_status,
    check_documents::check_documents,
    create_document::create_document,
    document::{Document, DocumentSummary},
    error::Result,
    get_summaries::get_summaries,
    initialize_project::initialize_project,
    ports::{
        driven::{
            for_obtaining_skill_files::ForObtainingSkillFiles,
            for_storing_documents::ForStoringDocuments,
        },
        driver::for_managing_documents::{
            ChangeDocumentStatusInput, CreateDocumentInput, ForManagingDocuments,
        },
    },
};

/// The hexagon: implements the driver port on top of injected driven ports.
pub(crate) struct Makespec<Store, Skill> {
    store: Store,
    skill: Skill,
}

impl<Store, Skill> Makespec<Store, Skill>
where
    Store: ForStoringDocuments,
    Skill: ForObtainingSkillFiles,
{
    pub(crate) fn new(store: Store, skill: Skill) -> Self {
        Self { store, skill }
    }
}

impl<Store, Skill> ForManagingDocuments for Makespec<Store, Skill>
where
    Store: ForStoringDocuments,
    Skill: ForObtainingSkillFiles,
{
    fn initialize_project(&self, date: &str) -> Result<()> {
        initialize_project(&self.store, &self.skill, date)
    }

    fn create_document(&self, input: CreateDocumentInput) -> Result<Document> {
        create_document(&self.store, input)
    }

    fn get_summaries(&self) -> Result<Vec<DocumentSummary>> {
        get_summaries(&self.store)
    }

    fn change_document_status(&self, input: ChangeDocumentStatusInput) -> Result<Document> {
        change_document_status(&self.store, input)
    }

    fn check_documents(&self) -> Result<Vec<String>> {
        check_documents(&self.store)
    }
}
