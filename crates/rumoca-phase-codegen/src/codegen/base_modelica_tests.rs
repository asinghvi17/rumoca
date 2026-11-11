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
