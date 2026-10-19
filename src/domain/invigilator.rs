use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

/// TODO — describe this fact.
#[problem_fact]
#[derive(Serialize, Deserialize)]
pub struct Invigilator {
    #[planning_id]
    pub id: String,
    pub name: String,
    pub index: usize,
    pub group: usize,
    pub min_workload: i64,
    pub max_workload: i64,
    pub available_slots: Vec<usize>,
}

impl Invigilator {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        index: usize,
        group: usize,
        min_workload: i64,
        max_workload: i64,
        available_slots: Vec<usize>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            index,
            group,
            min_workload,
            max_workload,
            available_slots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invigilator_construction() {
        let fact = Invigilator::new(
            "test-id",
            "test",
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        assert_eq!(fact.id, "test-id");
        assert_eq!(fact.name, "test");
        let _ = &fact.index;
        let _ = &fact.group;
        let _ = &fact.min_workload;
        let _ = &fact.max_workload;
        let _ = &fact.available_slots;
    }
}
