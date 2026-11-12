//! Vectorized calls of record-valued functions (MLS 3.7 §12.4.6).
//!
//! A function with scalar inputs applied to vector arguments is the vector
//! of its calls on the arguments' elements. MSL writes this for operator
//! records, e.g. `y = Modelica.ComplexMath.fromPolar(fill(V, m), phi)` in
//! `Magnetic.QuasiStatic.FundamentalWave.Utilities.VfController`, whose
//! `fromPolar` takes a scalar length and angle and returns one `Complex`.
//! Spelled out element by element, each call is an ordinary record operand,
//! so operators and record equations over the result resolve like those over
//! any other record vector.

use super::*;

/// The shape of one call argument, as far as vectorization needs it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ArgumentShape {
    Scalar,
    /// A vector; its extent when it is known at translation.
    Vector(Option<i64>),
}

impl Resolver<'_, '_, '_> {
    /// `f(a1, ..., an)` with some `ai` a vector, as the array of calls of `f`
    /// on the arguments' elements, when every input of `f` is scalar and its
    /// result is one operator record.
    pub(super) fn vectorized_record_call(
        &mut self,
        name: &rumoca_core::Reference,
        args: &[Expression],
        span: Span,
    ) -> Option<Expression> {
        if args.iter().any(is_named_argument) {
            return None;
        }
        let OperandType::Record(_) = self.call_type(name) else {
            return None;
        };
        let def_id = name.component_ref()?.target_def_id();
        let scalar_inputs = self
            .catalog
            .classes
            .get(def_id)?
            .components
            .values()
            .filter(|component| matches!(component.causality, rumoca_core::Causality::Input(_)))
            .all(|input| input.shape_expr.is_empty() && input.shape.is_empty());
        if !scalar_inputs {
            return None;
        }
        let shapes = args
            .iter()
            .map(|arg| self.argument_shape(arg, span))
            .collect::<Option<Vec<_>>>()?;
        let extent = vector_extent(&shapes)?;
        let elements = (1..=extent)
            .map(|index| Expression::FunctionCall {
                name: name.clone(),
                args: args
                    .iter()
                    .zip(&shapes)
                    .map(|(arg, shape)| match shape {
                        ArgumentShape::Scalar => arg.clone(),
                        ArgumentShape::Vector(_) => self.vector_element(arg, index, span),
                    })
                    .collect(),
                is_constructor: false,
                span,
            })
            .collect();
        Some(Expression::Array {
            elements,
            kind: rumoca_core::ArrayConstructor::Array,
            span,
        })
    }

    /// The shape of `arg`, or `None` when it is neither a scalar nor a vector
    /// this pass can see.
    fn argument_shape(&mut self, arg: &Expression, span: Span) -> Option<ArgumentShape> {
        match arg {
            Expression::Literal { .. } => Some(ArgumentShape::Scalar),
            Expression::VarRef {
                name, subscripts, ..
            } if subscripts.is_empty() => match self.operand_type(arg) {
                OperandType::Record(_) | OperandType::Numeric { .. } => Some(ArgumentShape::Scalar),
                OperandType::RecordVector(_) => Some(ArgumentShape::Vector(
                    self.record_elements(arg, span)
                        .and_then(|elements| i64::try_from(elements.len()).ok()),
                )),
                OperandType::Other => self
                    .scope
                    .vector_extent(name.as_str())
                    .map(|extent| ArgumentShape::Vector(Some(extent))),
            },
            Expression::Array {
                elements,
                kind: rumoca_core::ArrayConstructor::Array,
                ..
            } => Some(ArgumentShape::Vector(i64::try_from(elements.len()).ok())),
            Expression::BuiltinCall {
                function: rumoca_core::BuiltinFunction::Fill,
                args,
                ..
            } => match args.as_slice() {
                [_, extent] => Some(ArgumentShape::Vector(self.integer_value(extent))),
                _ => None,
            },
            Expression::BuiltinCall { function, args, .. } if scalar_real_builtin(*function) => {
                let shapes = args
                    .iter()
                    .map(|arg| self.argument_shape(arg, span))
                    .collect::<Option<Vec<_>>>()?;
                Some(combined_shape(&shapes))
            }
            Expression::Unary {
                op: OpUnary::Minus | OpUnary::Plus,
                rhs,
                ..
            } => self.argument_shape(rhs, span),
            Expression::Binary { op, lhs, rhs, .. } if !op.is_relational() => {
                let shapes = [
                    self.argument_shape(lhs, span)?,
                    self.argument_shape(rhs, span)?,
                ];
                // A product or quotient of two vectors is not element-wise.
                let both_vectors = shapes
                    .iter()
                    .all(|shape| matches!(shape, ArgumentShape::Vector(_)));
                (!both_vectors || matches!(op, OpBinary::Add | OpBinary::Sub) || is_elementwise(op))
                    .then(|| combined_shape(&shapes))
            }
            Expression::FunctionCall {
                name,
                is_constructor: false,
                ..
            } if self.call_output_rank(name) == Some(1) => Some(ArgumentShape::Vector(None)),
            _ => match self.operand_type(arg) {
                OperandType::Record(_) | OperandType::Numeric { .. } => Some(ArgumentShape::Scalar),
                _ => None,
            },
        }
    }

    /// Element `index` of the vector `arg`, distributed over the element-wise
    /// operations that build it so that each call reads plain elements.
    fn vector_element(&mut self, arg: &Expression, index: i64, span: Span) -> Expression {
        let element =
            |this: &mut Self, operand: &Expression| match this.argument_shape(operand, span) {
                Some(ArgumentShape::Vector(_)) => this.vector_element(operand, index, span),
                _ => operand.clone(),
            };
        match arg {
            Expression::Array { elements, .. } => usize::try_from(index - 1)
                .ok()
                .and_then(|position| elements.get(position).cloned())
                .unwrap_or_else(|| indexed(arg, index, span)),
            Expression::BuiltinCall {
                function: rumoca_core::BuiltinFunction::Fill,
                args,
                ..
            } => args[0].clone(),
            Expression::BuiltinCall {
                function,
                args,
                span,
            } if scalar_real_builtin(*function) => Expression::BuiltinCall {
                function: *function,
                args: args.iter().map(|arg| element(self, arg)).collect(),
                span: *span,
            },
            Expression::Unary { op, rhs, span } => unary(op, element(self, rhs), *span),
            Expression::Binary { op, lhs, rhs, span } => {
                let (lhs, rhs) = (element(self, lhs), element(self, rhs));
                binary(&scalar_operator(op), lhs, rhs, *span)
            }
            Expression::VarRef { .. } => self
                .record_elements(arg, span)
                .and_then(|elements| elements.into_iter().nth(usize::try_from(index - 1).ok()?))
                .unwrap_or_else(|| indexed(arg, index, span)),
            _ => indexed(arg, index, span),
        }
    }

    /// The rank of the single result of the function `name` calls.
    fn call_output_rank(&self, name: &rumoca_core::Reference) -> Option<usize> {
        let def_id = name.component_ref()?.target_def_id();
        let mut outputs = self
            .catalog
            .classes
            .get(def_id)?
            .components
            .values()
            .filter(|component| matches!(component.causality, rumoca_core::Causality::Output(_)));
        let output = outputs.next()?;
        outputs
            .next()
            .is_none()
            .then(|| output.shape_expr.len().max(output.shape.len()))
    }

    /// The translation-time value of an Integer extent expression.
    fn integer_value(&self, expression: &Expression) -> Option<i64> {
        match expression {
            Expression::Literal {
                value: rumoca_core::Literal::Integer(value),
                ..
            } => Some(*value),
            Expression::VarRef {
                name, subscripts, ..
            } if subscripts.is_empty() => self.scope.integer_value(name.as_str()),
            _ => None,
        }
    }
}

/// The common extent of the vector arguments, which MLS §12.4.6 requires to
/// agree, or `None` when there is no vector argument or no known extent.
fn vector_extent(shapes: &[ArgumentShape]) -> Option<i64> {
    let mut known = shapes.iter().filter_map(|shape| match shape {
        ArgumentShape::Vector(extent) => *extent,
        ArgumentShape::Scalar => None,
    });
    let extent = known.next()?;
    known.all(|other| other == extent).then_some(extent)
}

fn combined_shape(shapes: &[ArgumentShape]) -> ArgumentShape {
    if shapes
        .iter()
        .all(|shape| matches!(shape, ArgumentShape::Scalar))
    {
        return ArgumentShape::Scalar;
    }
    ArgumentShape::Vector(vector_extent(shapes))
}

fn is_elementwise(op: &OpBinary) -> bool {
    matches!(
        op,
        OpBinary::AddElem | OpBinary::SubElem | OpBinary::MulElem | OpBinary::DivElem
    )
}

fn scalar_operator(op: &OpBinary) -> OpBinary {
    match op {
        OpBinary::AddElem => OpBinary::Add,
        OpBinary::SubElem => OpBinary::Sub,
        OpBinary::MulElem => OpBinary::Mul,
        OpBinary::DivElem => OpBinary::Div,
        other => other.clone(),
    }
}

fn indexed(base: &Expression, index: i64, span: Span) -> Expression {
    Expression::Index {
        base: Box::new(base.clone()),
        subscripts: vec![rumoca_core::Subscript::index(index, span)],
        span,
    }
}
