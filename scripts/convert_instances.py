#!/usr/bin/env python3
"""Convert the IAP instance ZIP (exam + invigilator sections) into plan JSON
files consumed by the solverforge-iap app (src/data/instances/*.json).

Decoding rules (verified Feasibility-consistent on all 17 instances):
- Section 1 (exams): Exam, Time slot, Room, Inv. Required; timeslots are
  0-based ints matching the "TS k" columns of the invigilator section.
- Section 2 (invigilators): Invigilator, Group, FTE, Min Inv., Max. Inv, then
  one column per timeslot (TS 1 = slot 0). A positive cell marks availability;
  blank and zero both mean unavailable. Positive values are preference ranks,
  NOT slot identifiers (the larger instances explicitly zero-fill missing ranks).
- Departments: the invigilator Group letters are mapped to dense 0-based ints
  over the sorted distinct letters of each instance (injective, deterministic).
"""
import csv
import json
import os
from decimal import Decimal, InvalidOperation
import re
import sys
import tempfile
import zipfile

ZIP = "/home/pvd/.hermes/attachments/instances.zip"
OUT = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "src", "data", "instances"
)


def nonnegative_integer(text):
    """Accept spreadsheet integer decimals without silently rounding counts."""
    try:
        value = Decimal(text.strip())
    except InvalidOperation as error:
        raise ValueError(f"not an integer: {text!r}") from error
    if not value.is_finite() or value < 0 or value != value.to_integral_value():
        raise ValueError(f"not a nonnegative integer: {text!r}")
    return int(value)


def convert(path):
    with open(path, encoding="utf-8-sig", newline="") as source:
        rows = list(csv.reader(source))
    inv_start = None
    for i, r in enumerate(rows):
        if r and r[0].strip() == "Invigilator":
            inv_start = i
            break
    if inv_start is None:
        raise ValueError(f"no invigilator header in {path}")

    exams = []
    for r in rows[1:inv_start]:
        if not (r and r[0].strip()):
            continue
        slot = nonnegative_integer(r[1])
        room = r[2].strip()
        exams.append(
            {
                "id": f"exam-{r[0].strip()}",
                "name": f"exam {r[0].strip()}",
                "slot": slot,
                "room": room,
                "required": nonnegative_integer(r[3]),
            }
        )

    hdr = rows[inv_start]
    slot_columns = {}
    seen_slots = set()
    for col, header in enumerate(hdr):
        match = re.fullmatch(r"TS\s+([1-9][0-9]*)", header.strip(), re.IGNORECASE)
        if not match:
            continue
        slot = int(match.group(1)) - 1
        if slot in seen_slots:
            # ITC6 contains a second, entirely empty trailing 'TS 16'.
            # Do not relabel it as TS 17 or merge nonempty duplicate columns.
            if any(
                col < len(row) and row[col].strip() for row in rows[inv_start + 1 :]
            ):
                raise ValueError(
                    f"nonempty duplicate timeslot header {header!r} in {path}"
                )
            continue
        slot_columns[col] = slot
        seen_slots.add(slot)
    if not slot_columns:
        raise ValueError(f"missing timeslot headers in {path}")
    if any(e["slot"] not in slot_columns.values() for e in exams):
        raise ValueError(f"exam references an unknown timeslot in {path}")
    ts_base = min(slot_columns)

    invig_raw = []
    for r in rows[inv_start + 1 :]:
        if not (r and r[0].strip()):
            continue
        if any(
            value.strip()
            for col, value in enumerate(r)
            if col >= ts_base and col not in slot_columns
        ):
            raise ValueError(f"availability under an unlabelled column in {path}")
        available = sorted(
            slot
            for col, slot in slot_columns.items()
            if col < len(r) and r[col].strip() and nonnegative_integer(r[col]) > 0
        )
        minimum, maximum = nonnegative_integer(r[3]), nonnegative_integer(r[4])
        if minimum > maximum:
            raise ValueError(
                f"inverted workload bounds for invigilator {r[0]} in {path}"
            )
        invig_raw.append(
            {
                "id": r[0].strip(),
                "group": r[1].strip(),
                "min_workload": minimum,
                "max_workload": maximum,
                "available": available,
            }
        )

    letters = sorted({v["group"] for v in invig_raw})
    group_of = {letter: rank for rank, letter in enumerate(letters)}

    invigilators = [
        {
            "id": f"inv-{v['id']}",
            "name": f"invigilator {v['id']}",
            "index": i,
            "group": group_of[v["group"]],
            "min_workload": v["min_workload"],
            "max_workload": v["max_workload"],
            "available_slots": v["available"],
        }
        for i, v in enumerate(invig_raw)
    ]

    exams = [{**e, "index": i} for i, e in enumerate(exams)]
    seats = []
    for e in exams:
        for pos in range(int(e["required"])):
            seats.append(
                {
                    "id": f"seat-{e['id']}-{pos}",
                    "exam_index": e["index"],
                    "position": pos,
                    "slot": e["slot"],
                    "invigilator_idx": None,
                }
            )

    slots = max(e["slot"] for e in exams) + 1
    return {
        "name": os.path.basename(path).removesuffix(".csv"),
        "exams": exams,
        "invigilators": invigilators,
        "seats": seats,
        "slots": slots,
        "departments": letters,
    }


def main():
    os.makedirs(OUT, exist_ok=True)
    total = 0
    # Read archive members directly; never extract untrusted paths or leave
    # fixed /tmp state behind. Temporary files honor the runtime's TMPDIR.
    with zipfile.ZipFile(ZIP) as zf, tempfile.TemporaryDirectory(
        prefix="iap-convert-"
    ) as directory:
        names = sorted(n for n in zf.namelist() if n.endswith(".csv"))
        for name in names:
            path = os.path.join(directory, os.path.basename(name))
            with open(path, "wb") as source:
                source.write(zf.read(name))
            plan = convert(path)
            out = os.path.join(OUT, plan["name"] + ".json")
            with open(out, "w") as destination:
                json.dump(plan, destination)
            seats = sum(e["required"] for e in plan["exams"])
            print(
                f"{plan['name']:24s} exams={len(plan['exams']):4d} invigs={len(plan['invigilators']):4d} "
                f"seats={seats:5d} slots={plan['slots']:3d} depts={len(plan['departments'])}"
            )
            total += 1
    print(f"\n{total} instances written to {OUT}")


if __name__ == "__main__":
    sys.exit(main())
