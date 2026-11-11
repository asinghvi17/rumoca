//! Simulation defaults from the compiled class's `experiment` annotation
//! (MLS §18.4), exposed to Flat templates as the `experiment` context key.

use minijinja::Value;
use serde::Serialize;

/// `annotation(experiment(StartTime, StopTime, Tolerance, Interval))` of the
/// compiled root class. Unset (and non-finite) entries are omitted from the
/// template value; with none set the template sees `experiment` as `none`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct TemplateExperiment {
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
    pub tolerance: Option<f64>,
    pub interval: Option<f64>,
}

impl TemplateExperiment {
    pub(crate) fn context_value(&self) -> Value {
        let entries: Vec<(&str, f64)> = [
            ("StartTime", self.start_time),
            ("StopTime", self.stop_time),
            ("Tolerance", self.tolerance),
            ("Interval", self.interval),
        ]
        .into_iter()
        .filter_map(|(key, value)| Some((key, value.filter(|v| v.is_finite())?)))
        .collect();
        if entries.is_empty() {
            return Value::from(());
        }
        Value::from_iter(entries)
    }
}
