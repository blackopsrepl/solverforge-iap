#!/usr/bin/env python3
"""Sweep the IAP benchmark: for each variant (cm, cr) and each instance,
POST the plan to the app's /jobs, poll to a terminal lifecycle state, and
persist the snapshot + per-constraint analysis to results/.
"""
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
    import socket

    probe = socket.socket()
    try:
        probe.bind(("127.0.0.1", port))
    except OSError as e:
        raise SystemExit(
            f"port {port} is already in use ({e}); kill stale solverforge_iap"
            " processes or pick another sweep server pair"
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


def main():
    os.makedirs(RESULTS, exist_ok=True)
    only = sys.argv[1:] or INSTANCES
    for variant in ("cm", "cr"):
        port = 7860 if variant == "cm" else 7861
        assert_port_free(port)
        base = f"http://127.0.0.1:{port}"
        env = dict(os.environ, IAP_VARIANT=variant, PORT=str(port))
        log_path = os.path.join(RESULTS, f"server-{variant}.log")
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
                        job_id, terminal, snap, analysis = solve_job(
                            base, plan, hard_timeout=180
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
                    with open(
                        os.path.join(RESULTS, f"{variant}-{inst}.json"), "w"
                    ) as f:
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
