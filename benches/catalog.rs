use std::{
    env, fs,
    process::{Command, Stdio},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const BINARY: &str = env!("CARGO_BIN_EXE_makespec");
const DEFAULT_DOCUMENTS: u32 = 500;
const WARMUP_RUNS: &str = "3";
const BODY: &str = "## Context\n\nThe benchmark needs a focused decision record.\n\n## Decision\n\nUse the benchmark document.\n\n## Consequences\n\nThe synthetic record remains valid for the check command.";

fn main() -> Result<()> {
    let arguments: Vec<_> = env::args()
        .skip(1)
        .filter(|argument| argument != "--bench")
        .collect();
    let documents = match arguments.as_slice() {
        [] => DEFAULT_DOCUMENTS,
        [documents] => documents.parse()?,
        _ => return Err("usage: cargo bench --bench catalog -- [DOCUMENTS]".into()),
    };
    let last = documents
        .checked_add(1)
        .filter(|_| documents > 0)
        .ok_or("document count must be between 1 and 4294967294")?;
    let project = tempfile::tempdir()?;

    run(Command::new(BINARY)
        .arg("init")
        .current_dir(project.path())
        .stdout(Stdio::null()))?;

    for number in 2..=last {
        let title = format!("Benchmark decision {number:04}");
        let content = format!(
            "---\nid: ADR-{number:04}\ntitle: {title}\ndate: 2026-01-01\nstatus: proposed\n---\n\n# ADR-{number:04}: {title}\n\n{BODY}\n"
        );
        let path = project.path().join(format!(
            "docs/decisions/{number:04}-benchmark-decision-{number:04}.md"
        ));

        fs::write(path, content)?;
    }

    let binary = format!("'{}'", BINARY.replace('\'', "'\\''"));

    run(Command::new("hyperfine")
        .current_dir(project.path())
        .args([
            "--shell=none",
            "--warmup",
            WARMUP_RUNS,
            "--metrics",
            "time_wall_clock:ms,memory_peak_resident:MiB",
        ])
        .arg(format!("{binary} list"))
        .arg(format!("{binary} check")))
}

fn run(command: &mut Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .map_err(|error| format!("could not run {program}: {error}"))?;

    if !status.success() {
        return Err(format!("{program} failed ({status})").into());
    }

    Ok(())
}
