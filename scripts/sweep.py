#!/usr/bin/env python3
"""Sweep the IAP benchmark: for each variant (cm, cr) and each instance,
POST the plan to the app's /jobs, poll to a terminal lifecycle state, and
persist the snapshot + per-constraint analysis to results/.
"""
import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error

ROOT = os.path.dirname(os.path.abspath(__file__))
APP = os.path.abspath(os.path.join(ROOT, ".."))
RESULTS = os.path.join(ROOT, "..", "results")
INSTANCES = [
    "Input_UG1",
    "Input_UG2",
    "Input_UG3",
    "Input_UG4",
    "Input_UG5",
    "LargerInput_ITC1",
    "LargerInput_ITC2",
    "LargerInput_ITC3",
    "LargerInput_ITC4",
    "LargerInput_ITC5",
    "LargerInput_ITC6",
    "LargerInput_ITC7",
    "LargerInput_ITC8",
    "LargerInput_ITC9",
    "LargerInput_ITC10",
    "LargerInput_ITC11",
    "LargerInput_ITC12",
]


def api(base, path, method="GET", body=None, timeout=30):
    req = urllib.request.Request(base + path, method=method)
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        req.add_header("content-type", "application/json")
    with urllib.request.urlopen(req, data=data, timeout=timeout) as resp:
        body = resp.read()
        return json.loads(body) if body.strip() else None


def assert_port_free(port):
    """A listener must actually answer, not merely leave TIME_WAIT remnants.

    The sweep server binds with SO_REUSEADDR (tokio default), so only a live
    listener is a real collision."""
    import socket

    probe = socket.socket()
    probe.settimeout(1.0)
    try:
        probe.connect(("127.0.0.1", port))
    except OSError:
        return
    else:
        raise SystemExit(
            f"port {port} already has a live listener; kill stale"
            " solverforge_iap processes or pick another sweep server pair"
        )
    finally:
        probe.close()


def wait_health(base, deadline=120):
    end = time.time() + deadline
    while time.time() < end:
        try:
            r = api(base, "/health", timeout=3)
            if r.get("status") == "UP":
                return True
        except Exception:
            time.sleep(0.5)
    return False


def solve_job(base, plan, hard_timeout):
    job = api(base, "/jobs", method="POST", body=plan)
    job_id = job["id"]
    t0 = time.time()
    terminal = None
    while time.time() - t0 < hard_timeout:
        status = api(base, f"/jobs/{job_id}/status")
        state = status.get("lifecycleState")
        if state in ("COMPLETED", "FAILED", "CANCELLED"):
            terminal = state
            break
        time.sleep(1.0)
    snap = api(base, f"/jobs/{job_id}/snapshot")  # full snapshot with solution
    try:
        analysis = api(base, f"/jobs/{job_id}/analysis")
    except Exception:
        analysis = None
    return job_id, terminal, snap, analysis


def positive_seconds(value):
    seconds = int(value)
    if not 1 <= seconds <= 3600:
        raise argparse.ArgumentTypeError("seconds must be between 1 and 3600")
    return seconds


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("instances", nargs="*", help="instance IDs; default: all 17")
    parser.add_argument(
        "--seconds",
        type=positive_seconds,
        default=60,
        help="solve budget per job (default: 60)",
    )
    parser.add_argument(
        "--output", help="output directory (default: results/<seconds>s)"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print the run configuration without starting servers or jobs",
    )
    args = parser.parse_args()
    only = args.instances or INSTANCES
    unknown = sorted(set(only) - set(INSTANCES))
    if unknown:
        parser.error("unknown instances: " + ", ".join(unknown))
    output = args.output or os.path.join(RESULTS, f"{args.seconds}s")
    hard_timeout = args.seconds + 120
    print(
        f"{len(only)} instances × 2 variants; {args.seconds}s per job; "
        f"{hard_timeout}s timeout; output: {output}",
        flush=True,
    )
    if args.dry_run:
        return
    os.makedirs(output, exist_ok=True)
    for variant in ("cm", "cr"):
        port = 7860 if variant == "cm" else 7861
        assert_port_free(port)
        base = f"http://127.0.0.1:{port}"
        env = dict(os.environ, IAP_VARIANT=variant, PORT=str(port))
        log_path = os.path.join(output, f"server-{variant}.log")
        with open(log_path, "w") as log:
            server = subprocess.Popen(
                [os.path.join(APP, "target", "release", "solverforge_iap")],
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
                cwd=APP,
            )
            failures = []
            try:
                if not wait_health(base):
                    print(f"[{variant}] server did not come up", file=sys.stderr)
                    continue
                print(f"[{variant}] server up on {base}")
                for inst in only:
                    try:
                        plan = api(base, f"/instances/{inst}")
                        plan["duration_seconds"] = args.seconds
                        job_id, terminal, snap, analysis = solve_job(
                            base, plan, hard_timeout=hard_timeout
                        )
                    except urllib.error.HTTPError as e:
                        e.read()
                        failures.append((inst, f"HTTP {e.code}"))
                        print(
                            f"[{variant}] {inst:22s} FAILED      HTTP {e.code}",
                            file=sys.stderr,
                        )
                        continue
                    out = {
                        "instance": inst,
                        "variant": variant,
                        "job_id": job_id,
                        "terminal": terminal,
                        "snapshot": snap,
                        "analysis": analysis,
                    }
                    with open(os.path.join(output, f"{variant}-{inst}.json"), "w") as f:
                        json.dump(out, f)
                    best = snap.get("bestScore")
                    cur = snap.get("currentScore")
                    print(
                        f"[{variant}] {inst:22s} {str(terminal):10s} best={best} current={cur}"
                    )
                    # Free the job slot: the runtime retains at most MAX_JOBS
                    # jobs per server, and terminal jobs hold a slot until
                    # deleted (the same cleanup the workbench performs).
                    try:
                        api(base, f"/jobs/{job_id}", method="DELETE")
                    except urllib.error.HTTPError:
                        pass
            finally:
                server.terminate()
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()
                if failures:
                    print(
                        f"[{variant}] failures: "
                        + ", ".join(f"{i} ({r})" for i, r in failures),
                        file=sys.stderr,
                    )
                server.terminate()
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()


if __name__ == "__main__":
    main()
