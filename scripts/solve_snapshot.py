#!/usr/bin/env python3
"""Solve one instance in one shot: health-check the app, POST the plan with a
duration, poll to a terminal lifecycle state, then print the score alongside an
independently recomputed contact count."""
import json
import os
import sys
import time
import urllib.request

BASE = "http://127.0.0.1:" + os.environ.get("PORT", "7860")
instance = sys.argv[1] if len(sys.argv) > 1 else "Input_UG1"
seconds = int(sys.argv[2]) if len(sys.argv) > 2 else 60


def call(path, body=None):
    data = None if body is None else json.dumps(body).encode()
    extra = {} if data is None else {"content-type": "application/json"}
    with urllib.request.urlopen(
        urllib.request.Request(BASE + path, data=data, headers=extra)
    ) as resp:
        return json.loads(resp.read().decode())


while "status" not in json.dumps(call("/health")):
    time.sleep(0.5)
plan = call("/instances/" + instance)
plan["duration_seconds"] = seconds
job = call("/jobs", plan)["id"]
while True:
    s = call("/jobs/%s/status" % job)
    if s["lifecycleState"] in ("COMPLETED", "CANCELLED", "FAILED"):
        break
    time.sleep(0.5)
snap = call("/jobs/%s/snapshot" % job)["solution"]
teams = {}
for seat in snap["seats"]:
    teams.setdefault(seat["exam_index"], set()).add(seat["invigilator_idx"])
pairs = set()
for team in teams.values():
    members = sorted(i for i in team if i is not None)
    for a, member in enumerate(members):
        for other in members[a + 1 :]:
            pairs.add((member, other))
print(
    json.dumps(
        {
            "instance": instance,
            "seconds": seconds,
            "score": s["bestScore"],
            "independent_contact_pairs": len(pairs),
            "hard_score": s["bestScore"].split("/")[0],
        },
        indent=1,
    )
)
