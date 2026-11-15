use super::*;
use rumoca_core::{
    ArrayConstructor, BuiltinFunction, ComponentRefPart, ComponentReference, EffectiveType,
    FunctionParam, Literal, OpBinary, OpUnary, Reference, ResolvedFunctionReference, SourceId,
    Subscript, TypeId,
};

fn span() -> Span {
    Span::from_offsets(SourceId::from_source_name("attributes.mo"), 2, 11)
}
fn literal(value: f64) -> Expression {
    Expression::Literal {
        value: Literal::Real(value),
        span: span(),
    }
}
fn reference(name: &str, instance: Option<InstanceId>) -> Expression {
    let mut name = Reference::new(name);
    if let Some(instance) = instance {
        name = name.with_instance_id(instance);
    }
    Expression::VarRef {
        name,
        subscripts: vec![],
        span: span(),
    }
}
fn model(dimensions: Vec<i64>, source: Expression, reference_dimensions: Vec<i64>) -> Model {
    let mut model = Model::default();
    for (index, (name, dimensions, start)) in [
        ("x", dimensions, Some(source)),
        ("p", reference_dimensions, None),
    ]
    .into_iter()
    .enumerate()
    {
        let type_id = TypeId::new(index as u32);
        model.effective_types.insert(
            type_id,
            EffectiveType::new(type_id, type_id, dimensions.clone()).unwrap(),
        );
        model.variables.insert(
            VarName::new(name),
            Variable {
                name: VarName::new(name),
                instance_id: InstanceId::new(index as u32 + 1),
                type_id,
                dims: dimensions,
                start,
                ..Variable::empty_with_span(span())
            },
        );
    }
    model
}
fn start<'model>(catalog: &SourceAttributeCatalog<'model>) -> SourceAttributeValue<'model> {
    catalog
        .attribute(InstanceId::new(1), SourceAttributeKind::Start)
        .unwrap()
        .unwrap()
}

#[test]
fn scalar_attributes_issue_each_only_on_array_occurrences_and_preserve_values() {
    for dimensions in [vec![], vec![0], vec![2_000_000]] {
        let model = model(dimensions.clone(), literal(-0.0), vec![]);
        let before = serde_json::to_value(&model).unwrap();
        let catalog = model.source_attributes().unwrap();
        let value = start(&catalog);
        assert_eq!(value.kind(), SourceAttributeKind::Start);
        assert_eq!(value.rank(), 0);
        assert_eq!(value.each(), !dimensions.is_empty());
        assert!(std::ptr::eq(
            value.expression(),
            model.variables[&VarName::new("x")].start.as_ref().unwrap()
        ));
        let Expression::Literal {
            value: Literal::Real(value),
            ..
        } = value.expression()
        else {
            panic!("original literal")
        };
        assert_eq!(value.to_bits(), (-0.0f64).to_bits());
        assert_eq!(serde_json::to_value(&model).unwrap(), before);
    }
}

#[test]
fn reference_rank_comes_from_exact_finalized_occurrence_without_rechecking_extents() {
    for dimensions in [vec![], vec![0], vec![4], vec![2_000_000], vec![0, 2]] {
        let model = model(
            vec![2_000_000],
            reference("p", Some(InstanceId::new(2))),
            dimensions.clone(),
        );
        let catalog = model.source_attributes().unwrap();
        assert_eq!(start(&catalog).rank(), dimensions.len());
        assert_eq!(start(&catalog).each(), dimensions.is_empty());
    }
}

#[test]
fn rendered_name_alone_or_mismatched_identity_cannot_issue_rank() {
    for expression in [
        reference("p", None),
        reference("p", Some(InstanceId::new(1))),
        reference("other", Some(InstanceId::new(2))),
    ] {
        let model = model(vec![4], expression, vec![4]);
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
fn unknown_attribute_does_not_refuse_a_different_attribute() {
    let mut model = model(vec![4], literal(1.0), vec![]);
    model.variables.get_mut(&VarName::new("x")).unwrap().min = Some(Expression::FunctionCall {
        name: Reference::new("opaque"),
        args: vec![],
        is_constructor: false,
        span: span(),
    });
    let catalog = model.source_attributes().unwrap();
    assert!(start(&catalog).each());
    assert!(
        matches!(catalog.attribute(InstanceId::new(1), SourceAttributeKind::Min), Err(SourceAttributeError::UnknownShape { variable, attribute: SourceAttributeKind::Min, span: at }) if variable == InstanceId::new(1) && at == span())
    );
    assert!(
        catalog
            .attribute(InstanceId::new(1), SourceAttributeKind::Max)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        catalog.attribute(InstanceId::new(99), SourceAttributeKind::Start),
        Err(SourceAttributeError::MissingVariable(_))
    ));
}

#[test]
fn calls_use_exact_callable_rank_with_defaults_colons_and_vectorization() {
    for (input_dims, output_dims, argument_dims, defaulted, expected) in [
        (vec![], vec![2], vec![], false, 1),
        (vec![0], vec![0], vec![4], false, 1),
        (vec![], vec![2], vec![], true, 1),
        (vec![], vec![], vec![2], false, 1),
        (vec![], vec![2], vec![3], false, 2),
    ] {
        let mut function = Function::new("init", span());
        let mut input = FunctionParam::new(
            "u",
            "Real",
            EffectiveType::new(TypeId::new(10), TypeId::new(10), input_dims.clone()).unwrap(),
            span(),
        );
        if defaulted {
            input.default = Some(literal(1.0));
        }
        if !input_dims.is_empty() {
            input.shape_expr = vec![Subscript::colon(span())];
        }
        function.inputs.push(input);
        function.outputs.push(FunctionParam::new(
            "y",
            "Real",
            EffectiveType::new(TypeId::new(10), TypeId::new(10), output_dims).unwrap(),
            span(),
        ));
        let mut model = model(vec![2], literal(1.0), argument_dims);
        model.add_function(function);
        let instance_id = model.functions[&VarName::new("init")].instance_id.unwrap();
        for selected in [Some(instance_id), None, Some(FunctionInstanceId::new(999))] {
            let mut name = Reference::new("init");
            if let Some(instance_id) = selected {
                name = name.with_resolved_function(ResolvedFunctionReference {
                    instance_id,
                    base_part_count: 1,
                    transitively_non_replaceable: false,
                });
            }
            model.variables.get_mut(&VarName::new("x")).unwrap().start =
                Some(Expression::FunctionCall {
                    name,
                    args: if defaulted {
                        vec![]
                    } else {
                        vec![reference("p", Some(InstanceId::new(2)))]
                    },
                    is_constructor: false,
                    span: span(),
                });
            let catalog = model.source_attributes().unwrap();
            let result = catalog.attribute(InstanceId::new(1), SourceAttributeKind::Start);
            if selected == Some(instance_id) {
                assert_eq!(result.unwrap().unwrap().rank(), expected);
            } else {
                assert!(matches!(
                    result,
                    Err(SourceAttributeError::UnknownShape { .. })
                ));
            }
        }
    }
}

#[test]
fn symbolic_sizes_and_subscripted_elements_have_value_independent_rank() {
    let p = reference("p", Some(InstanceId::new(2)));
    let selected = Expression::Index {
        base: Box::new(p.clone()),
        subscripts: vec![Subscript::index(1, span())],
        span: span(),
    };
    for (expression, expected) in [
        (
            Expression::BuiltinCall {
                function: BuiltinFunction::Fill,
                args: vec![literal(1.0), p.clone()],
                span: span(),
            },
            1,
        ),
        (
            Expression::BuiltinCall {
                function: BuiltinFunction::Fill,
                args: vec![p.clone(), literal(2.0)],
                span: span(),
            },
            2,
        ),
        (
            Expression::BuiltinCall {
                function: BuiltinFunction::Zeros,
                args: vec![p.clone()],
                span: span(),
            },
            1,
        ),
        (
            Expression::BuiltinCall {
                function: BuiltinFunction::Ones,
                args: vec![p.clone(), p.clone()],
                span: span(),
            },
            2,
        ),
        (
            Expression::Array {
                elements: vec![selected.clone(), selected],
                kind: ArrayConstructor::Array,
                span: span(),
            },
            1,
        ),
        (
            Expression::Array {
                elements: vec![],
                kind: ArrayConstructor::Array,
                span: span(),
            },
            1,
        ),
        (
            Expression::Array {
                elements: vec![p.clone(), p.clone()],
                kind: ArrayConstructor::Horizontal,
                span: span(),
            },
            2,
        ),
        (
            Expression::Unary {
                op: OpUnary::Minus,
                rhs: Box::new(p.clone()),
                span: span(),
            },
            1,
        ),
        (
            Expression::Binary {
                op: OpBinary::Mul,
                lhs: Box::new(literal(2.0)),
                rhs: Box::new(p),
                span: span(),
            },
            1,
        ),
    ] {
        let model = model(vec![2], expression, vec![2]);
        let catalog = model.source_attributes().unwrap();
        assert_eq!(start(&catalog).rank(), expected);
        assert!(!start(&catalog).each());
    }
}

#[test]
fn structured_and_outer_subscripts_preserve_only_unselected_axes() {
    for dims in [vec![2], vec![2, 3]] {
        for subscript in [
            Subscript::index(1, span()),
            Subscript::colon(span()),
            Subscript::Expr {
                expr: Box::new(Expression::Range {
                    start: Box::new(literal(1.0)),
                    step: None,
                    end: Box::new(literal(2.0)),
                    span: span(),
                }),
                span: span(),
            },
        ] {
            let expected = dims.len() - usize::from(matches!(subscript, Subscript::Index { .. }));
            for embedded in [false, true] {
                let mut name = Reference::new("p").with_instance_id(InstanceId::new(2));
                if embedded {
                    let component = ComponentReference::construct(
                        false,
                        span(),
                        vec![ComponentRefPart {
                            ident: "p".into(),
                            span: span(),
                            def_id: DefId::new(22),
                            subs: vec![subscript.clone()],
                        }],
                    )
                    .unwrap();
                    name = Reference::with_component_reference("p", component)
                        .with_instance_id(InstanceId::new(2));
                }
                let expression = Expression::VarRef {
                    name,
                    subscripts: if embedded {
                        vec![]
                    } else {
                        vec![subscript.clone()]
                    },
                    span: span(),
                };
                let model = model(vec![2], expression, dims.clone());
                let catalog = model.source_attributes().unwrap();
                assert_eq!(start(&catalog).rank(), expected);
                assert_eq!(start(&catalog).each(), expected == 0);
            }
        }
    }
}

#[test]
fn field_access_uses_resolved_field_identity_and_retains_container_rank() {
    for (dims, field_dims) in [(vec![], vec![2]), (vec![2], vec![])] {
        let expression = Expression::FieldAccess {
            base: Box::new(reference("p", Some(InstanceId::new(2)))),
            field: "q".into(),
            field_def_id: DefId::new(23),
            span: span(),
        };
        let mut model = model(vec![2], expression, dims);
        model.record_types.insert(
            DefId::new(22),
            crate::RecordType {
                name: "R".into(),
                fields: vec![crate::RecordField {
                    name: "q".into(),
                    def_id: DefId::new(23),
                    dims: field_dims,
                }],
            },
        );
        let catalog = model.source_attributes().unwrap();
        assert_eq!(start(&catalog).rank(), 1);
    }
    let component = ComponentReference::construct(
        false,
        span(),
        vec![ComponentRefPart {
            ident: "other".into(),
            span: span(),
            def_id: DefId::new(23),
            subs: vec![],
        }],
    )
    .unwrap();
    let expression = Expression::VarRef {
        name: Reference::with_component_reference("p", component)
            .with_instance_id(InstanceId::new(2)),
        subscripts: vec![],
        span: span(),
    };
    let model = model(vec![2], expression, vec![2]);
    assert!(
        model
            .source_attributes()
            .unwrap()
            .attribute(InstanceId::new(1), SourceAttributeKind::Start)
            .is_err()
    );
}

#[test]
fn catalog_rejects_stale_effective_shape_and_duplicate_occurrence() {
    let mut model = model(vec![4], literal(1.0), vec![]);
    model.variables.get_mut(&VarName::new("x")).unwrap().dims = vec![5];
    assert!(model.source_attributes().is_err());
    model.variables.get_mut(&VarName::new("x")).unwrap().dims = vec![4];
    model
        .variables
        .get_mut(&VarName::new("p"))
        .unwrap()
        .instance_id = InstanceId::new(1);
    assert!(model.source_attributes().is_err());
}
