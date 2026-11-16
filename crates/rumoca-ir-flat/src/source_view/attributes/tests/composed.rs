use super::*;
use rumoca_core::{ComprehensionIndex, IndexBinderProvenance};

fn comprehension(body: Expression) -> Expression {
    Expression::ArrayComprehension {
        expr: Box::new(body),
        filter: None,
        indices: vec![ComprehensionIndex {
            binder: IndexBinderProvenance::Generated,
            name: "i".into(),
            range: Expression::Range {
                start: Box::new(literal(1.0)),
                step: None,
                end: Box::new(literal(3.0)),
                span: span(),
            },
        }],
        span: span(),
    }
}

fn unknown_comprehension() -> Expression {
    comprehension(reference("i", None))
}

fn binary(op: OpBinary, lhs: Expression, rhs: Expression) -> Expression {
    Expression::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
        span: span(),
    }
}

fn builtin(function: BuiltinFunction, args: Vec<Expression>) -> Expression {
    Expression::BuiltinCall {
        function,
        args,
        span: span(),
    }
}

fn assert_bound(expression: Expression, reference_dims: Vec<i64>, bound: usize) {
    let model = model(vec![3], expression, reference_dims);
    let catalog = model.source_attributes().unwrap();
    let value = start(&catalog);
    assert_eq!(
        catalog.rank(value.expression()),
        None,
        "a lower bound is not an exact rank"
    );
    assert_eq!(value.minimum_rank(), bound);
    assert!(!value.each());
}

#[test]
fn binary_operations_preserve_only_guaranteed_array_axes() {
    for op in [
        OpBinary::Mul,
        OpBinary::Div,
        OpBinary::Exp,
        OpBinary::AddElem,
        OpBinary::SubElem,
        OpBinary::MulElem,
        OpBinary::DivElem,
        OpBinary::ExpElem,
    ] {
        assert_bound(binary(op, unknown_comprehension(), literal(2.0)), vec![], 1);
    }
    assert_bound(
        binary(OpBinary::Mul, literal(2.0), unknown_comprehension()),
        vec![],
        1,
    );
    for op in [
        OpBinary::Add,
        OpBinary::Sub,
        OpBinary::AddElem,
        OpBinary::SubElem,
        OpBinary::MulElem,
        OpBinary::DivElem,
        OpBinary::ExpElem,
        OpBinary::And,
        OpBinary::Or,
    ] {
        assert_bound(
            binary(
                op,
                unknown_comprehension(),
                reference("p", Some(InstanceId::new(2))),
            ),
            vec![3],
            1,
        );
    }
    // The same unknown element body can be array-valued. A rank-two peer
    // proves more axes for equal-shape operators without fixing the body rank.
    assert_bound(
        binary(
            OpBinary::Add,
            unknown_comprehension(),
            reference("p", Some(InstanceId::new(2))),
        ),
        vec![3, 3],
        2,
    );
    assert_bound(
        binary(
            OpBinary::Mul,
            unknown_comprehension(),
            reference("p", Some(InstanceId::new(2))),
        ),
        vec![3, 3],
        1,
    );
}

#[test]
fn builtin_and_conditional_bounds_compose() {
    use BuiltinFunction::*;
    for function in [
        Abs, Sign, Sqrt, Floor, Ceil, Sin, Cos, Tan, Asin, Acos, Atan, Sinh, Cosh, Tanh, Exp, Log,
        Log10, Integer, NoEvent, Pre, Der, Transpose,
    ] {
        assert_bound(builtin(function, vec![unknown_comprehension()]), vec![], 1);
    }
    for function in [Smooth, Homotopy, Min, Max, Div, Mod, Rem, Atan2] {
        let args = if function == Smooth {
            vec![literal(1.0), unknown_comprehension()]
        } else {
            vec![unknown_comprehension(), unknown_comprehension()]
        };
        assert_bound(builtin(function, args), vec![], 1);
    }
    let expression = Expression::If {
        branches: vec![(literal(1.0), comprehension(unknown_comprehension()))],
        else_branch: Box::new(unknown_comprehension()),
        span: span(),
    };
    assert_bound(expression, vec![], 1);
}

#[test]
fn selections_retain_nested_comprehension_and_fill_axes() {
    for base in [
        comprehension(unknown_comprehension()),
        builtin(
            BuiltinFunction::Fill,
            vec![unknown_comprehension(), literal(2.0)],
        ),
    ] {
        assert_bound(
            Expression::Index {
                base: Box::new(base),
                subscripts: vec![Subscript::index(1, span())],
                span: span(),
            },
            vec![],
            1,
        );
    }
}

#[test]
fn scalar_returning_calls_can_prove_vectorization_from_a_lower_bound() {
    for (input_dims, body, expected) in [
        (vec![], unknown_comprehension(), 1),
        (vec![3], comprehension(unknown_comprehension()), 1),
    ] {
        let mut function = Function::new("f", span());
        function.inputs.push(FunctionParam::new(
            "u",
            "Real",
            EffectiveType::new(TypeId::new(10), TypeId::new(10), input_dims).unwrap(),
            span(),
        ));
        function.outputs.push(FunctionParam::new(
            "y",
            "Real",
            EffectiveType::new(TypeId::new(10), TypeId::new(10), vec![]).unwrap(),
            span(),
        ));
        let mut model = model(vec![3], literal(1.0), vec![]);
        model.add_function(function);
        let instance_id = model.functions[&VarName::new("f")].instance_id.unwrap();
        let expression = Expression::FunctionCall {
            name: Reference::new("f").with_resolved_function(ResolvedFunctionReference {
                instance_id,
                base_part_count: 1,
                transitively_non_replaceable: false,
            }),
            args: vec![body],
            is_constructor: false,
            span: span(),
        };
        assert_eq!(model.source_attributes().unwrap().rank(&expression), None);
        model.variables.get_mut(&VarName::new("x")).unwrap().start = Some(expression);
        let catalog = model.source_attributes().unwrap();
        assert_eq!(start(&catalog).minimum_rank(), expected);
        assert!(!start(&catalog).each());
    }
}

#[test]
fn possible_scalar_results_still_refuse_unknown_shape() {
    let vector = reference("p", Some(InstanceId::new(2)));
    for expression in [
        binary(OpBinary::Mul, unknown_comprehension(), vector),
        Expression::Index {
            base: Box::new(unknown_comprehension()),
            subscripts: vec![Subscript::index(1, span())],
            span: span(),
        },
        Expression::If {
            branches: vec![(literal(1.0), unknown_comprehension())],
            else_branch: Box::new(literal(1.0)),
            span: span(),
        },
        builtin(BuiltinFunction::Sin, vec![reference("unknown", None)]),
    ] {
        let model = model(vec![3], expression, vec![3]);
        assert!(matches!(
            model
                .source_attributes()
                .unwrap()
                .attribute(InstanceId::new(1), SourceAttributeKind::Start),
            Err(SourceAttributeError::UnknownShape { .. })
        ));
    }
}

#[test]
fn field_projection_retains_guaranteed_container_axes() {
    let expression = Expression::FieldAccess {
        base: Box::new(unknown_comprehension()),
        field: "q".into(),
        field_def_id: DefId::new(23),
        span: span(),
    };
    let mut model = model(vec![3], expression, vec![]);
    model.record_types.insert(
        DefId::new(22),
        crate::RecordType {
            name: "R".into(),
            fields: vec![crate::RecordField {
                name: "q".into(),
                def_id: DefId::new(23),
                dims: vec![2],
            }],
        },
    );
    let catalog = model.source_attributes().unwrap();
    let value = start(&catalog);
    assert_eq!(catalog.rank(value.expression()), None);
    assert_eq!(value.minimum_rank(), 2);
    assert!(!value.each());
}

#[test]
fn array_constructors_retain_nested_axes_after_selection() {
    for (kind, rank) in [
        (ArrayConstructor::Array, 4),
        (ArrayConstructor::Horizontal, 3),
        (ArrayConstructor::Vertical, 3),
    ] {
        let expression = Expression::Array {
            elements: vec![comprehension(comprehension(unknown_comprehension()))],
            kind,
            span: span(),
        };
        assert_bound(
            Expression::Index {
                base: Box::new(expression),
                subscripts: vec![Subscript::index(1, span()); rank - 1],
                span: span(),
            },
            vec![],
            1,
        );
    }
}
