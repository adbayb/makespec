use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::{TempDir, tempdir};

const DECISION_BODY: &str = "## Context\n\nThe project needs a focused decision.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nThe decision remains in Git.";
const SPECIFICATION_BODY: &str = "## Context\n\nCompare approaches to record management.\n\n## Requirements and constraints\n\n### Requirements\n\n| ID | Priority | Requirement |\n| --- | --- | --- |\n| FR-001 | MUST | Compare record management approaches. |\n\n## Options\n\n| Criterion ID | Markdown templates | Makespec CLI |\n| --- | --- | --- |\n| FR-001 | | |\n\n**Chosen option:**";

#[test]
fn list_reports_an_empty_catalog() {
    let project = tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("docs/decisions")).unwrap();
    std::fs::create_dir_all(project.path().join("docs/specifications")).unwrap();
    let listed = run(project.path(), &["list"]);

    assert!(listed.status.success());
    assert_eq!(
        String::from_utf8_lossy(&listed.stdout),
        "No records found.\n"
    );
}

#[test]
fn version_is_available_with_lowercase_short_flag() {
    let project = tempdir().unwrap();
    let version = run(project.path(), &["-v"]);
    let long_version = run(project.path(), &["--version"]);

    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));
    assert!(long_version.status.success());
    assert_eq!(run(project.path(), &["-V"]).status.code(), Some(2));
}

#[test]
fn cli_initializes_creates_lists_changes_status_and_checks() {
    let project = initialized_project();

    assert_eq!(
        std::fs::read_dir(project.path().join("docs/specifications"))
            .unwrap()
            .count(),
        1
    );
    assert!(
        project
            .path()
            .join("docs/specifications/.gitkeep")
            .is_file()
    );
    assert!(
        project
            .path()
            .join(".agents/skills/makespec/SKILL.md")
            .is_file()
    );
    for (name, expected) in [
        (
            "decision-template.md",
            include_str!("../.agents/skills/makespec/references/decision-template.md"),
        ),
        (
            "specification-template.md",
            include_str!("../.agents/skills/makespec/references/specification-template.md"),
        ),
        (
            "writing.md",
            include_str!("../.agents/skills/makespec/references/writing.md"),
        ),
    ] {
        assert_eq!(
            std::fs::read_to_string(
                project
                    .path()
                    .join(".agents/skills/makespec/references")
                    .join(name)
            )
            .unwrap(),
            expected
        );
    }

    let created = create(
        project.path(),
        "decision",
        "Document a focused choice",
        DECISION_BODY,
    );

    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    assert!(String::from_utf8_lossy(&created.stdout).contains("ADR-0002"));

    let created_specification = create(
        project.path(),
        "specification",
        "Compare record management approaches",
        SPECIFICATION_BODY,
    );

    assert!(
        created_specification.status.success(),
        "{}",
        String::from_utf8_lossy(&created_specification.stderr)
    );

    let listed = run(project.path(), &["list"]);
    let listed_text = String::from_utf8_lossy(&listed.stdout);

    assert!(listed.status.success());
    assert!(listed_text.contains("Decisions (2)"));
    assert!(listed_text.contains("Specifications (1)"));
    assert!(listed_text.contains("ADR-0002  Document a focused choice"));
    assert!(listed_text.contains("Status: draft  Date: "));
    assert!(listed_text.contains("Path: docs/decisions/0002-document-a-focused-choice.md"));
    assert!(listed_text.contains("RFC-0001  Compare record management approaches"));
    assert!(!listed_text.contains('\t'));

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
fn creation_rejects_abbreviated_document_kinds() {
    let project = initialized_project();

    for kind in ["adr", "rfc"] {
        let result = run(project.path(), &["new", kind, "A focused choice"]);

        assert_eq!(result.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("expected `decision` or `specification`")
        );
    }
}

#[test]
fn supersession_requires_a_positional_accepted_replacement() {
    let project = initialized_project();
    let created = create(
        project.path(),
        "decision",
        "Use a replacement",
        DECISION_BODY,
    );

    assert!(created.status.success());

    let missing = run(project.path(), &["status", "ADR-0001", "superseded"]);
    let premature = run(
        project.path(),
        &["status", "ADR-0001", "superseded", "ADR-0002"],
    );
    let unexpected = run(
        project.path(),
        &["status", "ADR-0001", "deprecated", "ADR-0002"],
    );

    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("requires a replacement ID"));
    assert!(!premature.status.success());
    assert!(String::from_utf8_lossy(&premature.stderr).contains("must be accepted first"));
    assert!(!unexpected.status.success());
    assert!(String::from_utf8_lossy(&unexpected.stderr).contains("only valid"));

    for next in ["proposed", "accepted"] {
        assert!(
            run(project.path(), &["status", "ADR-0002", next])
                .status
                .success()
        );
    }

    let superseded = run(
        project.path(),
        &["status", "ADR-0001", "superseded", "ADR-0002"],
    );

    assert!(
        superseded.status.success(),
        "{}",
        String::from_utf8_lossy(&superseded.stderr)
    );
    assert!(run(project.path(), &["check"]).status.success());

    let original = std::fs::read_to_string(
        project
            .path()
            .join("docs/decisions/0001-use-makespec-to-manage-decisions-and-specifications.md"),
    )
    .unwrap();

    assert!(original.contains("status: superseded\n"));
    assert!(original.contains("superseded_by: ADR-0002\n"));
}

#[test]
fn invalid_document_body_is_rejected_without_writing_a_record() {
    let project = initialized_project();

    let result = create(
        project.path(),
        "decision",
        "Incomplete decision",
        "## Context\n\nOnly one section.",
    );

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("missing required `## Decision` section")
    );
    assert_eq!(
        std::fs::read_dir(project.path().join("docs/decisions"))
            .unwrap()
            .count(),
        1
    );

    let result = create(
        project.path(),
        "decision",
        "Body only",
        "---\nid: ADR-0001\n---\n\n## Context\n\nIssue.",
    );

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("omit YAML front matter"));
}

#[test]
fn specification_drafts_require_completion_before_review_and_acceptance() {
    let project = initialized_project();

    let body = "## Context\n\nSearch local records.\n\n## Requirements and constraints\n\n### Requirements\n\n| ID | Priority | Requirement |\n| --- | --- | --- |\n| FR-001 | MUST | Find records by title. |\n\n## Options\n\n| Criterion ID | No change | Search |\n| --- | --- | --- |\n| FR-001 | | |\n\n**Chosen option:**";
    let created = create(project.path(), "specification", "Local search", body);

    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );

    assert!(String::from_utf8_lossy(&created.stdout).contains("RFC-0001"));

    let path = project
        .path()
        .join("docs/specifications/0001-local-search.md");
    let draft = std::fs::read_to_string(&path).unwrap();

    assert!(draft.contains("status: draft\n"));
    assert!(run(project.path(), &["check"]).status.success());

    for next in ["accepted", "proposed"] {
        let result = run(project.path(), &["status", "RFC-0001", next]);

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
        let result = run(project.path(), &["status", "RFC-0001", next]);

        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let rejected_draft = create(project.path(), "specification", "Another search", body);

    assert!(rejected_draft.status.success());

    let rejected = run(project.path(), &["status", "RFC-0002", "rejected"]);

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
        project.path().join("docs/decisions/0002-untrusted-title.md"),
        "---\nid: ADR-0002\ntitle: \"Unsafe\\u001b[2J title\"\ndate: 2026-11-09\nstatus: draft\n---\n",
    ).unwrap();

    let listed = run(project.path(), &["list"]);
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
    let initialized = run(project.path(), &["init"]);

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
        .args(["new", kind, title])
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
