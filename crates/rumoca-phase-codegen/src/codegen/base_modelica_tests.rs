//! Base Modelica template helpers: quoting, element enumeration and the
//! scalarizing equation expansion.

use super::*;

fn render(template: &str, context: Value) -> Result<String, minijinja::Error> {
    let mut env = create_environment();
    env.add_template("inline", template)?;
    env.get_template("inline")?.render(context)
}

#[test]
fn quote_identifier_escapes_and_keeps_already_quoted_names() {
    assert_eq!(quote_identifier("R1.p.v"), "'R1.p.v'");
    assert_eq!(quote_identifier("x[2,1]"), "'x[2,1]'");
    assert_eq!(quote_identifier("it's"), r"'it\'s'");
    assert_eq!(quote_identifier(r"a\b"), r"'a\\b'");
    assert_eq!(quote_identifier("'x'"), "'x'");
}

#[test]
fn scalar_indices_enumerate_elements_row_major() {
    let rendered = render(
        "{{ scalar_indices([2, 2]) | join(';') }}|{{ scalar_indices([3]) | join(';') }}",
        minijinja::context! {},
    )
    .expect("scalar_indices renders");
    assert_eq!(rendered, "1,1;1,2;2,1;2,2|1;2;3");
}

/// Declared names quote whether or not they are dotted; `time` and an
/// undeclared loop index stay bare; a declared function is quoted at its
/// call; a negative literal is parenthesized.
#[test]
fn base_modelica_expressions_quote_declared_names() {
    let call = rumoca_core::Expression::FunctionCall {
        name: rumoca_core::Reference::generated("Pkg.f"),
        args: vec![
            rumoca_core::Expression::VarRef {
                name: rumoca_core::Reference::generated("x"),
                subscripts: Vec::new(),
                span: rumoca_core::Span::DUMMY,
            },
            rumoca_core::Expression::VarRef {
                name: rumoca_core::Reference::generated("time"),
                subscripts: Vec::new(),
                span: rumoca_core::Span::DUMMY,
            },
            rumoca_core::Expression::VarRef {
                name: rumoca_core::Reference::generated("i"),
                subscripts: Vec::new(),
                span: rumoca_core::Span::DUMMY,
            },
            rumoca_core::Expression::Literal {
                value: rumoca_core::Literal::Real(-0.5),
                span: rumoca_core::Span::DUMMY,
            },
        ],
        is_constructor: false,
        span: rumoca_core::Span::DUMMY,
    };
    let rendered = render(
        r#"{{ render_expr(e, {"base_modelica": true, "sanitize_dots": false, "quote_names": ["x"], "quote_functions": ["Pkg.f"]}) }}"#,
        minijinja::context! { e => Value::from_serialize(&call) },
    )
    .expect("expression renders");
    assert_eq!(rendered, "'Pkg.f'('x', time, i, (-0.5))");
}

/// A generated comprehension binder (`$parentIndex1`, from an `each`
/// modifier on an array of components) is not a legal IDENT, so the binder
/// is quoted like its references in the body (MSL MultiBody `world`
/// arrow and label cylinders).
#[test]
fn base_modelica_quotes_generated_comprehension_binders() {
    let index = rumoca_core::Expression::VarRef {
        name: rumoca_core::Reference::generated("$parentIndex1"),
        subscripts: Vec::new(),
        span: rumoca_core::Span::DUMMY,
    };
    let integer = |value| rumoca_core::Expression::Literal {
        value: rumoca_core::Literal::Integer(value),
        span: rumoca_core::Span::DUMMY,
    };
    let comprehension = rumoca_core::Expression::ArrayComprehension {
        expr: Box::new(index),
        indices: vec![rumoca_core::ComprehensionIndex {
            binder: rumoca_core::IndexBinderProvenance::Generated,
            name: "$parentIndex1".to_string(),
            range: rumoca_core::Expression::Range {
                start: Box::new(integer(1)),
                step: None,
                end: Box::new(integer(2)),
                span: rumoca_core::Span::DUMMY,
            },
        }],
        filter: None,
        span: rumoca_core::Span::DUMMY,
    };
    let rendered = render(
        r#"{{ render_expr(e, {"base_modelica": true, "sanitize_dots": false, "quote_names": []}) }}"#,
        minijinja::context! { e => Value::from_serialize(&comprehension) },
    )
    .expect("comprehension renders");
    assert_eq!(rendered, "{'$parentIndex1' for '$parentIndex1' in 1:2}");
}

/// MCP-0031 subscripts only component references and parenthesized
/// expressions, so a slice of `fill` is the `fill` it denotes: a literal
/// range keeps its length, `:` the dimension, a literal index drops it (MSL
/// Fluid media: `Xi(start = fill(1 / nX, nX)[1:0])`).
#[test]
fn base_modelica_folds_a_slice_of_fill() {
    let integer = |value| rumoca_core::Expression::Literal {
        value: rumoca_core::Literal::Integer(value),
        span: rumoca_core::Span::DUMMY,
    };
    let range = |start, end| rumoca_core::Subscript::Expr {
        expr: Box::new(rumoca_core::Expression::Range {
            start: Box::new(integer(start)),
            step: None,
            end: Box::new(integer(end)),
            span: rumoca_core::Span::DUMMY,
        }),
        span: rumoca_core::Span::DUMMY,
    };
    let slice = |subscripts| rumoca_core::Expression::Index {
        base: Box::new(rumoca_core::Expression::BuiltinCall {
            function: rumoca_core::BuiltinFunction::Fill,
            args: vec![integer(7), integer(5), integer(3)],
            span: rumoca_core::Span::DUMMY,
        }),
        subscripts,
        span: rumoca_core::Span::DUMMY,
    };
    let cases = [
        (
            slice(vec![
                range(1, 0),
                rumoca_core::Subscript::Colon {
                    span: rumoca_core::Span::DUMMY,
                },
            ]),
            "fill(7, 0, 3)",
        ),
        (
            slice(vec![
                range(2, 4),
                rumoca_core::Subscript::index(1, rumoca_core::Span::DUMMY),
            ]),
            "fill(7, 3)",
        ),
        (
            slice(vec![
                rumoca_core::Subscript::index(1, rumoca_core::Span::DUMMY),
                rumoca_core::Subscript::index(2, rumoca_core::Span::DUMMY),
            ]),
            "7",
        ),
        (
            slice(vec![rumoca_core::Subscript::index(
                2,
                rumoca_core::Span::DUMMY,
            )]),
            "fill(7, 3)",
        ),
    ];
    for (expr, expected) in cases {
        let rendered = render(
            r#"{{ render_expr(e, {"base_modelica": true, "modelica_builtins": true, "sanitize_dots": false, "quote_names": ["n"]}) }}"#,
            minijinja::context! { e => Value::from_serialize(&expr) },
        )
        .expect("slice renders");
        assert_eq!(rendered, expected);
    }
}

#[test]
fn fill_slice_cardinality_handles_direction_steps_and_overflow() {
    let integer = |value| serde_json::json!({"Literal": {"value": {"Integer": value}}});
    let slice = |start, step, end| {
        Value::from_serialize(serde_json::json!({
            "base": {"BuiltinCall": {"function": "Fill", "args": [integer(7), integer(5)]}},
            "subscripts": [{"Expr": {"expr": {"Range": {
                "start": integer(start), "step": integer(step), "end": integer(end)
            }}}}]
        }))
    };
    for (start, step, end, count) in [
        (2, 2, 1, 0),
        (1, -2, 2, 0),
        (1, 2, 5, 3),
        (5, -2, 1, 3),
        (1, 2, 6, 3),
        (3, -2, 0, 2),
    ] {
        let folded = base_modelica::fold_fill_slice(&slice(start, step, end))
            .expect("checked representable range");
        let json = serde_json::to_value(folded).unwrap();
        assert_eq!(json["BuiltinCall"]["args"][1], integer(count));
    }
    for bounds in [
        (1, 0, 3),
        (i64::MIN, 1, i64::MAX),
        (i64::MIN, i64::MAX, i64::MAX),
    ] {
        assert!(base_modelica::fold_fill_slice(&slice(bounds.0, bounds.1, bounds.2)).is_none());
    }
}

#[test]
fn fill_slice_folding_requires_proven_in_bounds_selections() {
    use rumoca_core::{BuiltinFunction, Expression, Literal, OpUnary, Span, Subscript};
    let integer = |value| Expression::Literal {
        value: Literal::Integer(value),
        span: Span::DUMMY,
    };
    let negative = Expression::Unary {
        op: OpUnary::Minus,
        rhs: Box::new(integer(1)),
        span: Span::DUMMY,
    };
    let range = |start, step, end| Subscript::Expr {
        expr: Box::new(Expression::Range {
            start: Box::new(start),
            step: Some(Box::new(integer(step))),
            end: Box::new(integer(end)),
            span: Span::DUMMY,
        }),
        span: Span::DUMMY,
    };
    for subscript in [
        Subscript::index(-1, Span::DUMMY),
        Subscript::index(0, Span::DUMMY),
        Subscript::index(4, Span::DUMMY),
        Subscript::Expr {
            expr: Box::new(negative.clone()),
            span: Span::DUMMY,
        },
        range(negative, 1, 1),
        range(integer(2), 1, 5),
        range(integer(3), -1, 0),
    ] {
        let slice = Expression::Index {
            base: Box::new(Expression::BuiltinCall {
                function: BuiltinFunction::Fill,
                args: vec![integer(7), integer(3)],
                span: Span::DUMMY,
            }),
            subscripts: vec![subscript],
            span: Span::DUMMY,
        };
        let serialized = serde_json::to_value(slice).unwrap();
        assert!(
            base_modelica::fold_fill_slice(&Value::from_serialize(&serialized["Index"])).is_none(),
            "{serialized}"
        );
    }
}

#[test]
fn fill_slice_with_unknown_extent_retains_parenthesized_selection() {
    use rumoca_core::{BuiltinFunction, Expression, Literal, Reference, Span, Subscript};
    let expression = Expression::Index {
        base: Box::new(Expression::BuiltinCall {
            function: BuiltinFunction::Fill,
            args: vec![
                Expression::Literal {
                    value: Literal::Integer(7),
                    span: Span::DUMMY,
                },
                Expression::VarRef {
                    name: Reference::generated("n"),
                    subscripts: vec![],
                    span: Span::DUMMY,
                },
            ],
            span: Span::DUMMY,
        }),
        subscripts: vec![Subscript::index(1, Span::DUMMY)],
        span: Span::DUMMY,
    };
    let rendered = render(
        r#"{{ render_expr(e, {"base_modelica": true, "modelica_builtins": true, "one_based_index": true, "sanitize_dots": false, "quote_names": ["n"]}) }}"#,
        minijinja::context! { e => Value::from_serialize(expression) },
    ).unwrap();
    assert_eq!(rendered, "(fill(7, 'n'))[1]");
}
