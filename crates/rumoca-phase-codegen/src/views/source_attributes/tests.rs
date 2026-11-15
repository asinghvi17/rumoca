use super::*;
use rumoca_core::{
    EffectiveType, Expression, InstanceId, Literal, Reference, Span, TypeId, VarName,
};
use rumoca_ir_flat::Variable;

#[test]
fn an_unknown_unqueried_start_does_not_refuse_min_in_the_template_view() {
    let mut model = Model::default();
    let type_id = TypeId::new(0);
    model.effective_types.insert(
        type_id,
        EffectiveType::new(type_id, type_id, vec![2]).unwrap(),
    );
    let name = VarName::new("x");
    model.variables.insert(
        name.clone(),
        Variable {
            name,
            instance_id: InstanceId::new(1),
            type_id,
            dims: vec![2],
            start: Some(Expression::FunctionCall {
                name: Reference::new("opaque"),
                args: vec![],
                is_constructor: false,
                span: Span::DUMMY,
            }),
            min: Some(Expression::Literal {
                value: Literal::Real(0.0),
                span: Span::DUMMY,
            }),
            ..Variable::empty_with_span(Span::DUMMY)
        },
    );
    let mut environment = minijinja::Environment::new();
    environment.add_global("attribute_rank", context_value(&model));
    assert_eq!(
        environment
            .render_str("{{ attribute_rank(1, 'min').each }}", ())
            .unwrap(),
        "true"
    );
    let error = environment
        .render_str("{{ attribute_rank(1, 'start').each }}", ())
        .expect_err("a queried unknown rank must still refuse");
    assert!(error.to_string().contains("UnknownShape"), "{error}");
}
