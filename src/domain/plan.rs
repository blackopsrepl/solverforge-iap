use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

// @solverforge:neutral-solution
// @solverforge:begin solution-imports
use super::Exam;
use super::Invigilator;
use super::Seat;
// @solverforge:end solution-imports

/// The root planning solution.
///
/// Fresh projects start as a neutral shell. Add fact collections, planning
/// entity collections, and variable fields through the CLI as your domain
/// takes shape.
#[planning_solution(
    constraints = "crate::constraints::create_constraints",
    solver_toml = "../../solver.toml",
    config = "configure_solver"
)]
#[derive(Serialize, Deserialize)]
pub struct Plan {
    // @solverforge:begin solution-collections
    #[problem_fact_collection]
    pub invigilators: Vec<Invigilator>,
    #[problem_fact_collection]
    pub exams: Vec<Exam>,
    #[planning_entity_collection]
    pub seats: Vec<Seat>,
    // @solverforge:end solution-collections
    #[planning_score]
    pub score: Option<HardSoftScore>,
    /// Objective this plan is solved for (IAP-CM or IAP-CR): plan data, not
    /// process state, so one server can solve either objective.
    #[serde(default)]
    pub variant: crate::solver::variant::Variant,
    /// Wall-clock solve budget supplied with each job, in seconds.
    #[serde(default = "default_duration")]
    pub duration_seconds: u64,
    /// Immutable metadata for the native same-timeslot nearby-swap selector.
    #[serde(skip)]
    pub seats_by_slot: Box<[Vec<usize>]>,
}

impl Plan {
    #[rustfmt::skip]
    pub fn new(
        // @solverforge:begin solution-constructor-params
        invigilators: Vec<Invigilator>,
        exams: Vec<Exam>,
        seats: Vec<Seat>,
        // @solverforge:end solution-constructor-params
    ) -> Self {
        Self {
            // @solverforge:begin solution-constructor-init
            invigilators,
            exams,
            seats,
            // @solverforge:end solution-constructor-init
            score: None,
            variant: crate::solver::variant::variant(),
            duration_seconds: default_duration(),
            seats_by_slot: Box::default(),
        }
    }
}

fn default_duration() -> u64 {
    60
}

fn configure_solver(plan: &Plan, config: solverforge::SolverConfig) -> solverforge::SolverConfig {
    config.with_termination_seconds(plan.duration_seconds)
}

// @solverforge:begin scalar-groups
// @solverforge:end scalar-groups
