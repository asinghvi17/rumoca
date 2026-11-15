//! Source attribute presentation from finalized Flat occurrence ranks.

mod ranks;

#[cfg(test)]
mod tests;

use crate::{Model, ModelShapeContractError, Variable};
use rumoca_core::{
    DefId, EnumerationDeclaration, Expression, Function, FunctionInstanceId, InstanceId, Span,
    VarName,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAttributeKind {
    Start,
    Min,
    Max,
    Nominal,
}

/// Original attribute expression plus its source presentation choice. This
/// projection reports rank; typechecking owns extent compatibility.
#[derive(Debug)]
pub struct SourceAttributeValue<'model> {
    kind: SourceAttributeKind,
    expression: &'model Expression,
    minimum_rank: usize,
    each: bool,
}

impl<'model> SourceAttributeValue<'model> {
    pub fn kind(&self) -> SourceAttributeKind {
        self.kind
    }
    pub fn expression(&self) -> &'model Expression {
        self.expression
    }
    /// Zero proves scalar; a positive lower bound proves an array.
    /// Exact vectorization rank need not be available for this presentation.
    pub fn minimum_rank(&self) -> usize {
        self.minimum_rank
    }
    pub fn each(&self) -> bool {
        self.each
    }
}

#[derive(Debug)]
pub enum SourceAttributeError {
    MissingVariable(InstanceId),
    UnknownShape {
        variable: InstanceId,
        attribute: SourceAttributeKind,
        span: Span,
    },
}

/// One immutable borrow of a finalized canonical model; no cached shape sidecar.
pub struct SourceAttributeCatalog<'model> {
    variables: HashMap<InstanceId, &'model Variable>,
    occurrences: HashMap<InstanceId, (&'model VarName, usize)>,
    functions: HashMap<FunctionInstanceId, &'model Function>,
    fields: HashMap<DefId, usize>,
    enumerations: HashMap<DefId, &'model EnumerationDeclaration>,
}

impl Model {
    pub fn source_attributes(&self) -> Result<SourceAttributeCatalog<'_>, ModelShapeContractError> {
        self.validate()?;
        Ok(SourceAttributeCatalog {
            variables: self
                .variables
                .values()
                .map(|variable| (variable.instance_id, variable))
                .collect(),
            occurrences: self
                .variables
                .values()
                .map(|variable| (variable.instance_id, (&variable.name, variable.dims.len())))
                .chain(
                    self.record_instances
                        .iter()
                        .map(|(name, record)| (record.instance_id, (name, record.dims.len()))),
                )
                .collect(),
            functions: self
                .functions
                .values()
                .filter_map(|function| Some((function.instance_id?, function)))
                .collect(),
            fields: self
                .record_types
                .values()
                .flat_map(|record| {
                    record
                        .fields
                        .iter()
                        .map(|field| (field.def_id, field.dims.len()))
                })
                .collect(),
            enumerations: self
                .type_ids_by_def_id
                .iter()
                .filter_map(|(def_id, type_id)| {
                    self.enumeration_declarations
                        .get(type_id)
                        .map(|declaration| (*def_id, declaration))
                })
                .collect(),
        })
    }
}

impl<'model> SourceAttributeCatalog<'model> {
    /// Query one attribute independently. An unrequested attribute cannot
    /// prevent presentation of another attribute on the same occurrence.
    pub fn attribute(
        &self,
        instance: InstanceId,
        kind: SourceAttributeKind,
    ) -> Result<Option<SourceAttributeValue<'model>>, SourceAttributeError> {
        let variable = self
            .variables
            .get(&instance)
            .ok_or(SourceAttributeError::MissingVariable(instance))?;
        let expression = match kind {
            SourceAttributeKind::Start => variable.start.as_ref(),
            SourceAttributeKind::Min => variable.min.as_ref(),
            SourceAttributeKind::Max => variable.max.as_ref(),
            SourceAttributeKind::Nominal => variable.nominal.as_ref(),
        };
        let Some(expression) = expression else {
            return Ok(None);
        };
        let minimum_rank = self
            .rank(expression)
            .or_else(|| self.array_rank_lower_bound(expression))
            .ok_or(SourceAttributeError::UnknownShape {
                variable: instance,
                attribute: kind,
                span: expression.span().unwrap_or(variable.source_span),
            })?;
        Ok(Some(SourceAttributeValue {
            kind,
            expression,
            minimum_rank,
            each: !variable.dims.is_empty() && minimum_rank == 0,
        }))
    }
}

impl std::fmt::Display for SourceAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source attribute presentation refused: {self:?}")
    }
}

impl std::error::Error for SourceAttributeError {}
