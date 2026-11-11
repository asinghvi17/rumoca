//! The `base-modelica` and `base-modelica-scalar` targets: MCP-0031 Base
//! Modelica (`//! base 0.1.0`), one `package 'M'` holding the model's user
//! functions and enumeration types and then `model 'M'`, every declared name
//! a quoted identifier. The spellings checked here are the dialect
//! BaseModelica.jl's two parsers read (no `each`, `final`, or `protected`; no
//! signed primary).

use rumoca::{Compiler, render_target_files};

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

/// A record-valued equation on a model record (flattened to its fields) has
/// no Base Modelica spelling; it is refused rather than emitted undeclared.
#[test]
fn a_record_valued_model_equation_is_refused() {
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
  equation
    w = make(time);
  end M;
end Rec;";
    let error = render(source, "Rec.M", "base-modelica").expect_err("record equation");
    assert!(
        error.contains("unsupported-feature:base-modelica-record-value") && error.contains("`w`"),
        "{error}"
    );
}
