use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use solverforge::{
    CandidateTraceExternalDigest, HardSoftScore, QualifiedCandidateTraceRunProvenance,
    SolverLifecycleState, SolverSnapshot, SolverSnapshotAnalysis, SolverStatus,
    SolverTelemetryDetail, SolverTerminalReason,
};

use super::telemetry::{CandidateTraceDto, TelemetryDto};

use crate::domain::Plan;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    #[serde(flatten)]
    pub fields: Map<String, Value>,
    #[serde(default)]
    pub score: Option<String>,
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for PlanDto {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Plan".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // The plan fields are domain-owned and projected through serde
        // `flatten`, so the schema stays open: any object plus the optional
        // managed `score` field.
        serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "score": { "type": ["string", "null"] }
            },
            "additionalProperties": true
        }))
        .expect("plan schema must be a valid JSON Schema object")
    }
}

/// Constraint analysis result.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConstraintAnalysisDto {
    pub name: String,
    pub weight: String,
    pub score: String,
    pub match_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub score: String,
    pub constraints: Vec<ConstraintAnalysisDto>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct JobSummaryDto {
    pub id: String,
    pub job_id: String,
    pub lifecycle_state: &'static str,
    pub terminal_reason: Option<&'static str>,
    pub checkpoint_available: bool,
    pub event_sequence: u64,
    pub snapshot_revision: Option<u64>,
    pub current_score: Option<String>,
    pub best_score: Option<String>,
    pub telemetry: TelemetryDto,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshotDto {
    pub id: String,
    pub job_id: String,
    pub snapshot_revision: u64,
    pub lifecycle_state: &'static str,
    pub terminal_reason: Option<&'static str>,
    pub current_score: Option<String>,
    pub best_score: Option<String>,
    pub telemetry: TelemetryDto,
    pub solution: PlanDto,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct JobAnalysisDto {
    pub id: String,
    pub job_id: String,
    pub snapshot_revision: u64,
    pub lifecycle_state: &'static str,
    pub terminal_reason: Option<&'static str>,
    pub analysis: AnalyzeResponse,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct JobTelemetryDetailDto {
    pub id: String,
    pub job_id: String,
    pub status: JobSummaryDto,
    pub candidate_trace: Option<CandidateTraceDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct QualifiedJobRequestDto {
    pub plan: PlanDto,
    pub provenance: QualifiedCandidateTraceProvenanceDto,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct QualifiedCandidateTraceProvenanceDto {
    pub schema_digest_sha256: String,
    pub instance_digest_sha256: String,
    pub initial_state_digest_sha256: String,
    pub core_tree_digest_sha256: String,
    pub build_digest_sha256: String,
    pub producer: String,
}

impl PlanDto {
    pub fn from_plan(plan: &Plan) -> Self {
        let mut fields = match serde_json::to_value(plan).expect("failed to serialize plan") {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        fields.remove("score");
        let score = plan.score.map(|score| score.to_string());

        Self { fields, score }
    }

    pub fn to_domain(&self) -> Result<Plan, serde_json::Error> {
        let mut fields = self.fields.clone();
        let _ = &self.score;
        fields.insert("score".to_string(), Value::Null);
        // A payload that names no variant is solved for the server's default
        // objective (`IAP_VARIANT`), not for `Variant::default()`.
        if !fields.contains_key("variant") {
            fields.insert(
                "variant".to_string(),
                serde_json::to_value(crate::solver::variant::variant())?,
            );
        }
        let mut plan: Plan = serde_json::from_value(Value::Object(fields))?;
        if !(1..=3600).contains(&plan.duration_seconds) {
            return Err(serde::de::Error::custom(
                "duration_seconds must be between 1 and 3600",
            ));
        }
        // Derives the seat candidate lists for plans posted without them
        // (demo data included). Pure projection of invigilator availability;
        // idempotent for plans that arrived pre-derived.
        crate::data::instances::derive_seat_candidates(&mut plan);
        Ok(plan)
    }
}

impl JobSummaryDto {
    pub fn from_status(job_id: usize, status: &SolverStatus<HardSoftScore>) -> Self {
        Self {
            id: job_id.to_string(),
            job_id: job_id.to_string(),
            lifecycle_state: lifecycle_state_label(status.lifecycle_state),
            terminal_reason: status.terminal_reason.map(terminal_reason_label),
            checkpoint_available: status.checkpoint_available,
            event_sequence: status.event_sequence,
            snapshot_revision: status.latest_snapshot_revision,
            current_score: status.current_score.map(|score| score.to_string()),
            best_score: status.best_score.map(|score| score.to_string()),
            telemetry: TelemetryDto::from_runtime(&status.telemetry),
        }
    }
}

impl JobTelemetryDetailDto {
    pub fn from_runtime(detail: &SolverTelemetryDetail<HardSoftScore>) -> Self {
        let job_id = detail.status.job_id;
        Self {
            id: job_id.to_string(),
            job_id: job_id.to_string(),
            status: JobSummaryDto::from_status(job_id, &detail.status),
            candidate_trace: detail
                .candidate_trace
                .as_ref()
                .map(CandidateTraceDto::from_runtime),
        }
    }
}

impl QualifiedCandidateTraceProvenanceDto {
    pub fn to_runtime(&self) -> Result<QualifiedCandidateTraceRunProvenance, String> {
        QualifiedCandidateTraceRunProvenance::externally_attested(
            parse_sha256("schemaDigestSha256", &self.schema_digest_sha256)?,
            parse_sha256("instanceDigestSha256", &self.instance_digest_sha256)?,
            parse_sha256(
                "initialStateDigestSha256",
                &self.initial_state_digest_sha256,
            )?,
            parse_sha256("coreTreeDigestSha256", &self.core_tree_digest_sha256)?,
            parse_sha256("buildDigestSha256", &self.build_digest_sha256)?,
            self.producer.clone(),
        )
        .map_err(|error| error.to_string())
    }
}

impl JobSnapshotDto {
    pub fn from_snapshot(snapshot: &SolverSnapshot<Plan>) -> Self {
        Self {
            id: snapshot.job_id.to_string(),
            job_id: snapshot.job_id.to_string(),
            snapshot_revision: snapshot.snapshot_revision,
            lifecycle_state: lifecycle_state_label(snapshot.lifecycle_state),
            terminal_reason: snapshot.terminal_reason.map(terminal_reason_label),
            current_score: snapshot.current_score.map(|score| score.to_string()),
            best_score: snapshot.best_score.map(|score| score.to_string()),
            telemetry: TelemetryDto::from_runtime(&snapshot.telemetry),
            solution: PlanDto::from_plan(&snapshot.solution),
        }
    }
}

impl JobAnalysisDto {
    pub fn from_snapshot_analysis(
        snapshot: &SolverSnapshotAnalysis<HardSoftScore>,
        analysis: AnalyzeResponse,
    ) -> Self {
        Self {
            id: snapshot.job_id.to_string(),
            job_id: snapshot.job_id.to_string(),
            snapshot_revision: snapshot.snapshot_revision,
            lifecycle_state: lifecycle_state_label(snapshot.lifecycle_state),
            terminal_reason: snapshot.terminal_reason.map(terminal_reason_label),
            analysis,
        }
    }
}

pub fn analysis_response(analysis: &solverforge::ScoreAnalysis<HardSoftScore>) -> AnalyzeResponse {
    AnalyzeResponse {
        score: analysis.score.to_string(),
        constraints: analysis
            .constraints
            .iter()
            .map(|constraint| ConstraintAnalysisDto {
                name: constraint.name.clone(),
                weight: constraint.weight.to_string(),
                score: constraint.score.to_string(),
                match_count: constraint.match_count,
            })
            .collect(),
    }
}

pub fn lifecycle_state_label(state: SolverLifecycleState) -> &'static str {
    match state {
        SolverLifecycleState::Solving => "SOLVING",
        SolverLifecycleState::PauseRequested => "PAUSE_REQUESTED",
        SolverLifecycleState::Paused => "PAUSED",
        SolverLifecycleState::Completed => "COMPLETED",
        SolverLifecycleState::Cancelled => "CANCELLED",
        SolverLifecycleState::Failed => "FAILED",
    }
}

pub fn terminal_reason_label(reason: SolverTerminalReason) -> &'static str {
    match reason {
        SolverTerminalReason::Completed => "completed",
        SolverTerminalReason::TerminatedByConfig => "terminated_by_config",
        SolverTerminalReason::Cancelled => "cancelled",
        SolverTerminalReason::Failed => "failed",
    }
}

fn parse_sha256(name: &str, value: &str) -> Result<CandidateTraceExternalDigest, String> {
    if value.len() != 64 {
        return Err(format!(
            "{name} must contain exactly 64 hexadecimal characters"
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = hex_nibble(pair[0]).ok_or_else(|| format!("{name} must be hexadecimal"))?;
        let low = hex_nibble(pair[1]).ok_or_else(|| format!("{name} must be hexadecimal"))?;
        bytes[index] = (high << 4) | low;
    }
    Ok(CandidateTraceExternalDigest::sha256(bytes))
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PlanDto {
        PlanDto::from_plan(&crate::data::instances::load("Input_UG1").unwrap())
    }

    #[test]
    fn duration_defaults_and_round_trips_through_the_job_payload() {
        let mut dto = input();
        dto.fields.remove("duration_seconds");
        assert_eq!(dto.to_domain().unwrap().duration_seconds, 60);
        for seconds in [1_u64, 17, 3600] {
            dto.fields.insert("duration_seconds".into(), seconds.into());
            let plan = dto.to_domain().unwrap();
            assert_eq!(plan.duration_seconds, seconds);
            assert_eq!(
                PlanDto::from_plan(&plan).fields["duration_seconds"],
                seconds
            );
        }
    }

    #[test]
    fn duration_rejects_out_of_range_and_non_integer_values() {
        let mut dto = input();
        for invalid in [
            serde_json::json!(0),
            serde_json::json!(3601),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            Value::Null,
        ] {
            dto.fields
                .insert("duration_seconds".into(), invalid.clone());
            assert!(dto.to_domain().is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn snapshot_score_uses_the_native_display_contract() {
        let mut plan = crate::data::instances::load("Input_UG1").unwrap();
        plan.score = Some(HardSoftScore::of(-3, 78));
        let dto = PlanDto::from_plan(&plan);
        assert_eq!(dto.score.as_deref(), Some("-3hard/78soft"));
        assert!(!dto.fields.contains_key("score"));
        assert!(dto.to_domain().unwrap().score.is_none());
    }
}
