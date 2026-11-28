use crate::{
    adapters::fake::{
        for_obtaining_skill_files::StubSkillFiles, for_storing_documents::InMemoryDocuments,
    },
    hexagon::Makespec,
};

pub(crate) type TestMakespec = Makespec<InMemoryDocuments, StubSkillFiles>;

/// Test entry: wires in-memory doubles into the hexagon so tests drive it directly.
/// Returns a handle on the shared in-memory files for arranging and asserting.
pub(crate) fn makespec() -> (
    Makespec<InMemoryDocuments, StubSkillFiles>,
    InMemoryDocuments,
) {
    let documents = InMemoryDocuments::default();

    (Makespec::new(documents.clone(), StubSkillFiles), documents)
}

pub(crate) const DECISION_BODY: &str = "## Context\n\nA focused decision is needed.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nThe choice is documented.";

/// Renders a decision file with the given front matter, as an editor could write it.
pub(crate) fn decision_file(id: &str, title: &str, status: &str, extra: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\ndate: 2026-11-09\nstatus: {status}\n{extra}---\n\n# {id}: {title}\n\n{DECISION_BODY}"
    )
}
