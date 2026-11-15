//! Value-independent ranks over finalized Flat occurrence and callable facts.

use super::SourceAttributeCatalog;
use rumoca_core::{BuiltinFunction, ComponentPath, Expression, OpBinary, Reference, Subscript};

impl SourceAttributeCatalog<'_> {
    pub(super) fn rank(&self, expression: &Expression) -> Option<usize> {
        match expression {
            Expression::Literal { .. } => Some(0),
            Expression::VarRef {
                name, subscripts, ..
            } => self.selected_rank(self.reference_rank(name)?, subscripts),
            Expression::Array { elements, kind, .. } => {
                let ranks = elements
                    .iter()
                    .map(|value| self.rank(value))
                    .collect::<Option<Vec<_>>>()?;
                if kind.concatenation_axis().is_some() {
                    Some(ranks.into_iter().max()?.max(2))
                } else {
                    let rank = ranks.first().copied().unwrap_or(0);
                    ranks.iter().all(|other| *other == rank).then_some(rank + 1)
                }
            }
            Expression::FunctionCall { name, args, .. } => self.call_rank(name, args),
            Expression::Unary { rhs, .. } => self.rank(rhs),
            Expression::Binary { op, lhs, rhs, .. } => {
                binary_rank(op, self.rank(lhs)?, self.rank(rhs)?)
            }
            Expression::If {
                branches,
                else_branch,
                ..
            } => {
                let rank = self.rank(else_branch)?;
                branches
                    .iter()
                    .all(|(_, value)| self.rank(value) == Some(rank))
                    .then_some(rank)
            }
            Expression::BuiltinCall { function, args, .. } => self.builtin_rank(*function, args),
            Expression::Index {
                base, subscripts, ..
            } => self.selected_rank(self.rank(base)?, subscripts),
            Expression::Range { .. } => Some(1),
            Expression::FieldAccess {
                base, field_def_id, ..
            } => Some(self.rank(base)? + self.fields.get(field_def_id)?),
            Expression::ArrayComprehension { expr, indices, .. } => {
                Some(indices.len() + self.rank(expr)?)
            }
            _ => None,
        }
    }

    /// Positive lower bounds prove arrayness without inventing an exact rank.
    /// In particular, vectorization can add axes to an array-returning call.
    pub(super) fn array_rank_lower_bound(&self, expression: &Expression) -> Option<usize> {
        let rank = match expression {
            Expression::FunctionCall { name, args, .. } => {
                let function = self.functions.get(&name.resolved_function()?.instance_id)?;
                if args.len() > function.inputs.len()
                    || function.inputs[args.len()..]
                        .iter()
                        .any(|input| input.default.is_none())
                {
                    return None;
                }
                function.outputs.first()?.dimensions().len()
            }
            Expression::Array { kind, .. } => {
                if kind.concatenation_axis().is_some() {
                    2
                } else {
                    1
                }
            }
            Expression::BuiltinCall {
                function: BuiltinFunction::Fill,
                args,
                ..
            } if args.len() > 1 => args.len() - 1,
            Expression::Index {
                base, subscripts, ..
            } => {
                // Scalar selectors remove axes; colons and known vector
                // selectors retain them. Unknown selectors remove at most one.
                let removed = subscripts
                    .iter()
                    .filter(|subscript| match subscript {
                        Subscript::Colon { .. } => false,
                        Subscript::Expr { expr, .. } => self.rank(expr) != Some(1),
                        Subscript::Index { .. } => true,
                    })
                    .count();
                self.array_rank_lower_bound(base)?.checked_sub(removed)?
            }
            Expression::Unary { rhs, .. } => self.array_rank_lower_bound(rhs)?,
            _ => return None,
        };
        (rank > 0).then_some(rank)
    }

    fn reference_rank(&self, name: &Reference) -> Option<usize> {
        // MLS §4.9.5: a literal belongs to the enum declaration Resolve
        // selected, independently of the enclosing instance's scope identity.
        if let Some(declaration) = name
            .target_def_id()
            .and_then(|id| self.enumerations.get(&id))
        {
            let reference = name.component_ref()?;
            return (reference.parts().iter().all(|part| part.subs.is_empty())
                && declaration
                    .literals
                    .contains(&reference.parts().last()?.ident))
            .then_some(0);
        }
        let (path, rank) = self.occurrences.get(&name.instance_id()?)?;
        if ComponentPath::from_reference(name).var_name() == *path {
            return Some(*rank);
        }
        let component = name.component_ref()?;
        let unselected =
            ComponentPath::from_parts(component.parts().iter().map(|part| part.ident.clone()));
        if unselected.var_name() != *path {
            return None;
        }
        // Embedded selections belong to their respective component axes. Flat
        // retains the total occurrence rank, and scalar selectors remove axes.
        let mut rank = *rank;
        for part in component.parts() {
            rank = self.selected_rank(rank, &part.subs)?;
        }
        Some(rank)
    }

    fn selected_rank(&self, rank: usize, subscripts: &[Subscript]) -> Option<usize> {
        if subscripts.len() > rank {
            return None;
        }
        let mut selected = rank - subscripts.len();
        for subscript in subscripts {
            selected += match subscript {
                Subscript::Colon { .. } => 1,
                Subscript::Index { .. } => 0,
                Subscript::Expr { expr, .. } => match self.rank(expr)? {
                    rank @ 0..=1 => rank,
                    _ => return None,
                },
            };
        }
        Some(selected)
    }

    fn call_rank(&self, name: &Reference, args: &[Expression]) -> Option<usize> {
        let function = self.functions.get(&name.resolved_function()?.instance_id)?;
        if args.len() > function.inputs.len()
            || function.inputs[args.len()..]
                .iter()
                .any(|input| input.default.is_none())
        {
            return None;
        }
        // MLS §12.4.6: axes in excess of a formal input's rank vectorize the
        // call. Colon extents and default values do not change declared rank.
        let mut vectorized = None;
        for (arg, input) in args.iter().zip(&function.inputs) {
            let extra = self.rank(arg)?.checked_sub(input.dimensions().len())?;
            if extra > 0 && vectorized.is_some_and(|rank| rank != extra) {
                return None;
            }
            if extra > 0 {
                vectorized = Some(extra);
            }
        }
        Some(function.outputs.first()?.dimensions().len() + vectorized.unwrap_or(0))
    }

    fn builtin_rank(&self, function: BuiltinFunction, args: &[Expression]) -> Option<usize> {
        use BuiltinFunction::*;
        match (function, args) {
            (Fill, [value, sizes @ ..]) if !sizes.is_empty() => {
                Some(sizes.len() + self.rank(value)?)
            }
            (Zeros | Ones, sizes) if !sizes.is_empty() => Some(sizes.len()),
            (
                Abs | Sign | Sqrt | Floor | Ceil | Sin | Cos | Tan | Asin | Acos | Atan | Sinh
                | Cosh | Tanh | Exp | Log | Log10 | Integer | NoEvent | Pre | Der | Transpose,
                [value],
            ) => self.rank(value),
            (Smooth, [_, value]) => self.rank(value),
            (Homotopy, [value, _]) => self.rank(value),
            (Sum | Product | Min | Max | Ndims | Scalar, [_]) | (Size, [_, _]) => Some(0),
            (Size | Vector, [_]) | (Linspace, [_, _, _]) => Some(1),
            (Matrix | Identity | Diagonal, [_]) | (OuterProduct, [_, _]) => Some(2),
            (Min | Max | Div | Mod | Rem | Atan2, [lhs, rhs]) => {
                let rank = self.rank(lhs)?;
                (self.rank(rhs)? == rank).then_some(rank)
            }
            _ => None,
        }
    }
}

fn binary_rank(op: &OpBinary, lhs: usize, rhs: usize) -> Option<usize> {
    use OpBinary::*;
    match op {
        Add | Sub | And | Or if lhs == rhs => Some(lhs),
        AddElem | SubElem | MulElem | DivElem | ExpElem if lhs == rhs || rhs == 0 || lhs == 0 => {
            Some(lhs.max(rhs))
        }
        Mul if lhs == 0 => Some(rhs),
        Mul | Div if rhs == 0 => Some(lhs),
        Mul if (1..=2).contains(&lhs) && (1..=2).contains(&rhs) => Some(lhs + rhs - 2),
        Exp if rhs == 0 => Some(lhs),
        Eq | Neq | Lt | Le | Gt | Ge if lhs == 0 && rhs == 0 => Some(0),
        _ => None,
    }
}
