//! Source attribute presentation from exact finalized Flat occurrence facts.

use crate::{Model, ModelShapeContractError, Variable};
use rumoca_core::{
    BuiltinFunction, ComponentPath, Expression, Function, FunctionInstanceId, InstanceId, Literal,
    OpBinary, Reference, Span, Subscript,
};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAttributeKind {
    Start,
    Min,
    Max,
    Nominal,
}

/// Original attribute expression plus its checked source presentation choice.
#[derive(Debug)]
pub struct SourceAttributeValue<'model> {
    kind: SourceAttributeKind,
    expression: &'model Expression,
    dimensions: Box<[i64]>,
    each: bool,
}

impl<'model> SourceAttributeValue<'model> {
    pub fn kind(&self) -> SourceAttributeKind {
        self.kind
    }

    pub fn expression(&self) -> &'model Expression {
        self.expression
    }

    pub fn dimensions(&self) -> &[i64] {
        &self.dimensions
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
    ShapeMismatch {
        variable: InstanceId,
        attribute: SourceAttributeKind,
        span: Span,
    },
}

/// One immutable borrow of a finalized canonical model; no cached shape sidecar.
pub struct SourceAttributeCatalog<'model> {
    variables: HashMap<InstanceId, &'model Variable>,
    functions: HashMap<FunctionInstanceId, &'model Function>,
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
            functions: self
                .functions
                .values()
                .filter_map(|function| Some((function.instance_id?, function)))
                .collect(),
        })
    }
}

impl<'model> SourceAttributeCatalog<'model> {
    /// No partial list is returned when an attribute lacks a canonical shape.
    pub fn variable(
        &self,
        instance: InstanceId,
    ) -> Result<Vec<SourceAttributeValue<'model>>, SourceAttributeError> {
        let variable = self
            .variables
            .get(&instance)
            .ok_or(SourceAttributeError::MissingVariable(instance))?;
        let sources = [
            (SourceAttributeKind::Start, variable.start.as_ref()),
            (SourceAttributeKind::Min, variable.min.as_ref()),
            (SourceAttributeKind::Max, variable.max.as_ref()),
            (SourceAttributeKind::Nominal, variable.nominal.as_ref()),
        ];
        sources
            .into_iter()
            .filter_map(|(kind, expression)| expression.map(|expression| (kind, expression)))
            .map(|(kind, expression)| self.attribute(variable, kind, expression))
            .collect()
    }

    fn attribute(
        &self,
        variable: &Variable,
        kind: SourceAttributeKind,
        expression: &'model Expression,
    ) -> Result<SourceAttributeValue<'model>, SourceAttributeError> {
        let dimensions = self
            .dimensions(expression)
            .ok_or(SourceAttributeError::UnknownShape {
                variable: variable.instance_id,
                attribute: kind,
                span: expression.span().unwrap_or(variable.source_span),
            })?;
        if !dimensions.is_empty() && dimensions != variable.dims {
            return Err(SourceAttributeError::ShapeMismatch {
                variable: variable.instance_id,
                attribute: kind,
                span: expression.span().unwrap_or(variable.source_span),
            });
        }
        Ok(SourceAttributeValue {
            kind,
            expression,
            each: !variable.dims.is_empty() && dimensions.is_empty(),
            dimensions: dimensions.into_boxed_slice(),
        })
    }

    fn dimensions(&self, expression: &Expression) -> Option<Vec<i64>> {
        match expression {
            Expression::Literal { .. } => Some(Vec::new()),
            Expression::VarRef {
                name, subscripts, ..
            } if subscripts.is_empty() && unselected_reference(name) => {
                let variable = self.variables.get(&name.instance_id()?)?;
                // An occurrence identity cannot authorize a differently named reference.
                (ComponentPath::from_reference(name).var_name() == &variable.name)
                    .then(|| variable.dims.clone())
            }
            Expression::Array { elements, kind, .. } => {
                let operands = elements
                    .iter()
                    .map(|element| {
                        self.dimensions(element)?
                            .into_iter()
                            .map(|dimension| usize::try_from(dimension).ok())
                            .collect::<Option<Vec<_>>>()
                    })
                    .collect::<Option<Vec<_>>>()?;
                kind.checked_dimensions(&operands)?
                    .into_iter()
                    .map(|dimension| i64::try_from(dimension).ok())
                    .collect()
            }
            Expression::FunctionCall { name, args, .. } => {
                let function = self.functions.get(&name.resolved_function()?.instance_id)?;
                // Extra argument axes imply vectorization; declared output shape
                // alone cannot authorize those call occurrences.
                if args.len() != function.inputs.len()
                    || !args.iter().zip(&function.inputs).all(|(arg, input)| {
                        self.dimensions(arg).as_deref() == Some(input.dimensions())
                    })
                {
                    return None;
                }
                let output = function.outputs.first()?;
                if output
                    .shape_expr
                    .iter()
                    .any(|dim| !matches!(dim, Subscript::Index { .. }))
                {
                    return None;
                }
                Some(output.dimensions().to_vec())
            }
            Expression::Unary { rhs, .. } => self.dimensions(rhs),
            Expression::Binary { op, lhs, rhs, .. } => {
                binary_dimensions(op, self.dimensions(lhs)?, self.dimensions(rhs)?)
            }
            Expression::If {
                branches,
                else_branch,
                ..
            } => {
                let dimensions = self.dimensions(else_branch)?;
                branches
                    .iter()
                    .all(|(condition, value)| {
                        self.dimensions(condition)
                            .is_some_and(|shape| shape.is_empty())
                            && self.dimensions(value).as_ref() == Some(&dimensions)
                    })
                    .then_some(dimensions)
            }
            Expression::BuiltinCall { function, args, .. } => {
                self.builtin_dimensions(*function, args)
            }
            Expression::Index {
                base, subscripts, ..
            } => self.selected_dimensions(self.dimensions(base)?, subscripts),
            Expression::Range {
                start, step, end, ..
            } => range_dimensions(start, step.as_deref(), end),
            // Unretained selection and dependent-call shapes remain unknown.
            _ => None,
        }
    }

    fn selected_dimensions(
        &self,
        dimensions: Vec<i64>,
        subscripts: &[Subscript],
    ) -> Option<Vec<i64>> {
        if subscripts.len() > dimensions.len() {
            return None;
        }
        let mut selected = Vec::new();
        for (subscript, extent) in subscripts.iter().zip(&dimensions) {
            match subscript {
                Subscript::Colon { .. } => selected.push(*extent),
                Subscript::Index { .. } => {}
                Subscript::Expr { expr, .. } => match self.dimensions(expr)?.as_slice() {
                    [] => {}
                    [extent] => selected.push(*extent),
                    _ => return None,
                },
            }
        }
        selected.extend_from_slice(&dimensions[subscripts.len()..]);
        Some(selected)
    }

    fn builtin_dimensions(
        &self,
        function: BuiltinFunction,
        args: &[Expression],
    ) -> Option<Vec<i64>> {
        use BuiltinFunction::*;
        match (function, args) {
            (Fill, [value, sizes @ ..]) if !sizes.is_empty() => {
                let mut dimensions = literal_extents(sizes)?;
                dimensions.extend(self.dimensions(value)?);
                Some(dimensions)
            }
            (Zeros | Ones, sizes) if !sizes.is_empty() => literal_extents(sizes),
            (
                Abs | Sign | Sqrt | Floor | Ceil | Sin | Cos | Tan | Asin | Acos | Atan | Sinh
                | Cosh | Tanh | Exp | Log | Log10 | Integer | NoEvent | Pre | Der,
                [value],
            ) => self.dimensions(value),
            (Smooth, [_, value]) => self.dimensions(value),
            (Sum | Product | Min | Max | Ndims, [_]) => Some(vec![]),
            (Size, [_, _]) => Some(vec![]),
            (Min | Max | Div | Mod | Rem | Atan2, [lhs, rhs]) => {
                let lhs = self.dimensions(lhs)?;
                let rhs = self.dimensions(rhs)?;
                (lhs == rhs).then_some(lhs)
            }
            _ => None,
        }
    }
}

fn range_dimensions(
    start: &Expression,
    step: Option<&Expression>,
    end: &Expression,
) -> Option<Vec<i64>> {
    let domain = rumoca_core::StructuredIndexDomain {
        binders: vec![rumoca_core::StructuredIndexBinder {
            provenance: rumoca_core::IndexBinderProvenance::Generated,
            id: 0,
            display_name: String::new(),
            lower: literal_integer(start)?,
            upper: literal_integer(end)?,
            step: step.map_or(Some(1), literal_integer)?,
        }],
    };
    Some(vec![i64::try_from(domain.scalar_count().ok()?).ok()?])
}

fn literal_integer(expression: &Expression) -> Option<i64> {
    match expression {
        Expression::Literal {
            value: Literal::Integer(value),
            ..
        } => Some(*value),
        Expression::Unary {
            op: rumoca_core::OpUnary::Minus,
            rhs,
            ..
        } => literal_integer(rhs)?.checked_neg(),
        Expression::Unary {
            op: rumoca_core::OpUnary::Plus,
            rhs,
            ..
        } => literal_integer(rhs),
        _ => None,
    }
}

fn literal_extents(expressions: &[Expression]) -> Option<Vec<i64>> {
    expressions
        .iter()
        .map(|expression| match expression {
            Expression::Literal {
                value: Literal::Integer(value),
                ..
            } if *value >= 0 => Some(*value),
            _ => None,
        })
        .collect()
}

fn binary_dimensions(op: &OpBinary, lhs: Vec<i64>, rhs: Vec<i64>) -> Option<Vec<i64>> {
    use OpBinary::*;
    match op {
        Add | Sub | And | Or if lhs == rhs => Some(lhs),
        AddElem | SubElem | MulElem | DivElem | ExpElem if lhs == rhs || rhs.is_empty() => {
            Some(lhs)
        }
        AddElem | SubElem | MulElem | DivElem | ExpElem if lhs.is_empty() => Some(rhs),
        Mul if lhs.is_empty() => Some(rhs),
        Mul | Div if rhs.is_empty() => Some(lhs),
        Eq | Neq | Lt | Le | Gt | Ge | Exp if lhs.is_empty() && rhs.is_empty() => Some(vec![]),
        _ => None,
    }
}

impl std::fmt::Display for SourceAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source attribute presentation refused: {self:?}")
    }
}

impl std::error::Error for SourceAttributeError {}

fn unselected_reference(reference: &Reference) -> bool {
    reference
        .component_ref()
        .is_none_or(|component| component.parts().iter().all(|part| part.subs.is_empty()))
}
