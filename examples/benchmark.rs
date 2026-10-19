//! Reproducible benchmark entry point using the same SolverManager as the API.
use solverforge::{SolverEvent, SolverManager};
use solverforge_iap::{data::instances, domain::Plan};
use std::{env, fs, path::PathBuf, time::Instant};

static MANAGER: SolverManager<Plan> = SolverManager::new();

fn main() {
    let ids: Vec<String> = env::args().skip(1).collect();
    let ids = if ids.is_empty() {
        instances::INSTANCE_IDS
            .iter()
            .map(|id| (*id).to_owned())
            .collect()
    } else {
        ids
    };
    let variant = env::var("IAP_VARIANT").unwrap_or_else(|_| "cm".into());
    let root = PathBuf::from(env::var("IAP_RESULTS").unwrap_or_else(|_| "results".into()));
    fs::create_dir_all(&root).expect("create results directory");
    for id in ids {
        let plan = instances::load(&id).expect("known benchmark instance");
        let started = Instant::now();
        let mut first_feasible_seconds = None;
        let (job, mut events) = MANAGER.solve(plan).expect("start SolverForge job");
        let mut completed = false;
        while let Some(event) = events.blocking_recv() {
            match event {
                SolverEvent::BestSolution { metadata, .. } => {
                    if let Some(score) = metadata.best_score {
                        let elapsed = started.elapsed().as_secs_f64();
                        if score.hard() == 0 && first_feasible_seconds.is_none() {
                            first_feasible_seconds = Some(elapsed);
                        }
                        println!("BEST {variant} {id} {elapsed:.3}s {score}");
                    }
                }
                SolverEvent::Completed { solution, .. } => {
                    let analysis = MANAGER
                        .analyze_snapshot(job, None)
                        .expect("native score analysis");
                    let output = serde_json::json!({
                        "instance": id, "variant": variant, "terminal": "COMPLETED",
                        "first_feasible_seconds": first_feasible_seconds,
                        "elapsed_seconds": started.elapsed().as_secs_f64(),
                        "score": solution.score.map(|score| score.to_string()),
                        "solution": solution, "analysis": solverforge_iap::api::analysis_response(&analysis.analysis),
                    });
                    fs::write(
                        root.join(format!("{variant}-{id}.json")),
                        serde_json::to_vec(&output).unwrap(),
                    )
                    .unwrap();
                    println!("BENCH {variant} {id} {}", output["score"]);
                    completed = true;
                    break;
                }
                SolverEvent::Failed { error, .. } => panic!("{id}: {error}"),
                SolverEvent::Cancelled { .. } => panic!("{id}: cancelled"),
                _ => {}
            }
        }
        assert!(completed, "event stream closed before completion");
        MANAGER.delete(job).expect("delete retained benchmark job");
    }
}
