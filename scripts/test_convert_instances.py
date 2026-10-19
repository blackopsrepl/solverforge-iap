import csv
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from convert_instances import convert, ZIP, OUT


class ConversionTests(unittest.TestCase):
    def convert_rows(self, rows):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.csv"
            with path.open("w", newline="") as stream:
                csv.writer(stream).writerows(rows)
            return convert(path)

    def fixture(self):
        return [
            ["Exam", "Time slot 1", "Room ID", "Inv. Required"],
            ["1", "0", "room", "1"],
            ["2", "1", "room", "1"],
            ["Invigilator", "Group", "FTE", "Min", "Max", "TS 1", "TS 2", "TS 3", ""],
            ["1", "D", "1", "0", "2", "0", "2", "", ""],
            ["2", "A", "0.5", "1", "2", "1", "0", "0", ""],
        ]

    def test_zero_is_unavailable_and_positive_rank_uses_column_not_value(self):
        plan = self.convert_rows(self.fixture())
        self.assertEqual(plan["invigilators"][0]["available_slots"], [1])
        self.assertEqual(plan["invigilators"][1]["available_slots"], [0])
        self.assertEqual(plan["departments"], ["A", "D"])
        self.assertEqual([i["group"] for i in plan["invigilators"]], [1, 0])

    def test_fractional_counts_fail_instead_of_silent_truncation(self):
        rows = self.fixture()
        rows[1][3] = "1.5"
        with self.assertRaises(ValueError):
            self.convert_rows(rows)

    def test_inverted_bounds_rejected(self):
        rows = self.fixture()
        rows[4][3] = "3"
        with self.assertRaises(ValueError):
            self.convert_rows(rows)

    def test_nonempty_unlabelled_availability_is_rejected(self):
        rows = self.fixture()
        rows[4][-1] = "1"
        with self.assertRaises(ValueError):
            self.convert_rows(rows)

    def test_timeslot_headers_not_physical_column_order_define_slot_ids(self):
        rows = self.fixture()
        rows[3][5:8] = ["TS 3", "TS 1", "TS 2"]
        plan = self.convert_rows(rows)
        self.assertEqual(plan["invigilators"][0]["available_slots"], [0])
        self.assertEqual(plan["invigilators"][1]["available_slots"], [2])

    def test_duplicate_timeslot_header_only_allowed_when_entirely_empty(self):
        rows = self.fixture()
        rows[3][-1] = "TS 3"
        self.convert_rows(rows)
        rows[4][-1] = "1"
        with self.assertRaises(ValueError):
            self.convert_rows(rows)

    def test_negative_and_nonfinite_counts_rejected(self):
        for invalid in ["-1", "NaN", "Infinity"]:
            rows = self.fixture()
            rows[1][3] = invalid
            with self.subTest(value=invalid), self.assertRaises(ValueError):
                self.convert_rows(rows)

    def test_all_embedded_instances_equal_fresh_conversion(self):
        with zipfile.ZipFile(
            ZIP
        ) as archive, tempfile.TemporaryDirectory() as directory:
            names = sorted(n for n in archive.namelist() if n.endswith(".csv"))
            self.assertEqual(len(names), 17)
            for name in names:
                with self.subTest(instance=name):
                    path = Path(directory) / Path(name).name
                    path.write_bytes(archive.read(name))
                    converted = convert(path)
                    embedded = json.loads(
                        (Path(OUT) / (converted["name"] + ".json")).read_text()
                    )
                    self.assertEqual(converted, embedded)


if __name__ == "__main__":
    unittest.main()
