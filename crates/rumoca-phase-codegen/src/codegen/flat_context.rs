//! Facts about the compiled model that Flat IR does not carry, exposed to Flat
//! templates beside `flat`: the root class's `experiment` annotation and the
//! parameters translation evaluated.

use std::collections::BTreeSet;

use minijinja::Value;
use rumoca_ir_dae as dae;
use rumoca_ir_flat as flat;

use super::TemplateExperiment;

/// The extra Flat template context: `experiment` (see [`TemplateExperiment`])
/// and `evaluated_parameters`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlatTemplateContext {
    pub experiment: TemplateExperiment,
    /// Flat names of the parameters whose value translation used, so a
    /// later change cannot take effect; see [`Self::evaluated_parameters`].
    pub evaluated_parameters: BTreeSet<String>,
}

impl FlatTemplateContext {
    /// The parameters of `dae` that translation evaluated: every non-tunable
    /// parameter except one computed during initialization (a `fixed = false`
    /// parameter or an initial-algorithm target) and a `final` one whose
    /// binding is not evaluable, which follows parameters that stay tunable.
    ///
    /// What remains is a parameter a structural use fixed (an array
    /// dimension, a for-equation range, a selected unbalanced if-branch, a
    /// conditional component's condition), one its binding reads, an
    /// `Evaluate = true` parameter (MLS §18.3), and a `final` one whose
    /// binding is evaluable (MLS §4.5).
    pub fn evaluated_parameters(
        dae_model: &dae::Dae,
        flat_model: &flat::Model,
    ) -> BTreeSet<String> {
        dae_model.inspect(|view| {
            view.variables()
                .filter(|(_, variable)| {
                    variable.role() == dae::VariableRole::Parameter
                        && !variable.is_tunable()
                        && variable.causality() != dae::VariableCausality::CalculatedParameter
                        && (variable.is_evaluable()
                            || !flat_model
                                .variables
                                .get(variable.name())
                                .is_some_and(|declared| declared.is_final))
                })
                .map(|(_, variable)| variable.name().to_string())
                .collect()
        })
    }

    pub(crate) fn evaluated_parameters_value(&self) -> Value {
        Value::from_iter(
            self.evaluated_parameters
                .iter()
                .map(|name| (name.clone(), true)),
        )
    }
}
