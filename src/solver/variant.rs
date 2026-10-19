use serde::{Deserialize, Serialize};

/// Which IAP objective a plan is solved for.
///
/// The variant is plan data, not process state: it travels with the posted
/// plan (see `Plan::variant`) so one server can solve either objective.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    /// IAP-CM: maximize distinct interdepartmental contacts.
    #[default]
    Cm,
    /// IAP-CR: minimize the total number of contacts.
    Cr,
}

impl Variant {
    /// Whether this variant maximizes contact count instead of penalizing it.
    pub fn maximize(self) -> bool {
        self == Variant::Cm
    }
}

/// The default variant of a plan that arrives without one, read once per
/// process from `IAP_VARIANT` (supported values: `cm`, `cr`; default `cm`).
pub fn variant() -> Variant {
    static VARIANT: std::sync::OnceLock<Variant> = std::sync::OnceLock::new();
    *VARIANT.get_or_init(|| match std::env::var("IAP_VARIANT").as_deref() {
        Ok("cr") => Variant::Cr,
        _ => Variant::Cm,
    })
}
