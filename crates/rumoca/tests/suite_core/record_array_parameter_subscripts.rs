//! A record-array element selected by an element of an Integer parameter array
//! (MLS §10.5): `z[idx[k]]` where `idx` is a parameter array.
//!
//! Flatten scalarizes a record array into one variable per element and field
//! (`'z[2].a'`), so the subscript has to fold to its translation-time value to
//! name one of them. `MSL Magnetic.QuasiStatic.FundamentalWave.Components.
//! PolyphaseElectroMagneticConverter` writes
//! `iSymmetricalComponent[indexNonPos[k]] = Complex(0, 0)` with `indexNonPos`
//! a final parameter array computed by a function. Before the fix only an
//! unsubscripted parameter (`z[i]`) folded; `z[idx[k]].a` stayed a projection
//! of a variable the flat model does not declare, which the DAE phase rejected
//! with ED020 "wrong DAE coordinate role" (or ED008), and which the
//! base-modelica target wrote out verbatim.

use rumoca::{Compiler, render_target_files};
use rumoca_sim::{SimOptions, eval_dae_at};

const SOURCE: &str = r#"
within;
record Pair
  Real a;
  Real b;
end Pair;
function upper
  input Integer m;
  output Integer r[m - 1];
algorithm
  r := 2:m;
end upper;
model RecordArrayParameterSubscripts
  parameter Integer m = 3;
  final parameter Integer idx[:] = upper(m);
  final parameter Integer first[1] = {1};
  Pair z[m];
  Real y;
equation
  z[first[1]] = Pair(time, 1);
  for k in 1:size(idx, 1) loop
    z[idx[k]].a = k;
    z[idx[k]].b = 10*k;
  end for;
  y = z[idx[2]].a + z[idx[1]].b;
end RecordArrayParameterSubscripts;
"#;

const MODEL: &str = "RecordArrayParameterSubscripts";

fn value(report: &rumoca_sim::EvalAtReport, name: &str) -> f64 {
    report
        .solver_y
        .iter()
        .find(|slot| slot.name.replace(' ', "") == name)
        .unwrap_or_else(|| {
            panic!(
                "missing solver value {name}; have: {:?}",
                report
                    .solver_y
                    .iter()
                    .map(|slot| slot.name.clone())
                    .collect::<Vec<_>>()
            )
        })
        .value
}

#[test]
fn parameter_array_elements_select_scalarized_record_elements() {
    let compiled = Compiler::new()
        .model(MODEL)
        .compile_str(SOURCE, "RecordArrayParameterSubscripts.mo")
        .expect("model should compile to a checked DAE");
    let probe = eval_dae_at(&compiled.dae, &SimOptions::default(), &[], 0.5)
        .expect("checked DAE should evaluate");
    let report = probe.report;
    assert!(report.error.is_none(), "eval error: {:?}", report.error);
    for (name, expected) in [
        ("z[2].a", 1.0),
        ("z[2].b", 10.0),
        ("z[3].a", 2.0),
        ("z[3].b", 20.0),
        ("y", 12.0),
    ] {
        let found = value(&report, name);
        assert!(
            (found - expected).abs() < 1e-12,
            "{name}: expected {expected}, found {found}"
        );
    }
}

#[test]
fn base_modelica_names_the_selected_element_variables() {
    let compiled = Compiler::new()
        .model(MODEL)
        .compile_str(SOURCE, "RecordArrayParameterSubscripts.mo")
        .expect("model should compile to a checked DAE");
    let files = render_target_files(&compiled, MODEL, "base-modelica", None)
        .expect("base-modelica should render");
    let out = &files[0].content;
    for expected in [
        "'z[2].a' = 1;",
        "'z[3].b' = (10 * 2);",
        "'y' = ('z[3].a' + 'z[2].b');",
    ] {
        assert!(out.contains(expected), "missing `{expected}` in:\n{out}");
    }
    assert!(!out.contains("['idx'"), "unfolded subscript in:\n{out}");
}
