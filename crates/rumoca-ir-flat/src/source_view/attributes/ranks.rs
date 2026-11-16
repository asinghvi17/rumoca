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

    pub(super) fn minimum_rank(&self, expression: &Expression) -> Option<usize> {
        self.rank(expression)
            .or_else(|| self.array_rank_lower_bound(expression))
    }

    /// Positive lower bounds prove arrayness without inventing an exact rank.
    /// Each enclosing operation must preserve the guaranteed axes; a product
    /// of two possible vectors, for example, does not prove an array result.
    fn array_rank_lower_bound(&self, expression: &Expression) -> Option<usize> {
        let rank = match expression {
            Expression::FunctionCall { name, args, .. } => {
                self.call_rank_lower_bound(name, args)?
            }
            Expression::Array { elements, kind, .. } => {
                let element_rank = elements
                    .iter()
                    .map(|value| self.minimum_rank(value).unwrap_or(0))
                    .min()
                    .unwrap_or(0);
                if kind.concatenation_axis().is_some() {
                    element_rank.max(2)
                } else {
                    element_rank + 1
                }
            }
            Expression::ArrayComprehension { expr, indices, .. } => {
                indices.len() + self.minimum_rank(expr).unwrap_or(0)
            }
            Expression::BuiltinCall { function, args, .. } => {
                self.builtin_rank_lower_bound(*function, args)?
            }
            Expression::Index {
                base, subscripts, ..
            } => {
                // Unknown selectors remove at most one axis. A known vector
                // selector or colon retains the selected axis (MLS §10.6.9).
                let removed = subscripts
                    .iter()
                    .filter(|subscript| match subscript {
                        Subscript::Colon { .. } => false,
                        Subscript::Expr { expr, .. } => self.rank(expr) != Some(1),
                        Subscript::Index { .. } => true,
                    })
                    .count();
                self.minimum_rank(base)?.checked_sub(removed)?
            }
            Expression::Unary { rhs, .. } => self.minimum_rank(rhs)?,
            Expression::Binary { op, lhs, rhs, .. } => {
                binary_rank_lower_bound(op, self.minimum_rank(lhs)?, self.minimum_rank(rhs)?)?
            }
            Expression::If {
                branches,
                else_branch,
                ..
            } => {
                let mut rank = self.minimum_rank(else_branch)?;
                for (_, value) in branches {
                    rank = rank.min(self.minimum_rank(value)?);
                }
                rank
            }
            Expression::FieldAccess {
                base, field_def_id, ..
            } => self.minimum_rank(base)? + self.fields.get(field_def_id)?,
            _ => return None,
        };
        (rank > 0).then_some(rank)
    }

    fn call_rank_lower_bound(&self, name: &Reference, args: &[Expression]) -> Option<usize> {
        let function = self.functions.get(&name.resolved_function()?.instance_id)?;
        if args.len() > function.inputs.len()
            || function.inputs[args.len()..]
                .iter()
                .any(|input| input.default.is_none())
        {
            return None;
        }
        // MLS §12.4.6: every guaranteed excess input axis vectorizes the
        // result. Unknown argument ranks never imply zero excess axes.
        let vectorized = args
            .iter()
            .zip(&function.inputs)
            .filter_map(|(arg, input)| {
                self.minimum_rank(arg)?
                    .checked_sub(input.dimensions().len())
            })
            .max()
            .unwrap_or(0);
        Some(function.outputs.first()?.dimensions().len() + vectorized)
    }

    fn builtin_rank_lower_bound(
        &self,
        function: BuiltinFunction,
        args: &[Expression],
    ) -> Option<usize> {
        use BuiltinFunction::*;
        if let Some(value) = rank_preserving_argument(function, args) {
            return self.minimum_rank(value);
        }
        match (function, args) {
            (Fill, [value, sizes @ ..]) if !sizes.is_empty() => {
                Some(sizes.len() + self.minimum_rank(value).unwrap_or(0))
            }
            (Min | Max | Div | Mod | Rem | Atan2, [lhs, rhs]) => {
                let lhs = self.minimum_rank(lhs)?;
                let rhs = self.minimum_rank(rhs)?;
                (lhs > 0 && rhs > 0).then_some(lhs.max(rhs))
            }
            _ => None,
        }
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
        if let Some(value) = rank_preserving_argument(function, args) {
            return self.rank(value);
        }
        match (function, args) {
            (Fill, [value, sizes @ ..]) if !sizes.is_empty() => {
                Some(sizes.len() + self.rank(value)?)
            }
            (Zeros | Ones, sizes) if !sizes.is_empty() => Some(sizes.len()),
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

/// These builtins preserve operand rank, including all vectorization axes.
fn rank_preserving_argument(function: BuiltinFunction, args: &[Expression]) -> Option<&Expression> {
    use BuiltinFunction::*;
    match (function, args) {
        (
            Abs | Sign | Sqrt | Floor | Ceil | Sin | Cos | Tan | Asin | Acos | Atan | Sinh | Cosh
            | Tanh | Exp | Log | Log10 | Integer | NoEvent | Pre | Der | Transpose,
            [value],
        )
        | (Smooth, [_, value])
        | (Homotopy, [value, _]) => Some(value),
        _ => None,
    }
}

fn binary_rank_lower_bound(op: &OpBinary, lhs: usize, rhs: usize) -> Option<usize> {
    use OpBinary::*;
    match op {
        // MLS §10.6.2 and §10.6.12 require equal array shapes here. Either
        // operand's guaranteed axes therefore also belong to the result.
        Add | Sub | And | Or | AddElem | SubElem | MulElem | DivElem | ExpElem
            if lhs > 0 && rhs > 0 =>
        {
            Some(lhs.max(rhs))
        }
        // MLS §10.6.4 contracts two non-scalar operands over one axis each.
        // Two rank-one lower bounds cannot distinguish a dot product.
        Mul if lhs > 0 && rhs > 0 => (lhs + rhs).checked_sub(2),
        _ => binary_rank(op, lhs, rhs),
    }
}
