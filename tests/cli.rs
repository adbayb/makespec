use std::{
    io::Write,
    process::{Command, Stdio},
};

use tempfile::tempdir;

const ADR_BODY: &str = "## Context\n\nThe project needs a focused decision.\n\n## Decision\n\nUse the proposed approach.\n\n## Consequences\n\nThe decision remains in Git.";

fn makespec() -> Command {
    Command::new(env!("CARGO_BIN_EXE_makespec"))
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

    let created = makespec()
        .current_dir(project.path())
        .args(["new", "adr", "--title", "Document a focused choice"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut created = created;

    created
        .stdin
        .take()
        .unwrap()
        .write_all(ADR_BODY.as_bytes())
        .unwrap();

    let created = created.wait_with_output().unwrap();

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
        .args(["list", "adr", "--status", "proposed"])
        .output()
        .unwrap();

    assert!(listed.status.success());
    assert!(String::from_utf8_lossy(&listed.stdout).contains("Document a focused choice"));

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

    let mut command = makespec()
        .current_dir(project.path())
        .args(["new", "adr", "--title", "Incomplete decision"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    command
        .stdin
        .take()
        .unwrap()
        .write_all(b"## Context\n\nOnly one section.")
        .unwrap();

    let result = command.wait_with_output().unwrap();

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

    let mut command = makespec()
        .current_dir(project.path())
        .args(["new", "adr", "--title", "Body only"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    command
        .stdin
        .take()
        .unwrap()
        .write_all(b"---\nid: ADR-0001\n---\n\n## Context\n\nIssue.")
        .unwrap();

    let result = command.wait_with_output().unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("omit YAML front matter"));
}
