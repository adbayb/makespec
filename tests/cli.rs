use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

const ADR_BODY: &str = "## Context\n\nThe project needs a focused decision.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nThe decision remains in Git.";

fn makespec() -> Command {
    Command::new(env!("CARGO_BIN_EXE_makespec"))
}

fn create(project: &Path, kind: &str, title: &str, body: &str) -> Output {
    let mut command = makespec()
        .current_dir(project)
        .args(["new", kind, "--title", title])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    command
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();

    command.wait_with_output().unwrap()
}

#[test]
fn cli_initializes_creates_lists_changes_status_and_checks() {
    let project = tempdir().unwrap();
    let initialized = makespec().arg("init").arg(project.path()).output().unwrap();

    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    assert!(
        project
            .path()
            .join(".agents/skills/makespec/SKILL.md")
            .is_file()
    );

    let seed = std::fs::read_to_string(
        project
            .path()
            .join("docs/adrs/0001-use-adrs-for-decisions-and-rfcs-for-changes.md"),
    )
    .unwrap();

    for field in ["kind:", "slug:", "schema_version:"] {
        assert!(!seed.contains(field));
    }

    let created = create(project.path(), "adr", "Document a focused choice", ADR_BODY);

    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    assert!(String::from_utf8_lossy(&created.stdout).contains("ADR-0002"));

    let created_document = std::fs::read_to_string(
        project
            .path()
            .join("docs/adrs/0002-document-a-focused-choice.md"),
    )
    .unwrap();

    for field in ["kind:", "slug:", "schema_version:"] {
        assert!(!created_document.contains(field));
    }

    let listed = makespec()
        .current_dir(project.path())
        .args(["list", "adr", "--status", "draft"])
        .output()
        .unwrap();

    assert!(listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stdout).contains("Document a focused choice"));

    let proposed = makespec()
        .current_dir(project.path())
        .args(["status", "ADR-0002", "proposed"])
        .output()
        .unwrap();

    assert!(proposed.status.success());

    let accepted = makespec()
        .current_dir(project.path())
        .args(["status", "ADR-0002", "accepted"])
        .output()
        .unwrap();

    assert!(accepted.status.success());

    let checked = makespec()
        .current_dir(project.path())
        .arg("check")
        .output()
        .unwrap();

    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn no_skills_skips_agent_skill_installation() {
    let project = tempdir().unwrap();
    let output = makespec()
        .args(["init", "--no-skills"])
        .arg(project.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!project.path().join(".agents/skills/makespec").exists());
    assert!(project.path().join("docs/adrs").is_dir());
    assert!(project.path().join("docs/rfcs").is_dir());
}

#[test]
fn invalid_document_body_is_rejected_without_writing_a_record() {
    let project = tempdir().unwrap();
    let initialized = makespec()
        .args(["init", "--no-skills"])
        .arg(project.path())
        .output()
        .unwrap();

    assert!(initialized.status.success());

    let result = create(
        project.path(),
        "adr",
        "Incomplete decision",
        "## Context\n\nOnly one section.",
    );

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("missing required `## Decision` section")
    );
    assert_eq!(
        std::fs::read_dir(project.path().join("docs/adrs"))
            .unwrap()
            .count(),
        1
    );

    let result = create(
        project.path(),
        "adr",
        "Body only",
        "---\nid: ADR-0001\n---\n\n## Context\n\nIssue.",
    );

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("omit YAML front matter"));
}

#[test]
fn rfc_drafts_require_completion_before_review_and_acceptance() {
    let project = tempdir().unwrap();
    let initialized = makespec().arg("init").arg(project.path()).output().unwrap();

    assert!(initialized.status.success());

    let body = "## Context\n\nSearch local records.\n\n## Requirements and constraints\n\n### Requirements\n\n| ID | Priority | Requirement |\n| --- | --- | --- |\n| FR-001 | MUST | Find records by title. |\n\n## Options\n\n| Criterion ID | Do nothing | Search |\n| --- | --- | --- |\n| FR-001 | | |\n\n**Chosen option:**";
    let created = create(project.path(), "rfc", "Local search", body);

    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );

    let path = project.path().join("docs/rfcs/0002-local-search.md");
    let draft = std::fs::read_to_string(&path).unwrap();

    assert!(draft.contains("status: draft\n"));
    assert!(
        makespec()
            .current_dir(project.path())
            .arg("check")
            .output()
            .unwrap()
            .status
            .success()
    );

    for next in ["accepted", "proposed"] {
        let result = makespec()
            .current_dir(project.path())
            .args(["status", "RFC-0002", next])
            .output()
            .unwrap();

        assert!(!result.status.success());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), draft);
    }

    let completed = draft
        .replace(
            "| FR-001 | | |",
            "| FR-001 | Does not meet - No search exists. | Fully meets - Query matches titles. |",
        )
        .replace("**Chosen option:**", "**Chosen option:** Search")
        + "\n\n## Design\n\nMatch queries against record titles.\n\n## Adoption\n\nShip the command and verify search results.";

    std::fs::write(&path, &completed).unwrap();

    for next in ["proposed", "draft", "proposed", "accepted"] {
        let result = makespec()
            .current_dir(project.path())
            .args(["status", "RFC-0002", next])
            .output()
            .unwrap();

        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let rejected_draft = create(project.path(), "rfc", "Another search", body);

    assert!(rejected_draft.status.success());

    let rejected = makespec()
        .current_dir(project.path())
        .args(["status", "RFC-0003", "rejected"])
        .output()
        .unwrap();

    assert!(rejected.status.success());

    let corrupted = std::fs::read_to_string(&path)
        .unwrap()
        .replace("Fully meets - Query matches titles.", "");

    std::fs::write(&path, corrupted).unwrap();

    let checked = makespec()
        .current_dir(project.path())
        .arg("check")
        .output()
        .unwrap();

    assert!(!checked.status.success());

    std::fs::write(
        &path,
        completed.replace("status: draft", "status: accepted"),
    )
    .unwrap();

    let checked = makespec()
        .current_dir(project.path())
        .arg("check")
        .output()
        .unwrap();

    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
