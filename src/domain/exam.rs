use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

/// TODO — describe this fact.
#[problem_fact]
#[derive(Serialize, Deserialize)]
pub struct Exam {
    #[planning_id]
    pub id: String,
    pub name: String,
    pub index: usize,
    pub slot: usize,
    pub room: String,
    pub required: i64,
}

impl Exam {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        index: usize,
        slot: usize,
        room: String,
        required: i64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            index,
            slot,
            room,
            required,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exam_construction() {
        let fact = Exam::new(
            "test-id",
            "test",
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        assert_eq!(fact.id, "test-id");
        assert_eq!(fact.name, "test");
        let _ = &fact.index;
        let _ = &fact.slot;
        let _ = &fact.room;
        let _ = &fact.required;
    }
}
