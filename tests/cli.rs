use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::{TempDir, tempdir};

const ADR_BODY: &str = "## Context\n\nThe project needs a focused decision.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nThe decision remains in Git.";

#[test]
fn cli_initializes_creates_lists_changes_status_and_checks() {
    let project = initialized_project();

    assert!(
        project
            .path()
            .join(".agents/skills/makespec/SKILL.md")
            .is_file()
    );

    let created = create(project.path(), "adr", "Document a focused choice", ADR_BODY);

    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    assert!(String::from_utf8_lossy(&created.stdout).contains("ADR-0002"));

    let listed = run(project.path(), &["list", "adr", "--status", "draft"]);

    assert!(listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stdout).contains("Document a focused choice"));

    let proposed = run(project.path(), &["status", "ADR-0002", "proposed"]);

    assert!(proposed.status.success());

    let accepted = run(project.path(), &["status", "ADR-0002", "accepted"]);

    assert!(accepted.status.success());

    let rejected = run(project.path(), &["status", "ADR-0002", "rejected"]);

    assert!(!rejected.status.success());

    let checked = run(project.path(), &["check"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn no_skills_skips_agent_skill_installation() {
    let project = tempdir().unwrap();
    let output = run(project.path(), &["init", "--no-skills"]);

    assert!(output.status.success());
    assert!(!project.path().join(".agents/skills/makespec").exists());
    assert!(project.path().join("docs/adrs").is_dir());
    assert!(project.path().join("docs/rfcs").is_dir());
}

#[test]
fn invalid_document_body_is_rejected_without_writing_a_record() {
    let project = initialized_project();

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
    let project = initialized_project();

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
    assert!(run(project.path(), &["check"]).status.success());

    for next in ["accepted", "proposed"] {
        let result = run(project.path(), &["status", "RFC-0002", next]);

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
        let result = run(project.path(), &["status", "RFC-0002", next]);

        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let rejected_draft = create(project.path(), "rfc", "Another search", body);

    assert!(rejected_draft.status.success());

    let rejected = run(project.path(), &["status", "RFC-0003", "rejected"]);

    assert!(rejected.status.success());

    let corrupted = std::fs::read_to_string(&path)
        .unwrap()
        .replace("Fully meets - Query matches titles.", "");

    std::fs::write(&path, corrupted).unwrap();

    let checked = run(project.path(), &["check"]);

    assert!(!checked.status.success());

    std::fs::write(
        &path,
        completed.replace("status: draft", "status: accepted"),
    )
    .unwrap();

    let checked = run(project.path(), &["check"]);

    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn terminal_output_escapes_controls_in_titles_and_errors() {
    let project = initialized_project();

    std::fs::write(
        project.path().join("docs/adrs/0002-untrusted-title.md"),
        "---\nid: ADR-0002\ntitle: \"Unsafe\\u001b[2J title\"\ndate: 2026-11-09\nstatus: draft\n---\n",
    ).unwrap();

    let listed = run(project.path(), &["list", "adr"]);
    let missing = run(project.path(), &["status", "ADR-\u{1b}[2J", "accepted"]);

    assert!(listed.status.success());
    assert!(!missing.status.success());

    for output in [&listed.stdout, &missing.stderr] {
        let text = String::from_utf8_lossy(output);

        assert!(text.contains('\u{fffd}'));
        assert!(!text.contains('\u{1b}'));
    }
}

fn initialized_project() -> TempDir {
    let project = tempdir().unwrap();
    let initialized = makespec().arg("init").arg(project.path()).output().unwrap();

    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );

    project
}

fn makespec() -> Command {
    Command::new(env!("CARGO_BIN_EXE_makespec"))
}

fn run(project: &Path, arguments: &[&str]) -> Output {
    makespec()
        .current_dir(project)
        .args(arguments)
        .output()
        .unwrap()
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
