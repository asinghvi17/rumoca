//! The `base-modelica` and `base-modelica-scalar` targets: MCP-0031 Base
//! Modelica (`//! base 0.1.0`), one `package 'M'` holding the model's user
//! functions and enumeration types and then `model 'M'`, every declared name
//! a quoted identifier. The spellings checked here are the dialect
//! BaseModelica.jl's two parsers read (no `each`, `final`, or `protected`; no
//! signed primary).

use rumoca::{Compiler, render_target_files};

mod comprehension_attributes;

fn render(source: &str, model: &str, target: &str) -> Result<String, String> {
    let compiled = Compiler::new()
        .model(model)
        .compile_str(source, &format!("{model}.mo"))
        .unwrap_or_else(|error| panic!("compile {model}: {error:#}"));
    let mut files = render_target_files(&compiled, model, target, None)
        .map_err(|error| format!("{error:#}"))?;
    assert_eq!(files.len(), 1, "{target} emits one document");
    let file = files.remove(0);
    assert!(file.path.ends_with(".bmo"), "{}", file.path);
    Ok(file.content)
}

fn render_ok(source: &str, model: &str, target: &str) -> String {
    render(source, model, target).unwrap_or_else(|error| panic!("{target}/{model}: {error}"))
}

fn assert_contains_all(text: &str, expected: &[&str]) {
    for needle in expected {
        assert!(text.contains(needle), "missing `{needle}` in:\n{text}");
    }
}

const SPRING_MASS_DAMPER: &str = r#"
model SpringMassDamper "Mass on a spring with viscous damping"
  parameter Real m(unit = "kg") = 1.0 "Mass";
  parameter Real k(unit = "N/m") = 10.0;
  parameter Real d = 0.5;
  Real x(start = -0.5, fixed = true, unit = "m") "Position";
  Real v(start = 0.0, fixed = true);
equation
  der(x) = v;
  m * der(v) = -k * x - d * v;
end SpringMassDamper;
"#;

#[test]
fn wraps_the_model_in_a_versioned_package_of_quoted_names() {
    let out = render_ok(SPRING_MASS_DAMPER, "SpringMassDamper", "base-modelica");
    assert!(
        out.starts_with(
            "//! base 0.1.0\npackage 'SpringMassDamper'\n  model 'SpringMassDamper' \
             \"Mass on a spring with viscous damping\"\n"
        ),
        "{out}"
    );
    assert!(
        out.ends_with("  end 'SpringMassDamper';\nend 'SpringMassDamper';\n"),
        "{out}"
    );
    assert_contains_all(
        &out,
        &[
            "    parameter Real 'm'(unit = \"kg\") = 1.0 \"Mass\";",
            // MCP-0031 has no signed primary: a negative literal is parenthesized.
            "    Real 'x'(unit = \"m\", start = (-0.5), fixed = true) \"Position\";",
            "  equation\n    der('x') = 'v';",
            "('m' * der('v'))",
        ],
    );
    for absent in ["each ", "final ", "protected", "class "] {
        assert!(
            !out.contains(absent),
            "`{absent}` is not Base Modelica:\n{out}"
        );
    }
}

const RC_CIRCUIT: &str = r#"
package RC
  connector Pin
    Real v;
    flow Real i;
  end Pin;
  partial model TwoPin
    Pin p;
    Pin n;
    Real v;
    Real i;
  equation
    v = p.v - n.v;
    0 = p.i + n.i;
    i = p.i;
  end TwoPin;
  model Resistor
    extends TwoPin;
    parameter Real R = 1;
  equation
    v = R * i;
  end Resistor;
  model Capacitor
    extends TwoPin(v(start = 0, fixed = true));
    parameter Real C = 1;
  equation
    C * der(v) = i;
  end Capacitor;
  model Ground
    Pin p;
  equation
    p.v = 0;
  end Ground;
  model Circuit
    Resistor R1(R = 100);
    Capacitor C1(C = 0.01);
    Ground G;
  equation
    connect(R1.p, C1.p);
    connect(R1.n, C1.n);
    connect(C1.n, G.p);
  end Circuit;
end RC;
"#;

#[test]
fn connected_components_flatten_to_quoted_hierarchical_names() {
    let out = render_ok(RC_CIRCUIT, "RC.Circuit", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "package 'RC_Circuit'",
            "    Real 'C1.v'(start = 0, fixed = true);",
            "    parameter Real 'R1.R'",
            "'R1.v' = ('R1.R' * 'R1.i');",
            "'G.p.v' = 0;",
        ],
    );
}

const BOUNCING_BALL: &str = r#"
model BouncingBall
  parameter Real e = 0.8;
  Real h(start = 1, fixed = true);
  Real v(start = 0, fixed = true);
  discrete Integer bounces(start = 0, fixed = true);
equation
  der(h) = v;
  der(v) = -9.81;
  when h < 0 then
    reinit(v, -e * pre(v));
    bounces = pre(bounces) + 1;
  end when;
end BouncingBall;
"#;

#[test]
fn when_equations_keep_reinit_and_discrete_assignments() {
    let out = render_ok(BOUNCING_BALL, "BouncingBall", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "    discrete Integer 'bounces'(start = 0, fixed = true);",
            "    when ('h' < 0) then\n      reinit('v', (-('e' * pre('v'))));\n      \
             'bounces' = (pre('bounces') + 1);\n    end when;\n",
        ],
    );
}

const USER_FUNCTION: &str = r#"
package Fns
  function drag
    input Real v;
    input Real c3 = 0.1;
    output Real f;
  protected
    Real v3;
  algorithm
    v3 := v * v * v;
    f := c3 * v3;
  end drag;
  function saturate
    input Real u;
    output Real y;
  algorithm
    if u > 1 then
      y := 1;
    else
      y := u;
    end if;
  end saturate;
  model Decay
    Real x(start = 1, fixed = true);
    Real y;
  equation
    der(x) = -drag(x);
    y = saturate(x);
  end Decay;
end Fns;
"#;

#[test]
fn user_functions_are_declared_in_the_package_before_the_model() {
    let out = render_ok(USER_FUNCTION, "Fns.Decay", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "  function 'drag'\n    input Real 'v';\n    input Real 'c3' = 0.1;\n    \
             output Real 'f';\n    Real 'v3';\n  algorithm\n    \
             'v3' := (('v' * 'v') * 'v');\n    'f' := ('c3' * 'v3');\n  end 'drag';\n",
            "    if ('u' > 1) then\n      'y' := 1;\n    else\n      'y' := 'u';\n    end if;\n",
            "der('x') = (-'drag'('x', 0.1));",
            "'y' = 'saturate'('x');",
        ],
    );
    let model_at = out.find("  model 'Fns_Decay'").expect("model");
    assert!(
        out.find("  function 'drag'")
            .is_some_and(|at| at < model_at),
        "{out}"
    );
}

#[test]
fn algorithm_sections_are_kept() {
    let source = "model Alg
  Real x(start = 1, fixed = true);
  Real y;
equation
  der(x) = -x;
algorithm
  y := 2*x;
end Alg;
";
    let out = render_ok(source, "Alg", "base-modelica");
    assert_contains_all(&out, &["  algorithm\n    'y' := (2 * 'x');\n  end 'Alg';"]);
}

const ARRAYS: &str = r#"
model Arrays
  parameter Real tau[3] = {1, 2, 3};
  parameter Real a[3] = {1, 1, 2};
  Real x[3](each start = 0, each fixed = true);
  Real z[3](each start = 1, each fixed = true);
equation
  der(x[1]) = (1 - x[1]) / tau[1];
  for i in 2:3 loop
    der(x[i]) = (x[i - 1] - x[i]) / tau[i];
  end for;
  der(z) = -(a .* z);
end Arrays;
"#;

#[test]
fn arrays_stay_arrays_with_fill_for_scalar_attributes() {
    let out = render_ok(ARRAYS, "Arrays", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "    parameter Real 'tau'[3] = {1, 2, 3};",
            "    Real 'x'[3](start = fill(0, 3), fixed = fill(true, 3));",
            "der('x'[1]) = ((1 - 'x'[1]) / 'tau'[1]);",
            "der('z') = (-('a' .* 'z'));",
        ],
    );
}

#[test]
fn scalar_target_declares_and_equates_every_element() {
    let out = render_ok(ARRAYS, "Arrays", "base-modelica-scalar");
    assert_contains_all(
        &out,
        &[
            "    parameter Real 'tau[2]' = 2;",
            "    Real 'x[3]'(start = 0, fixed = true);",
            "der('x[1]') = ((1 - 'x[1]') / 'tau[1]');",
            "der('x[3]') = (('x[2]' - 'x[3]') / 'tau[3]');",
            // A whole-array elementwise equation is split per element, and
            // between scalars `.*` is the plain product.
            "der('z[2]') = (-('a[2]' * 'z[2]'));",
        ],
    );
    assert!(!out.contains("for "), "loops are unrolled:\n{out}");
    assert!(
        !out.contains("'x'["),
        "no array declaration remains:\n{out}"
    );
}

/// A `for` family Flatten keeps unmaterialized has placeholder rows; the
/// scalar target unrolls the family's template body instead of printing them.
#[test]
fn scalar_target_unrolls_unmaterialized_families() {
    let source = "model Family
  parameter Integer N = 6;
  parameter Real k = 1;
  Real x[N](each start = 1);
equation
  for i in 1:N loop
    der(x[i]) = -k * x[i] + 0.5;
  end for;
end Family;
";
    let out = render_ok(source, "Family", "base-modelica-scalar");
    for i in 1..=6 {
        let row = format!("der('x[{i}]') = ((-('k' * 'x[{i}]')) + 0.5);");
        assert!(out.contains(&row), "missing `{row}`:\n{out}");
    }
    assert!(!out.contains("= 0.0;"), "no placeholder row:\n{out}");
}

#[test]
fn scalar_target_refuses_what_it_cannot_split_elementwise() {
    let source = "model MatMul
  parameter Real A[2, 2] = {{1, 2}, {3, 4}};
  Real x[2](each start = 1, each fixed = true);
equation
  der(x) = A * x;
end MatMul;
";
    let error = render(source, "MatMul", "base-modelica-scalar").expect_err("refused");
    assert!(error.contains("cannot scalarize"), "{error}");
    assert!(
        render(source, "MatMul", "base-modelica").is_ok(),
        "the array target keeps the matrix product"
    );
}

#[test]
fn scalar_target_refuses_matrix_power_with_a_scalar_exponent() {
    for exponent in [2, 0, 1] {
        let source = format!(
            "model MatPower
  Real A[2,2] = {{{{1,2}},{{3,4}}}};
  Real y[2,2];
equation
  y = A^{exponent};
end MatPower;"
        );
        let error = render(&source, "MatPower", "base-modelica-scalar")
            .expect_err("matrix power must not become elementwise power");
        assert!(error.contains("matrix operator"), "{error}");
        recompiles(&render_ok(&source, "MatPower", "base-modelica"), "MatPower");
    }
}

#[test]
fn scalar_target_refuses_array_results_from_scalar_function_arguments() {
    let source = "model ArrayResult
  function f
    input Real u;
    output Real v[2];
  algorithm
    v := {u, 2*u};
  end f;
  Real y[2];
equation
  y = f(time);
end ArrayResult;";
    let error = render(source, "ArrayResult", "base-modelica-scalar")
        .expect_err("an array result cannot be assigned to each scalar element");
    assert!(error.contains("array result"), "{error}");
    recompiles(
        &render_ok(source, "ArrayResult", "base-modelica"),
        "ArrayResult",
    );
}

#[test]
fn scalar_target_refuses_symbolic_algorithm_array_targets() {
    let source = "model AlgorithmArray
  Real y[2];
algorithm
  for i in 1:2 loop
    y[i] := time;
  end for;
end AlgorithmArray;";
    let error = render(source, "AlgorithmArray", "base-modelica-scalar")
        .expect_err("symbolic targets must not refer to removed array declarations");
    assert!(error.contains("cannot scalarize"), "{error}");
    recompiles(
        &render_ok(source, "AlgorithmArray", "base-modelica"),
        "AlgorithmArray",
    );
}

#[test]
fn empty_nonunit_fill_slice_keeps_zero_extent() {
    for range in ["2:2:1", "1:-2:2"] {
        let source = format!(
            "model EmptySlice
  Real x[0](start=(fill(7,3))[{range}]);
end EmptySlice;"
        );
        let out = render_ok(&source, "EmptySlice", "base-modelica");
        assert!(out.contains("start = fill(7, 0)"), "{out}");
        recompiles(&out, "EmptySlice");
    }
}

#[test]
fn empty_fill_slice_with_symbolic_extent_exports_zero_extent() {
    let source = "model EmptySym
  parameter Integer n=3;
  Real x[0](start=(fill(7,n))[1:0]);
end EmptySym;";
    let out = render_ok(source, "EmptySym", "base-modelica");
    assert!(out.contains("start = fill(7, 0)"), "{out}");
    recompiles(&out, "EmptySym");
}

#[test]
fn iterator_dependent_array_attributes_export_without_replication() {
    for value in [
        "{a[i] for i in 1:n}",
        "{i*1.0 for i in 1:n}",
        "{sin(i) for i in 1:n}",
    ] {
        for attribute in ["start", "min", "max", "nominal"] {
            let source = format!(
                "model CompAttr parameter Integer n=3; parameter Real a[n]={{1,2,3}};
                 Real x[n]({attribute}={value}); equation der(x)=-x; end CompAttr;"
            );
            let out = render_ok(&source, "CompAttr", "base-modelica");
            assert!(out.contains(&format!("{attribute} = {{")), "{out}");
            assert!(!out.contains(&format!("{attribute} = fill(")), "{out}");
            recompiles(&out, "CompAttr");
        }
    }
}

/// MLS §7.2.5 and §10: only scalar attributes need replication. Sizes
/// may depend on parameters, and selecting an array can retain array axes.
#[test]
fn array_attribute_rank_preserves_symbolic_sizes_and_selections() {
    for (declarations, value, expected) in [
        ("parameter Integer n=2;", "fill(1.0,n)", "fill(1.0, 'n')"),
        ("parameter Integer n=2;", "zeros(n)", "zeros('n')"),
        ("parameter Integer n=2;", "ones(n)", "ones('n')"),
        (
            "parameter Real p[2]={1,2};",
            "{p[1],p[2]}",
            "{'p'[1], 'p'[2]}",
        ),
        ("parameter Real p[3]={1,2,3};", "p[1:2]", "'p'[1:2]"),
        (
            "parameter Real p[2,2]={{1,2},{3,4}};",
            "p[1,:]",
            "'p'[1, :]",
        ),
    ] {
        for attribute in ["start", "min", "max", "nominal"] {
            let source = format!(
                "model AttributeRank {declarations} Real x[2]({attribute}={value}); equation der(x)=-x; end AttributeRank;"
            );
            let out = render_ok(&source, "AttributeRank", "base-modelica");
            assert!(out.contains(&format!("{attribute} = {expected}")), "{out}");
            recompiles(&out, "AttributeRank");
        }
    }
}

#[test]
fn unrendered_parameter_start_does_not_refuse_other_attributes() {
    let source = "model UnrenderedStart
  parameter Real q[2]={1,2};
  parameter Real x[2](each min=0, start={q[1],q[2]})=q;
end UnrenderedStart;";
    let out = render_ok(source, "UnrenderedStart", "base-modelica");
    let x = out.lines().find(|line| line.contains("'x'[2]")).unwrap();
    assert!(x.contains("min = fill(0, 2)"), "{out}");
    assert!(!x.contains("start ="), "{out}");
    recompiles(&out, "UnrenderedStart");
}

#[test]
fn sliced_attribute_modifiers_retain_the_writing_instance() {
    let source = "model Inner
  Real x[2];
equation der(x)=-x;
end Inner;
model Outer
  parameter Real p[3]={1,2,3};
  Inner c(x(start=p[1:2], min=p[1:2], max=p[2:3], nominal=p[1:2]));
end Outer;
model ModifierScope
  Outer a;
  Outer b(p={4,5,6});
end ModifierScope;";
    let out = render_ok(source, "ModifierScope", "base-modelica");
    for prefix in ["a", "b"] {
        let declaration = out
            .lines()
            .find(|line| line.contains(&format!("'{prefix}.c.x'[2]")))
            .unwrap();
        for (attribute, selection) in [
            ("start", "1:2"),
            ("min", "1:2"),
            ("max", "2:3"),
            ("nominal", "1:2"),
        ] {
            assert!(
                declaration.contains(&format!("{attribute} = '{prefix}.p'[{selection}]")),
                "{out}"
            );
        }
    }
    recompiles(&out, "ModifierScope");
}

#[test]
fn array_returning_record_calls_remain_array_attributes_after_record_expansion() {
    let source = "record Frame
  Real T[2,2];
  Real w[2];
end Frame;
function resolveFrame
  input Frame R;
  input Real v[2];
  output Real y[2];
algorithm
  y := R.T*v + R.w;
end resolveFrame;
model RecordCallStart
  parameter Frame R(T={{1,0},{0,1}}, w={0,0});
  parameter Real v[2]={1,2};
  Real x[2](start=resolveFrame(R,v));
equation der(x)=-x;
end RecordCallStart;";
    let out = render_ok(source, "RecordCallStart", "base-modelica");
    let declaration = out.lines().find(|line| line.contains("'x'[2]")).unwrap();
    assert!(declaration.contains("start = 'resolveFrame'("), "{out}");
    assert!(!declaration.contains("start = fill("), "{out}");
    recompiles(&out, "RecordCallStart");
}

#[test]
fn duplicate_inherited_attribute_references_use_effective_declarations() {
    for (parameter, each, expected) in [
        (
            "parameter Real m_flow_start=1;",
            "each ",
            "fill('pipe.flowModel.m_flow_start', 2)",
        ),
        (
            "parameter Real m_flow_start[2]={1,2};",
            "",
            "'pipe.flowModel.m_flow_start'",
        ),
    ] {
        let modifiers = ["start", "min", "max", "nominal"]
            .map(|attribute| format!("{each}{attribute}=m_flow_start"))
            .join(", ");
        let source = format!(
            "partial model FlowInterface
  {parameter}
  Real m_flows[2]({modifiers});
end FlowInterface;
model FlowImplementation
  {parameter}
  extends FlowInterface;
equation der(m_flows)=-m_flows;
end FlowImplementation;
model Pipe
  replaceable model Flow = FlowInterface constrainedby FlowInterface;
  Flow flowModel;
end Pipe;
model RedeclaredStart
  Pipe pipe(redeclare model Flow = FlowImplementation);
end RedeclaredStart;"
        );
        let out = render_ok(&source, "RedeclaredStart", "base-modelica");
        for attribute in ["start", "min", "max", "nominal"] {
            assert!(out.contains(&format!("{attribute} = {expected}")), "{out}");
        }
        recompiles(&out, "RedeclaredStart");
    }
}

#[test]
fn scalar_selected_attributes_are_replicated() {
    let source = "model SelectedAttributes
  parameter Real p[2]={1,2};
  Real x[2](each start=p[1], each min=p[1], each max=p[2], each nominal=p[1]);
equation
  der(x)=-x;
end SelectedAttributes;";
    let out = render_ok(source, "SelectedAttributes", "base-modelica");
    for (key, index) in [("start", 1), ("min", 1), ("max", 2), ("nominal", 1)] {
        assert!(
            out.contains(&format!("{key} = fill('p'[{index}], 2)")),
            "{out}"
        );
    }
    recompiles(&out, "SelectedAttributes");
}

#[test]
fn enumeration_literal_starts_are_scalar_and_replicated() {
    let source = "type Logic = enumeration(U, X);
model EnumStart
  Logic x[2](each start=Logic.U);
equation
  x={Logic.U, Logic.X};
end EnumStart;";
    let out = render_ok(source, "EnumStart", "base-modelica");
    assert!(out.contains("start = fill('Logic'.U, 2)"), "{out}");
    recompiles(&out, "EnumStart");
}

#[test]
fn array_attribute_calls_use_input_and_output_rank() {
    for (inputs, output, algorithm, call, size) in [
        (
            "input Real u[:];",
            "Real y[size(u,1)]",
            "y:=u;",
            "init({1.0,2.0})",
            "2",
        ),
        ("input Real u=1.0;", "Real y[2]", "y:={u,u};", "init()", "2"),
        ("input Real u;", "Real y", "y:=u;", "init({1.0,2.0})", "2"),
        (
            "input Real u;",
            "Real y[2]",
            "y:={u,u};",
            "init({1.0,2.0})",
            "2,2",
        ),
    ] {
        let source = format!(
            "function init {inputs} output {output}; algorithm {algorithm} end init; model CallRank Real x[{size}](start={call}); equation der(x)=-x; end CallRank;"
        );
        let out = render_ok(&source, "CallRank", "base-modelica");
        let x = out.lines().find(|line| line.contains("'x'[")).unwrap();
        assert!(x.contains("start = 'init'("), "{out}");
        assert!(!x.contains("fill("), "{out}");
        recompiles(&out, "CallRank");
    }
}

#[test]
fn invalid_fill_subscripts_remain_visible_in_export() {
    for (selection, size) in [("-1:1", "3"), ("2:5", "4")] {
        let source = format!(
            "model InvalidSlice Real x[{size}](start=(fill(7,3))[{selection}]); equation der(x)=-x; end InvalidSlice;"
        );
        let out = render_ok(&source, "InvalidSlice", "base-modelica");
        assert!(out.contains("start = (fill(7, 3))["), "{out}");
    }
}

#[test]
fn scalar_target_preserves_elementwise_power() {
    let source = "model ElementPower
  Real A[2,2] = {{1,2},{3,4}};
  Real y[2,2];
equation
  y = A.^2;
end ElementPower;";
    let out = render_ok(source, "ElementPower", "base-modelica-scalar");
    assert!(out.contains("'y[1,2]' = ('A[1,2]' ^ 2);"), "{out}");
    recompiles(&out, "ElementPower");
}

#[test]
fn enumerations_are_package_types_with_quoted_literal_paths() {
    let source = "package Types
  type Mode = enumeration(Off, On);
end Types;
model Enum
  parameter Types.Mode mode = Types.Mode.On;
  Real x(start = 0, fixed = true);
equation
  der(x) = if mode == Types.Mode.On then 1 else 0;
end Enum;
";
    let out = render_ok(source, "Enum", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "  type 'Types.Mode' = enumeration(Off, On);\n  model 'Enum'",
            "    parameter 'Types.Mode' 'mode'",
            "= 'Types.Mode'.On",
        ],
    );
}

/// Base Modelica is Modelica: the emitted package recompiles through Rumoca
/// under its quoted model path.
#[test]
fn output_recompiles_through_the_compiler() {
    for (source, model) in [
        (SPRING_MASS_DAMPER, "SpringMassDamper"),
        (BOUNCING_BALL, "BouncingBall"),
        (RC_CIRCUIT, "RC.Circuit"),
        (ARRAYS, "Arrays"),
    ] {
        for target in ["base-modelica", "base-modelica-scalar"] {
            let out = render_ok(source, model, target);
            let name = model.replace('.', "_");
            Compiler::new()
                .model(&format!("'{name}'.'{name}'"))
                .compile_str(&out, &format!("{name}.bmo"))
                .unwrap_or_else(|error| panic!("{target}/{model} recompiles: {error:#}\n{out}"));
        }
    }
}

/// A parameter or constant's value is its binding (MLS §8.6): no `start`
/// beside it, in particular not the declared type's default `start = 0.0`
/// that a parameter bound by a component modifier (`R1(R = 100)`) carries.
/// A `fixed = false` parameter keeps its start, its iteration guess.
#[test]
fn a_bound_parameter_carries_only_its_binding() {
    let source = "package P
  model Inner
    parameter Real R = 1000.0 \"Resistance\";
    parameter Real k(start = 2.0) = 3.0;
    parameter Real g(fixed = false, start = 0.5);
    Real x(start = 1.0, fixed = true);
  initial equation
    g = 2 * k;
  equation
    der(x) = -x / R * k;
  end Inner;
  model Top
    Inner r1(R = 7.0);
  end Top;
end P;";
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "P.Top", target);
        assert_contains_all(
            &out,
            &[
                "    parameter Real 'r1.R' = 7.0 \"Resistance\";",
                "    parameter Real 'r1.k' = 3.0;",
                "    parameter Real 'r1.g'(start = 0.5, fixed = false);",
                "    Real 'r1.x'(start = 1.0, fixed = true);",
            ],
        );
    }
    let out = render_ok(RC_CIRCUIT, "RC.Circuit", "base-modelica");
    assert_contains_all(&out, &["    parameter Real 'R1.R' = 100;"]);
}

/// MCP-0031 annotations.md: the `experiment` class annotation of the model
/// carries its default StartTime, StopTime, Tolerance and Interval, in that
/// order and only the ones the source sets.
#[test]
fn the_experiment_annotation_closes_the_model() {
    let source = "model Exp
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
  annotation(experiment(StopTime = 5, Interval = 0.01, StartTime = -1, Tolerance = 1e-8));
end Exp;";
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "Exp", target);
        assert!(
            out.ends_with(
                "    der('x') = (-'x');\n  annotation(experiment(StartTime = -1.0, \
                 StopTime = 5.0, Tolerance = 0.00000001, Interval = 0.01));\n  \
                 end 'Exp';\nend 'Exp';\n"
            ),
            "{target}:\n{out}"
        );
    }
    let partial = "model Exp
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
  annotation(experiment(StopTime = 2));
end Exp;";
    let out = render_ok(partial, "Exp", "base-modelica");
    assert_contains_all(
        &out,
        &["  annotation(experiment(StopTime = 2.0));\n  end 'Exp';"],
    );
    let out = render_ok(SPRING_MASS_DAMPER, "SpringMassDamper", "base-modelica");
    assert!(!out.contains("annotation"), "{out}");
}

const RECORD_FUNCTIONS: &str = r#"
package Rec
  record Point
    Real x;
    Real y;
  end Point;
  function make
    input Real a;
    output Point p;
  algorithm
    p := Point(a, 2 * a);
  end make;
  function sum
    input Real a;
    output Real n;
  protected
    Point p;
  algorithm
    p := make(a);
    n := p.x + p.y;
  end sum;
  model M
    Point q(x = 1, y = 2);
    Real s;
  equation
    s = sum(time) + q.x;
  end M;
end Rec;
"#;

/// MCP-0031 declares a record type a function uses as a package-level
/// `record`, has no implicit record constructors (so `Point(...)` calls a
/// generated `'-Rec.Point'`), and reaches members through the instance
/// (`'p'.'x'`). A record component of the model itself stays flattened to its
/// fields, which the spec permits.
#[test]
fn records_used_by_functions_are_package_classes() {
    let out = render_ok(RECORD_FUNCTIONS, "Rec.M", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "  record 'Rec.Point'\n    Real 'x';\n    Real 'y';\n  end 'Rec.Point';\n",
            "  function '-Rec.Point' \"Automatically generated constructor for 'Rec.Point'\"\n    \
             input Real 'x';\n    input Real 'y';\n    \
             output 'Rec.Point' _result('x' = 'x', 'y' = 'y');\n  end '-Rec.Point';\n",
            "    output 'Rec.Point' 'p';",
            "    'p' := '-Rec.Point'('a', (2 * 'a'));",
            "    'Rec.Point' 'p';",
            "    'n' := ('p'.'x' + 'p'.'y');",
            "    Real 'q.x'(start = 0.0) = 1;",
        ],
    );
    let record_at = out.find("  record 'Rec.Point'").expect("record");
    assert!(
        out.find("  function").is_some_and(|at| record_at < at),
        "{out}"
    );
    assert!(!out.contains("function 'Rec.Point'"), "{out}");
}

/// A record-valued equation on a model record (flattened to its fields) is
/// its field equations (MLS §8.3.1): from another record of the same type,
/// from a constructor call, and from a function returning the record, each
/// field read off the call (MSL QuasiStatic and ComplexBlocks `Complex`
/// equations).
#[test]
fn record_valued_model_equations_are_written_field_by_field() {
    let source = "package Rec
  record Point
    Real x;
    Real y;
  end Point;
  function make
    input Real a;
    output Point p;
  algorithm
    p := Point(a, 2 * a);
  end make;
  model M
    Point w;
    Point u;
    Point c;
  equation
    w = make(time);
    u = w;
    c = Point(time, 3);
  end M;
end Rec;";
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "Rec.M", target);
        assert_contains_all(
            &out,
            &[
                "    'w.x' = '-make().x'(time);\n    'w.y' = '-make().y'(time);",
                "    'u.x' = 'w.x';\n    'u.y' = 'w.y';",
                "    'c.x' = time;\n    'c.y' = 3;",
            ],
        );
    }
}

/// A member read off a record-valued call (`make(a).x`) has no Base Modelica
/// spelling: MCP-0031's `primary` ends at the call arguments, and a record
/// member is reached only through an instance. Each member read this way is
/// a call of a generated accessor `'-f().x'` that keeps the call's inputs,
/// stores the call in a record local and returns the member, wherever the
/// read occurs (a binding, an expression, a function body, a nested record
/// member, a record equation). A read off a record-valued `if` moves into
/// its arms, and a constructor's member is its argument. 109 MSL examples (MultiBody `Frames`, Media
/// `ThermodynamicState`, `Complex` operators) stopped at the member access.
#[test]
fn members_of_record_valued_calls_read_through_generated_accessors() {
    let source = "package Rec
  record Point
    Real x;
    Real y;
  end Point;
  record Seg
    Point a;
    Real w;
  end Seg;
  function make
    input Real a;
    input Real k = 2;
    output Point p;
  algorithm
    p := Point(a, k * a);
  end make;
  function seg
    input Real a;
    output Seg s;
  algorithm
    s := Seg(Point(a, a), 1);
  end seg;
  function norm
    input Point p;
    output Real n;
  algorithm
    n := abs(p.x) + abs(p.y);
  end norm;
  function norm1
    input Real a;
    output Real n;
  algorithm
    n := norm(make(a));
  end norm1;
  model M
    parameter Point q = make(3);
    Seg s = seg(time);
    Real u;
    Real v;
    Point w;
    Point z = if time > 0.5 then make(time) else Point(0, 1);
  equation
    u = norm(make(time, 4));
    v = norm1(time);
    w = make(v);
  end M;
end Rec;";
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "Rec.M", target);
        assert_contains_all(
            &out,
            &[
                "  function '-make().x' \"Automatically generated accessor for member x of the \
                 result of make\"\n    input Real 'a';\n    input Real 'k' = 2;\n    \
                 output Real '-member';\n    'Rec.Point' '-record';\n  algorithm\n    \
                 '-record' := 'make'('a', 'k');\n    '-member' := '-record'.'x';\n  \
                 end '-make().x';\n",
                "  function '-seg().a.x' ",
                "    '-member' := '-record'.'a'.'x';",
                "    'n' := 'Rec.norm'('-Rec.make().x'('a', 2), '-Rec.make().y'('a', 2));",
                "    parameter Real 'q.x' = '-make().x'(3, 2);",
                "    Real 's.a.x'(start = 0.0) = '-seg().a.x'(time);",
                "    'u' = 'norm'('-make().x'(time, 4), '-make().y'(time, 4));",
                "    'w.x' = '-make().x'('v', 2);\n    'w.y' = '-make().y'('v', 2);",
                "    Real 'z.x'(start = 0.0) = (if (time > 0.5) then '-make().x'(time, 2) else 0);",
                "    Real 'z.y'(start = 0.0) = (if (time > 0.5) then '-make().y'(time, 2) else 1);",
            ],
        );
        assert!(!out.contains(").'"), "member access on a call in:\n{out}");
        assert!(!out.contains("'-seg().a'"), "unused accessor in:\n{out}");
    }
}

/// Re-reads emitted Base Modelica through the compiler under its quoted path.
fn recompiles(out: &str, model: &str) {
    Compiler::new()
        .model(&format!("'{model}'.'{model}'"))
        .compile_str(out, &format!("{model}.bmo"))
        .unwrap_or_else(|error| panic!("{model} recompiles: {error:#}\n{out}"));
}

/// A function parameter's declared dimensions are subscripts, not
/// expressions: an unsized `u[:]`, a literal `c[2]` and an expression
/// `b[size(a, 1)]` each keep their source spelling (MSL Polynomials,
/// Vectors, Tables, Matrices; 212 MSL examples stopped here).
#[test]
fn function_parameter_dimensions_render_as_subscripts() {
    let source = "model FunctionDims
  function dot
    input Real a[:];
    input Real b[size(a, 1)];
    input Real c[2];
    output Real y;
  algorithm
    y := a * b + c[1];
  end dot;
  parameter Real p[2] = {1, 2};
  Real x(start = 0, fixed = true);
equation
  der(x) = dot(p, p, p) - x;
end FunctionDims;";
    let out = render_ok(source, "FunctionDims", "base-modelica");
    assert_contains_all(
        &out,
        &[
            "    input Real 'a'[:];",
            "    input Real 'b'[size('a', 1)];",
            "    input Real 'c'[2];",
        ],
    );
    recompiles(&out, "FunctionDims");
}

/// The MLS §3.7.5 event operator `sample(start, interval)` is a Base Modelica
/// builtin and is written as is in a `when` condition (MSL Blocks.Discrete,
/// SampleTrigger).
#[test]
fn event_sample_is_kept_in_when_conditions() {
    let source = "model EventSample
  Integer n(start = 0, fixed = true);
equation
  when sample(0.05, 0.1) then
    n = pre(n) + 1;
  end when;
end EventSample;";
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "EventSample", target);
        assert_contains_all(
            &out,
            &["    when sample(0.05, 0.1) then\n      'n' = (pre('n') + 1);\n    end when;"],
        );
        recompiles(&out, "EventSample");
    }
}

/// The predefined `String(...)` conversion (MLS §3.7.1) renders with the
/// formatting arguments the source gave, in an `assert` message and in a
/// function body (MSL Limiter, PID, Media property checks).
#[test]
fn string_conversions_keep_their_format_arguments() {
    let source = "model StringConv
  function label
    input Real v;
    input Integer n;
    output String s;
  algorithm
    s := String(v, significantDigits = 3) + String(n, minimumLength = 4, leftJustified = false)
      + String(v, format = \"6.2f\");
  end label;
  parameter Real k = 2;
  parameter String l = label(k, 3);
  Real x(start = 1, fixed = true);
equation
  der(x) = -k * x;
  assert(k > 0, \"k = \" + String(k) + \" must be positive\");
end StringConv;";
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "StringConv", target);
        assert_contains_all(
            &out,
            &[
                "    assert(('k' > 0), ((\"k = \" + String('k')) + \" must be positive\"));",
                "String('v', significantDigits = 3)",
                "String('n', minimumLength = 4, leftJustified = false)",
                "String('v', format = \"6.2f\")",
            ],
        );
        recompiles(&out, "StringConv");
    }
}

/// Descriptions and string literals hold decoded text, so a quote or a
/// backslash in them is escaped again on output: `"Choose \"Y\""` must not
/// come out as `"Choose "Y""`, which does not parse (MSL Machines
/// TerminalBox).
#[test]
fn quotes_in_descriptions_and_string_literals_are_escaped() {
    let source = r#"model Escaped "A \"quoted\" model"
  parameter String connection = "Y \"star\" \\ D" "Choose \"Y\" for star";
  Real x(start = 1, fixed = true) "back\\slash";
equation
  der(x) = -x;
  assert(x > -1, "x is \"negative\"");
end Escaped;"#;
    for target in ["base-modelica", "base-modelica-scalar"] {
        let out = render_ok(source, "Escaped", target);
        assert_contains_all(
            &out,
            &[
                r#"  model 'Escaped' "A \"quoted\" model""#,
                r#"    parameter String 'connection' = "Y \"star\" \\ D" "Choose \"Y\" for star";"#,
                r#"    Real 'x'(start = 1, fixed = true) "back\\slash";"#,
                r#"assert(('x' > (-1)), "x is \"negative\"");"#,
            ],
        );
        recompiles(&out, "Escaped");
    }
}
