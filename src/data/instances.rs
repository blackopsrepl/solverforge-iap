//! Real IAP benchmark instances (converted from the instance ZIP by
//! `scripts/convert_instances.py`). The JSON files compile into the binary via
//! `include_str!`; each parses into the domain `Plan` on demand.

use std::sync::OnceLock;

use serde::Serialize;

use crate::domain::Plan;

/// The benchmark set, in paper-table order for the report.
pub const INSTANCE_IDS: &[&str] = &[
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
];

fn raw(id: &str) -> &'static str {
    match id {
        "Input_UG1" => include_str!("instances/Input_UG1.json"),
        "Input_UG2" => include_str!("instances/Input_UG2.json"),
        "Input_UG3" => include_str!("instances/Input_UG3.json"),
        "Input_UG4" => include_str!("instances/Input_UG4.json"),
        "Input_UG5" => include_str!("instances/Input_UG5.json"),
        "LargerInput_ITC1" => include_str!("instances/LargerInput_ITC1.json"),
        "LargerInput_ITC2" => include_str!("instances/LargerInput_ITC2.json"),
        "LargerInput_ITC3" => include_str!("instances/LargerInput_ITC3.json"),
        "LargerInput_ITC4" => include_str!("instances/LargerInput_ITC4.json"),
        "LargerInput_ITC5" => include_str!("instances/LargerInput_ITC5.json"),
        "LargerInput_ITC6" => include_str!("instances/LargerInput_ITC6.json"),
        "LargerInput_ITC7" => include_str!("instances/LargerInput_ITC7.json"),
        "LargerInput_ITC8" => include_str!("instances/LargerInput_ITC8.json"),
        "LargerInput_ITC9" => include_str!("instances/LargerInput_ITC9.json"),
        "LargerInput_ITC10" => include_str!("instances/LargerInput_ITC10.json"),
        "LargerInput_ITC11" => include_str!("instances/LargerInput_ITC11.json"),
        "LargerInput_ITC12" => include_str!("instances/LargerInput_ITC12.json"),
        _ => "",
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstanceInfo {
    pub id: &'static str,
    pub exams: usize,
    pub invigilators: usize,
    pub seats: usize,
    pub slots: usize,
    pub departments: usize,
}

/// Parses one benchmark instance into the domain plan. Data files are
/// repo-owned: a malformed file fails loudly instead of serving a broken plan.
///
/// After parsing, projects the invigilator availability facts onto each seat's
/// `candidates` (the candidate-values hook's backing data). Idempotent.
pub fn load(id: &str) -> Option<Plan> {
    let raw = raw(id);
    if raw.is_empty() {
        return None;
    }
    let mut plan: Plan = serde_json::from_str(raw)
        .unwrap_or_else(|e| panic!("instance {id} is not a valid plan: {e}"));
    // Instances are objective-free benchmark data: the objective comes from the
    // server (`IAP_VARIANT`) unless a client overrides it on the posted job.
    // `#[serde(default)]` on the field would otherwise pin every instance to CM.
    plan.variant = crate::solver::variant::variant();
    derive_seat_candidates(&mut plan);
    Some(plan)
}

/// Projects invigilator availability onto each seat: `candidates` = the
/// invigilator indexes available at that seat's exam timeslot, and `maximize`
/// = the plan's objective sense. Pure function over the factual instance data;
/// the `availability` constraint remains the scored statement of the rule.
pub fn derive_seat_candidates(plan: &mut Plan) {
    let slot_count = plan
        .seats
        .iter()
        .map(|seat| seat.slot + 1)
        .max()
        .unwrap_or(0);
    plan.seats_by_slot = vec![Vec::new(); slot_count].into_boxed_slice();
    for (index, seat) in plan.seats.iter().enumerate() {
        plan.seats_by_slot[seat.slot].push(index);
    }
    for seat in &mut plan.seats {
        seat.candidates = plan
            .invigilators
            .iter()
            .filter(|invigilator| invigilator.available_slots.contains(&seat.slot))
            .map(|invigilator| invigilator.index)
            .collect();
        // The `contacts` group key reads the objective sense off the entity.
        seat.maximize = plan.variant.maximize();
    }
}

fn info_for(id: &'static str) -> InstanceInfo {
    let value: serde_json::Value = serde_json::from_str(raw(id)).expect("instance json");
    let count = |key: &str| value[key].as_array().map(Vec::len).unwrap_or(0);
    InstanceInfo {
        id,
        exams: count("exams"),
        invigilators: count("invigilators"),
        seats: count("seats"),
        slots: value["slots"].as_u64().unwrap_or(0) as usize,
        departments: count("departments"),
    }
}

/// Cached catalog for `GET /instances`.
pub fn catalog() -> &'static Vec<InstanceInfo> {
    static CATALOG: OnceLock<Vec<InstanceInfo>> = OnceLock::new();
    CATALOG.get_or_init(|| INSTANCE_IDS.iter().map(|id| info_for(id)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Exam;

    #[test]
    fn every_instance_loads_and_matches_its_shape() {
        assert_eq!(INSTANCE_IDS.len(), 17);
        for id in INSTANCE_IDS {
            let plan = load(id).unwrap_or_else(|| panic!("instance {id} must load"));
            let info = INSTANCE_IDS
                .iter()
                .find(|i| **i == *id)
                .map(|id| info_for(id))
                .unwrap();
            assert_eq!(plan.exams.len(), info.exams);
            assert_eq!(plan.invigilators.len(), info.invigilators);
            assert_eq!(plan.seats.len(), info.seats);
            // One seat per required invigilator, plus dense exam indexes.
            let demanded: usize = plan
                .exams
                .iter()
                .map(|e: &Exam| e.required.max(0) as usize)
                .sum();
            assert_eq!(plan.seats.len(), demanded, "instance {id} seat demand");
            let _ = info.slots;
        }
    }

    #[test]
    fn seats_match_required_counts() {
        for id in INSTANCE_IDS {
            let plan = load(id).unwrap();
            let seats: Vec<usize> = plan.seats.iter().map(|s| s.exam_index).collect::<Vec<_>>();
            let mut per_exam: std::collections::HashMap<usize, usize> =
                std::collections::HashMap::new();
            for e in &seats {
                *per_exam.entry(*e).or_default() += 1;
            }
            for (i, exam) in plan.exams.iter().enumerate() {
                assert_eq!(
                    per_exam.get(&i).copied().unwrap_or(0),
                    exam.required.max(0) as usize,
                    "instance {id} exam {i}"
                );
            }
        }
    }

    #[test]
    fn every_instance_preserves_fact_indexes_slots_and_candidate_domains() {
        use std::collections::HashSet;
        for id in INSTANCE_IDS {
            let plan = load(id).unwrap();
            assert_eq!(
                plan.invigilators
                    .iter()
                    .map(|i| &i.id)
                    .collect::<HashSet<_>>()
                    .len(),
                plan.invigilators.len(),
                "{id} unique invigilator IDs"
            );
            assert_eq!(
                plan.exams
                    .iter()
                    .map(|e| &e.id)
                    .collect::<HashSet<_>>()
                    .len(),
                plan.exams.len(),
                "{id} unique exam IDs"
            );
            assert_eq!(
                plan.seats
                    .iter()
                    .map(|s| &s.id)
                    .collect::<HashSet<_>>()
                    .len(),
                plan.seats.len(),
                "{id} unique seat IDs"
            );
            for (index, invigilator) in plan.invigilators.iter().enumerate() {
                assert_eq!(
                    index, invigilator.index,
                    "{id} planning values are vector indexes"
                );
                assert!(
                    0 <= invigilator.min_workload
                        && invigilator.min_workload <= invigilator.max_workload,
                    "{id} valid bounds"
                );
                assert!(
                    invigilator.available_slots.windows(2).all(|p| p[0] < p[1]),
                    "{id} sorted unique availability"
                );
            }
            for (index, exam) in plan.exams.iter().enumerate() {
                assert_eq!(index, exam.index, "{id} dense exam indexes");
                assert!(exam.required >= 0, "{id} nonnegative demand");
                let positions: HashSet<_> = plan
                    .seats
                    .iter()
                    .filter(|s| s.exam_index == index)
                    .map(|s| s.position)
                    .collect();
                assert_eq!(
                    positions,
                    (0..exam.required as usize).collect(),
                    "{id} exact positions for exam {index}"
                );
            }
            for seat in &plan.seats {
                let exam = plan
                    .exams
                    .get(seat.exam_index)
                    .expect("seat references an existing exam");
                assert_eq!(
                    seat.slot, exam.slot,
                    "{id} duplicated slot agrees with exam fact"
                );
                let expected: Vec<_> = plan
                    .invigilators
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.available_slots.contains(&exam.slot))
                    .map(|(index, _)| index)
                    .collect();
                assert_eq!(
                    seat.candidates, expected,
                    "{id} complete native candidate domain"
                );
                assert!(
                    seat.invigilator_idx.is_none(),
                    "{id} no hand-built initial solution"
                );
            }
        }
    }

    #[test]
    fn unknown_id_is_none() {
        assert!(load("nope").is_none());
    }

    #[test]
    fn candidates_are_projected_from_availability() {
        let plan = load("Input_UG1").unwrap();
        for seat in plan.seats.iter().take(50) {
            assert!(seat.invigilator_idx.is_none());
            let available: Vec<usize> = plan
                .invigilators
                .iter()
                .filter(|i| i.available_slots.contains(&seat.slot))
                .map(|i| i.index)
                .collect();
            assert_eq!(seat.candidates, available);
        }
    }
}
