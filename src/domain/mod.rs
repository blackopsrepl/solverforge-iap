solverforge::planning_model! {
    root = "src/domain";

    // @solverforge:begin domain-exports
mod invigilator;
mod exam;
mod seat;
mod plan;

pub use invigilator::Invigilator;
pub use exam::Exam;
pub use seat::Seat;
pub use plan::Plan;
    // @solverforge:end domain-exports
}
