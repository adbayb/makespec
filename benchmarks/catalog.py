#!/usr/bin/env python3
"""Measure Makespec list/check and TUI behavior on a synthetic catalog."""

import json
import os
import pty
import resource
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time
import fcntl
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


def drain_until_quiet(master, process, timeout=5.0, quiet=0.15):
    deadline = time.monotonic() + timeout
    last_output = None
    captured = bytearray()
    while time.monotonic() < deadline:
        wait = 0.05
        if last_output is not None:
            wait = min(wait, max(0.0, quiet - (time.monotonic() - last_output)))
            if wait == 0:
                return bytes(captured)
        readable, _, _ = select.select([master], [], [], wait)
        if readable:
            try:
                chunk = os.read(master, 65536)
            except OSError:
                break
            if not chunk:
                break
            captured.extend(chunk)
            last_output = time.monotonic()
        elif process.poll() is not None:
            break
    return bytes(captured)


def drain_for(master, process, duration):
    deadline = time.monotonic() + duration
    while time.monotonic() < deadline:
        readable, _, _ = select.select([master], [], [], min(0.1, deadline - time.monotonic()))
        if readable:
            try:
                os.read(master, 65536)
            except OSError:
                break
        if process.poll() is not None:
            break
    return time.monotonic()


def start_tui(binary, project):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
    environment = os.environ.copy()
    environment["TERM"] = "xterm-256color"
    process = subprocess.Popen(
        [binary],
        cwd=project,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=environment,
        close_fds=True,
    )
    os.close(slave)
    return master, process


def measure_tui(binary, project, mode):
    master, process = start_tui(binary, project)
    try:
        initial = drain_until_quiet(master, process)
        if process.poll() is not None:
            raise RuntimeError(f"TUI exited during startup: {initial[-500:]!r}")

        if mode == "idle":
            idle_seconds = 2.0
            started = time.monotonic()
            drain_for(master, process, idle_seconds)
            elapsed = time.monotonic() - started
            os.write(master, b"q")
            process.wait(timeout=5)
            usage = resource.getrusage(resource.RUSAGE_CHILDREN)
            cpu_seconds = usage.ru_utime + usage.ru_stime
            return {
                "idle_wall_seconds": elapsed,
                "idle_cpu_seconds": cpu_seconds,
                "idle_cpu_percent": round(100 * cpu_seconds / elapsed, 2),
                "peak_rss_bytes": peak_rss_bytes(),
            }

        latencies = []
        for _ in range(5):
            drain_until_quiet(master, process)
            started = time.perf_counter()
            os.write(master, b"\x1b[B")
            output = drain_until_quiet(master, process, timeout=2.0, quiet=0.05)
            if not output:
                raise RuntimeError("TUI did not redraw after a navigation key")
            latencies.append(time.perf_counter() - started)
        os.write(master, b"q")
        process.wait(timeout=5)
        return {
            "navigation_latency_ms": [round(value * 1000, 2) for value in latencies],
            "peak_rss_bytes": peak_rss_bytes(),
        }
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        os.close(master)


def internal_worker(arguments):
    mode, binary, project, command = arguments
    if mode == "command":
        report = measure_command(binary, project, command)
    elif mode == "tui":
        report = measure_tui(binary, project, command)
    else:
        raise ValueError(f"unknown benchmark worker {mode}")
    print(json.dumps(report))


def run_worker(mode, binary, project, command):
    result = subprocess.run(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--worker",
            mode,
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
        internal_worker(arguments[1:])
        return
    if os.name != "posix":
        raise RuntimeError("the TUI benchmark requires a POSIX pseudo-terminal")
    binary = Path(arguments[0] if arguments else "target/release/makespec").resolve()
    documents = int(arguments[1]) if len(arguments) > 1 else 500
    if documents < 10:
        raise ValueError("use at least 10 synthetic documents to measure navigation")
    if not binary.is_file():
        raise FileNotFoundError(f"Makespec binary not found: {binary}")

    with tempfile.TemporaryDirectory(prefix="makespec-benchmark-") as directory:
        project = Path(directory)
        create_catalog(str(binary), project, documents)
        report = {
            "synthetic_adr_documents": documents,
            "list": run_worker("command", binary, project, "list"),
            "check": run_worker("command", binary, project, "check"),
            "tui_idle": run_worker("tui", binary, project, "idle"),
            "tui_navigation": run_worker("tui", binary, project, "navigation"),
        }
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main(sys.argv[1:])
