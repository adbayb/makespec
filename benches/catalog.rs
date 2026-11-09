#[cfg(any(target_os = "linux", target_os = "macos"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    benchmark::run()
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn main() {
    eprintln!("The catalog benchmark requires Linux or macOS for peak-RSS measurements.");
    std::process::exit(1);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod benchmark {
    use std::{
        env, fs,
        path::Path,
        process::{Command, Output, Stdio},
        time::Instant,
    };

    use nix::sys::resource::{UsageWho, getrusage};

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    const BINARY: &str = env!("CARGO_BIN_EXE_makespec");
    const BODY: &str = "## Context\n\nThe benchmark needs a focused decision record.\n\n## Decision\n\nUse the benchmark document.\n\n## Consequences\n\nThe synthetic record remains valid for the check command.";

    pub(super) fn run() -> Result<()> {
        let arguments: Vec<_> = env::args()
            .skip(1)
            .filter(|argument| argument != "--bench")
            .collect();

        match arguments.as_slice() {
            [mode, project, command] if mode == "--worker" => measure(Path::new(project), command),
            [] => catalog(500),
            [documents] => catalog(documents.parse()?),
            _ => Err("usage: cargo bench --bench catalog -- [DOCUMENTS]".into()),
        }
    }

    fn catalog(documents: u32) -> Result<()> {
        let last = documents
            .checked_add(1)
            .filter(|_| documents > 0)
            .ok_or("document count must be between 1 and 4294967294")?;
        let project = tempfile::tempdir()?;

        output(
            Command::new(BINARY)
                .arg("init")
                .arg(project.path())
                .arg("--no-skills"),
        )?;

        for number in 2..=last {
            let title = format!("Benchmark decision {number:04}");
            let content = format!(
                "---\nid: ADR-{number:04}\ntitle: {title}\ndate: 2026-01-01\nstatus: proposed\n---\n\n# ADR-{number:04}: {title}\n\n{BODY}\n"
            );
            let path = project.path().join(format!(
                "docs/adrs/{number:04}-benchmark-decision-{number:04}.md"
            ));

            fs::write(path, content)?;
        }

        let list = worker(project.path(), "list")?;
        let check = worker(project.path(), "check")?;

        println!(
            "{{\n  \"synthetic_adr_documents\": {documents},\n  \"list\": {},\n  \"check\": {}\n}}",
            list.trim(),
            check.trim()
        );

        Ok(())
    }

    fn worker(project: &Path, command: &str) -> Result<String> {
        let result = output(
            Command::new(env::current_exe()?)
                .arg("--worker")
                .arg(project)
                .arg(command),
        )?;

        Ok(String::from_utf8(result.stdout)?)
    }

    fn measure(project: &Path, command: &str) -> Result<()> {
        let started = Instant::now();

        output(
            Command::new(BINARY)
                .arg(command)
                .current_dir(project)
                .stdout(Stdio::null()),
        )?;

        let elapsed = started.elapsed().as_secs_f64();
        let peak_rss = u64::try_from(getrusage(UsageWho::RUSAGE_CHILDREN)?.max_rss())?;
        let peak_rss = if cfg!(target_os = "macos") {
            peak_rss
        } else {
            peak_rss.checked_mul(1024).ok_or("peak RSS overflow")?
        };

        println!("{{\"wall_seconds\": {elapsed}, \"peak_rss_bytes\": {peak_rss}}}");

        Ok(())
    }

    fn output(command: &mut Command) -> Result<Output> {
        let output = command.output()?;

        if !output.status.success() {
            return Err(format!(
                "{} failed ({}): {}",
                command.get_program().to_string_lossy(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }

        Ok(output)
    }
}
