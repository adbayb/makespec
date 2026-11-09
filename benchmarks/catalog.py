#!/usr/bin/env python3
"""Measure Makespec list/check time and peak memory on a synthetic catalog."""

import json
import resource
import subprocess
import sys
import tempfile
import time
from pathlib import Path


BODY = """## Context

The benchmark needs a focused decision record.

## Decision

Use the benchmark document.

## Consequences

The synthetic record remains valid for the check command.
"""


def peak_rss_bytes():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    return int(usage if sys.platform == "darwin" else usage * 1024)


def measure_command(binary, project, command):
    started = time.perf_counter()
    result = subprocess.run(
        [binary, command],
        cwd=project,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    elapsed = time.perf_counter() - started
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return {
        "wall_seconds": elapsed,
        "peak_rss_bytes": peak_rss_bytes(),
    }


def run_worker(binary, project, command):
    result = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--worker",
            str(binary),
            str(project),
            command,
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def create_catalog(binary, project, documents):
    initialized = subprocess.run(
        [binary, "init", str(project), "--no-skills"],
        capture_output=True,
        text=True,
        check=False,
    )
    if initialized.returncode:
        raise RuntimeError(initialized.stderr.strip())
    for number in range(2, documents + 2):
        title = f"Benchmark decision {number:04}"
        slug = f"benchmark-decision-{number:04}"
        content = (
            "---\n"
            f"id: ADR-{number:04}\n"
            f"title: {title}\n"
            "date: 2026-01-01\n"
            "status: proposed\n"
            "---\n\n"
            f"# ADR-{number:04}: {title}\n\n"
            f"{BODY}"
        )
        (project / "docs" / "adrs" / f"{number:04}-{slug}.md").write_text(
            content, encoding="utf-8"
        )


def main(arguments):
    if arguments and arguments[0] == "--worker":
        binary, project, command = arguments[1:]
        print(json.dumps(measure_command(binary, project, command)))
        return
    binary = Path(arguments[0] if arguments else "target/release/makespec").resolve()
    documents = int(arguments[1]) if len(arguments) > 1 else 500
    if documents < 1:
        raise ValueError("use at least one synthetic document")
    if not binary.is_file():
        raise FileNotFoundError(f"Makespec binary not found: {binary}")

    with tempfile.TemporaryDirectory(prefix="makespec-benchmark-") as directory:
        project = Path(directory)
        create_catalog(str(binary), project, documents)
        report = {
            "synthetic_adr_documents": documents,
            "list": run_worker(binary, project, "list"),
            "check": run_worker(binary, project, "check"),
        }
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main(sys.argv[1:])
