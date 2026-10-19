#!/usr/bin/env python3
"""Solve one instance in one shot: health-check the app, POST the plan with a
duration, poll to a terminal lifecycle state, then print the score alongside
the counts recomputed independently from the seat assignment: the constraint-
relevant contact pairs (CM cross-department; CR all departments) and the hard
rule census (availability, single occupancy, workload bounds, coverage)."""
import json
import os
import sys
import time
import urllib.error
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
        raw = resp.read()
        return json.loads(raw) if raw.strip() else None


while True:
    try:
        call("/health")
        break
    except (urllib.error.URLError, ConnectionError):
        time.sleep(0.5)
plan = call("/instances/" + instance)
plan["duration_seconds"] = seconds
job = call("/jobs", plan)["id"]
while True:
    s = call(f"/jobs/{job}/status")
    if s["lifecycleState"] in ("COMPLETED", "CANCELLED", "FAILED"):
        break
    time.sleep(0.5)
snap = call(f"/jobs/{job}/snapshot")["solution"]
cm = call("/info").get("variant") == "cm"
invigilators = {i["index"]: i for i in snap["invigilators"]}
exams = {e["index"]: e for e in snap["exams"]}
teams = {}
for seat in snap["seats"]:
    teams.setdefault(seat["exam_index"], set()).add(seat["invigilator_idx"])
pairs = set()
duplicate_slot_assignments = 0
unavailable = 0
loads = {index: 0 for index in invigilators}
exams_with_unassigned_seats = 0
for index, team in teams.items():
    slot = exams[index]["slot"]
    members = sorted(i for i in team if i is not None)
    if None in team:
        exams_with_unassigned_seats += 1
    if len(members) > 1:
        duplicate_slot_assignments += len(members) - 1
    for position, member in enumerate(members):
        loads[member] += 1
        if slot not in invigilators[member]["available_slots"]:
            unavailable += 1
        for other in members[position + 1 :]:
            if not (
                cm and invigilators[member]["group"] == invigilators[other]["group"]
            ):
                pairs.add((min(member, other), max(member, other)))
out_of_bounds = sum(
    1
    for index, load in loads.items()
    if not invigilators[index]["min_workload"]
    <= load
    <= invigilators[index]["max_workload"]
)
print(
    json.dumps(
        {
            "instance": instance,
            "seconds": seconds,
            "score": s["bestScore"],
            "constraint_relevant_pairs": len(pairs),
            "hard_score": s["bestScore"].split("/")[0],
            "independent_census": {
                "unavailable_assignments": unavailable,
                "duplicate_slot_assignments": duplicate_slot_assignments,
                "workload_bound_violators": out_of_bounds,
                "exams_with_unassigned_seats": exams_with_unassigned_seats,
            },
        },
        indent=1,
    )
)
