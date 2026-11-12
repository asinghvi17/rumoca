//! Base Modelica output of parameters translation evaluated (MLS 3.7 §4.4.5,
//! §8.3.3, §8.3.4, §10.1, §18.3; SPEC_0040 DAE-C22).
//!
//! MCP-0031 has no conditional components, no parameter-sized arrays and no
//! unbalanced if-equations, so the exporter decides them, and the parameters
//! that decide them keep a value that can no longer take effect. Each such
//! parameter, an `Evaluate = true` one and a `final` one with an evaluable
//! binding keep full Modelica's `annotation(Evaluate = true)` (MCP-0031:
//! annotations stay annotations); every other parameter stays unmarked and
//! tunable, and a `constant` stays `constant`. Each structural use and each
//! `Evaluate = true` declaration also warns (WD001).
//!
//! One test per probe of `outputs/structural-params/probes` (copied to
//! `fixtures/structural_params`): the emitted model, and the WD001 messages.

use rumoca::{Compiler, render_target_files};
use rumoca_compile::compile::{Session, SessionConfig};

fn render(source: &str, model: &str, target: &str) -> String {
    let compiled = Compiler::new()
        .model(model)
        .compile_str(source, &format!("{model}.mo"))
        .unwrap_or_else(|error| panic!("compile {model}: {error:#}"));
    let mut files = render_target_files(&compiled, model, target, None)
        .unwrap_or_else(|error| panic!("{target}/{model}: {error:#}"));
    assert_eq!(files.len(), 1, "{target} emits one document");
    files.remove(0).content
}

fn wd001_messages(source: &str, model: &str) -> Vec<String> {
    let mut session = Session::new(SessionConfig::default());
    session
        .add_document(&format!("{model}.mo"), source)
        .expect("probe parses");
    session
        .compile_model_diagnostics(model)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code.as_deref() == Some("WD001"))
        .map(|diagnostic| diagnostic.message)
        .collect()
}

/// The `base-modelica` rendering of `model`, from `model '...'` to its `end`,
/// equals `expected`, and its WD001 warnings are `warnings`, in order.
fn assert_probe(source: &str, model: &str, expected: &str, warnings: &[&str]) {
    let out = render(source, model, "base-modelica");
    let start = out
        .find("  model ")
        .unwrap_or_else(|| panic!("no model in:\n{out}"));
    let end = out.rfind("end '").unwrap_or(out.len());
    assert_eq!(&out[start + 2..end], expected, "{model}:\n{out}");
    assert_eq!(wd001_messages(source, model), warnings, "{model}");
}

/// An array dimension and a for range read `n` (MLS §10.1, §8.3.3): the
/// array is sized and the loop bounded at translation.
#[test]
fn an_array_dimension_parameter_is_marked_evaluated() {
    assert_probe(
        include_str!("fixtures/structural_params/ArraySize.mo"),
        "ArraySize",
        "\
model 'ArraySize'
    parameter Integer 'n' = 3 annotation(Evaluate = true);
    Real 'x'[3](start = fill(0, 3), fixed = fill(true, 3));
  equation
    for i in 1:3 loop
      der('x'[i]) = ((-(i * 'x'[i])) + 1);
    end for;
  end 'ArraySize';
",
        &[
            "this array dimension is fixed at translation, so parameter n cannot be set (MLS 3.7 §10.1)",
            "this for-equation range is fixed at translation, so parameter n cannot be set (MLS 3.7 §8.3.3)",
        ],
    );
}

/// A dimension set by a component modifier (`Sub s(n = 4)`) is fixed the same way.
#[test]
fn a_modified_array_dimension_parameter_is_marked_evaluated() {
    assert_probe(
        include_str!("fixtures/structural_params/ArrayModified.mo"),
        "ArrayModified",
        "\
model 'ArrayModified'
    parameter Integer 's.n' = 4 annotation(Evaluate = true);
    Real 's.x'[4](start = fill(1, 4), fixed = fill(true, 4));
  equation
    der('s.x') = (-'s.x');
  end 'ArrayModified';
",
        &[
            "this array dimension is fixed at translation, so parameter s.n cannot be set (MLS 3.7 §10.1)",
        ],
    );
}

/// Two for ranges read `m` (MLS §8.3.3): both loops are bounded at translation.
#[test]
fn a_for_range_parameter_is_marked_evaluated() {
    assert_probe(
        include_str!("fixtures/structural_params/ForParamRange.mo"),
        "ForParamRange",
        "\
model 'ForParamRange'
    parameter Integer 'm' = 2 annotation(Evaluate = true);
    Real 'x'[3](start = fill(0, 3), fixed = fill(true, 3));
  equation
    for i in 1:2 loop
      der('x'[i]) = i;
    end for;
    for i in 3:3 loop
      der('x'[i]) = 0;
    end for;
  end 'ForParamRange';
",
        &[
            "this for-equation range is fixed at translation, so parameter m cannot be set (MLS 3.7 §8.3.3)",
            "this for-equation range is fixed at translation, so parameter m cannot be set (MLS 3.7 §8.3.3)",
        ],
    );
}

/// A constant range is a constant (it stays `constant`), and `m`, read by
/// nothing structural, stays an ordinary parameter.
#[test]
fn a_constant_for_range_stays_constant_and_an_unread_parameter_unmarked() {
    assert_probe(
        include_str!("fixtures/structural_params/ForRange.mo"),
        "ForRange",
        "\
model 'ForRange'
    constant Integer 'n' = 3;
    parameter Integer 'm' = 2;
    Real 'x'[3](start = fill(0, 3), fixed = fill(true, 3));
  equation
    for i in 1:3 loop
      der('x'[i]) = i;
    end for;
  end 'ForRange';
",
        &[],
    );
}

/// Branches of equal size keep the if-equation as a run-time conditional
/// (MLS §8.3.4): `fast` stays tunable and unmarked.
#[test]
fn a_balanced_if_equation_guard_stays_tunable() {
    assert_probe(
        include_str!("fixtures/structural_params/IfEqSameSize.mo"),
        "IfEqSameSize",
        "\
model 'IfEqSameSize'
    parameter Boolean 'fast' = true;
    Real 'x'(start = 0, fixed = true);
  equation
    0 = (if 'fast' then (der('x') - 2) else (der('x') - 1));
  end 'IfEqSameSize';
",
        &[],
    );
}

/// Branches of different size are selected at translation (MLS §8.3.4): the
/// `withY = true` branch is kept and `withY` is marked.
#[test]
fn an_unbalanced_if_equation_guard_is_marked_evaluated() {
    assert_probe(
        include_str!("fixtures/structural_params/IfEqUnbalanced.mo"),
        "IfEqUnbalanced",
        "\
model 'IfEqUnbalanced'
    parameter Boolean 'withY' = true annotation(Evaluate = true);
    Real 'x'(start = 0, fixed = true);
    Real 'y'(start = 0, fixed = true);
  equation
    der('x') = 1;
    der('y') = 'x';
  end 'IfEqUnbalanced';
",
        &[
            "the branch of this equation is selected at translation, so parameter withY cannot be set: its branches differ in structure (MLS 3.7 §8.3.4)",
        ],
    );
}

/// A parameter-valued selector between balanced branches stays tunable.
#[test]
fn a_parameter_selecting_balanced_branches_stays_tunable() {
    assert_probe(
        include_str!("fixtures/structural_params/ParamSelect.mo"),
        "ParamSelect",
        "\
model 'ParamSelect'
    parameter Integer 'kind' = 1 \"1 = linear, 2 = quadratic\";
    Real 'x'(start = 1, fixed = true);
  equation
    0 = (if ('kind' == 1) then (der('x') - (-'x')) else (der('x') - (-('x' * 'x'))));
  end 'ParamSelect';
",
        &[],
    );
}

/// A conditional component's condition is decided at translation
/// (MLS §4.4.5): `dmp` exists and `useDamper` is marked.
#[test]
fn an_enabled_conditional_component_condition_is_marked_evaluated() {
    assert_probe(
        include_str!("fixtures/structural_params/CondComp.mo"),
        "CondComp",
        "\
model 'CondComp'
    parameter Boolean 'useDamper' = true annotation(Evaluate = true);
    parameter Real 'dmp.d' = 1;
    Real 'dmp.v'(start = 1, fixed = true);
    Real 'y'(start = 0, fixed = true);
  equation
    der('dmp.v') = (-('dmp.d' * 'dmp.v'));
    der('y') = 1;
  end 'CondComp';
",
        &[
            "this component condition is decided at translation, so parameter useDamper cannot be set: the component exists or not by its value (MLS 3.7 §4.4.5)",
        ],
    );
}

/// A false condition removes the component; its condition is marked just the
/// same.
#[test]
fn a_disabled_conditional_component_condition_is_marked_evaluated() {
    assert_probe(
        include_str!("fixtures/structural_params/CondCompOff.mo"),
        "CondCompOff",
        "\
model 'CondCompOff'
    parameter Boolean 'useDamper' = false annotation(Evaluate = true);
    Real 'y'(start = 0, fixed = true);
  equation
    der('y') = 1;
  end 'CondCompOff';
",
        &[
            "this component condition is decided at translation, so parameter useDamper cannot be set: the component exists or not by its value (MLS 3.7 §4.4.5)",
        ],
    );
}

/// `Evaluate = true` carries over as the same annotation (MCP-0031:
/// annotations stay annotations); `q` and the dependent `r` stay tunable.
#[test]
fn an_evaluate_true_parameter_keeps_its_annotation() {
    assert_probe(
        include_str!("fixtures/structural_params/EvaluateAnn.mo"),
        "EvaluateAnn",
        "\
model 'EvaluateAnn'
    parameter Real 'p' = 2 annotation(Evaluate = true);
    parameter Real 'q' = 3;
    parameter Real 'r' = ('p' * 'q');
    Real 'x'(start = 0, fixed = true);
  equation
    der('x') = 'r';
  end 'EvaluateAnn';
",
        &[
            "this declaration has annotation(Evaluate = true), so parameter p is evaluated at translation and cannot be set (MLS 3.7 §18.3)",
        ],
    );
}

/// A `final` parameter with an evaluable binding is marked; a `constant`
/// stays `constant`, unannotated.
#[test]
fn a_final_evaluable_parameter_is_marked_and_a_constant_stays_constant() {
    assert_probe(
        include_str!("fixtures/structural_params/ConstFinal.mo"),
        "ConstFinal",
        "\
model 'ConstFinal'
    constant Real 'c' = 2;
    parameter Real 'k' = 3 annotation(Evaluate = true);
    parameter Real 'tt' = 1;
    Real 'x'(start = 0, fixed = true);
  equation
    der('x') = (('c' * 'k') * 'tt');
  end 'ConstFinal';
",
        &[],
    );
}

/// A replaceable class is chosen by `redeclare` (MLS §7.3), not by a
/// parameter: nothing is marked.
#[test]
fn a_replaceable_class_selection_marks_nothing() {
    assert_probe(
        include_str!("fixtures/structural_params/Replaceable.mo"),
        "Replaceable",
        "\
model 'Replaceable'
    Real 'law.f'(start = 0.0);
    Real 'law.u' = 'x';
    Real 'x'(start = 1, fixed = true);
  equation
    'law.f' = 'law.u';
    der('x') = (-'law.f');
  end 'Replaceable';
",
        &[],
    );
}

#[test]
fn a_redeclared_class_selection_marks_nothing() {
    assert_probe(
        include_str!("fixtures/structural_params/Replaceable.mo"),
        "ReplaceableB",
        "\
model 'ReplaceableB'
    Real 'law.f'(start = 0.0);
    Real 'law.u' = 'x';
    Real 'x'(start = 1, fixed = true);
  equation
    'law.f' = (2 * 'law.u');
    der('x') = (-'law.f');
  end 'ReplaceableB';
",
        &[],
    );
}

/// An ordinary parameter no structure reads stays tunable and unmarked.
#[test]
fn an_ordinary_parameter_stays_unmarked() {
    assert_probe(
        include_str!("fixtures/structural_params/TunableOnly.mo"),
        "TunableOnly",
        "\
model 'TunableOnly'
    parameter Real 'tau' = 1;
    Real 'x'(start = 1, fixed = true);
  equation
    der('x') = (-('x' / 'tau'));
  end 'TunableOnly';
",
        &[],
    );
}

/// The scalar target marks the parameter once, beside the elements it sized.
#[test]
fn the_scalar_target_marks_an_array_dimension_parameter() {
    let out = render(
        include_str!("fixtures/structural_params/ArraySize.mo"),
        "ArraySize",
        "base-modelica-scalar",
    );
    assert!(
        out.contains(
            "    parameter Integer 'n' = 3 annotation(Evaluate = true);\n    \
             Real 'x[1]'(start = 0, fixed = true);\n"
        ),
        "{out}"
    );
    assert_eq!(
        out.matches("annotation(Evaluate = true)").count(),
        1,
        "{out}"
    );
}

/// The marked output reads back through the compiler, for both targets.
#[test]
fn marked_output_recompiles_through_the_compiler() {
    for (source, model) in [
        (
            include_str!("fixtures/structural_params/ArraySize.mo"),
            "ArraySize",
        ),
        (
            include_str!("fixtures/structural_params/CondComp.mo"),
            "CondComp",
        ),
        (
            include_str!("fixtures/structural_params/EvaluateAnn.mo"),
            "EvaluateAnn",
        ),
    ] {
        for target in ["base-modelica", "base-modelica-scalar"] {
            let out = render(source, model, target);
            assert!(out.contains("annotation(Evaluate = true)"), "{out}");
            Compiler::new()
                .model(&format!("'{model}'.'{model}'"))
                .compile_str(&out, &format!("{model}.bmo"))
                .unwrap_or_else(|error| panic!("{target}/{model} recompiles: {error:#}\n{out}"));
        }
    }
}

/// A scalar argument does not make an array-valued function call scalar
/// (MLS §12.4, §10.1). Its start value must retain the function's result shape.
#[test]
fn an_array_function_start_value_keeps_its_return_shape() {
    let source = r#"
function init
  input Real seed;
  output Real result[2];
algorithm
  result := {seed, seed + 1.0};
end init;
model ArrayFunctionStart
  Real y[2](start = init(1.0));
equation
  der(y) = -y;
end ArrayFunctionStart;
"#;
    let out = render(source, "ArrayFunctionStart", "base-modelica");
    let declaration = out
        .lines()
        .find(|line| line.contains("Real 'y'[2]"))
        .unwrap_or_else(|| panic!("missing array declaration:\n{out}"));
    assert!(declaration.contains("start = 'init'(1.0)"), "{out}");
    assert!(!declaration.contains("fill("), "{out}");
    Compiler::new()
        .model("'ArrayFunctionStart'.'ArrayFunctionStart'")
        .compile_str(&out, "ArrayFunctionStart.bmo")
        .unwrap_or_else(|error| panic!("array function start recompiles: {error:#}\n{out}"));
}
