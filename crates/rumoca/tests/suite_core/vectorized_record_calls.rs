//! A function with scalar inputs and an operator-record result applied to
//! vector arguments (MLS 3.7 §12.4.6): the vector of its element calls.
//!
//! MSL `Modelica.ComplexMath.fromPolar` takes a scalar length and angle and
//! returns one `Complex`; `Magnetic.QuasiStatic.FundamentalWave.Utilities.VfController`
//! writes `y = fromPolar(fill(amplitude, m), orientation + ...)` for a
//! `Complex y[m]`, and `Electrical.QuasiStatic.Polyphase.Blocks.SingleToPolyphase`
//! writes `y = u*fromPolar(fill(1, m), -symmetricOrientation(m))`. The
//! operator-record pass typed such a call as one scalar record, so the
//! equation was restated with one residual per field instead of one per
//! element and field: ED001 "unbalanced model", -4 for `m = 3`.

use rumoca::Compiler;
use rumoca_sim::{SimOptions, eval_dae_at};

const SOURCE: &str = r#"
within;
operator record Cx
  Real re;
  Real im;
  encapsulated operator function '*'
    import Cx;
    input Cx a;
    input Cx b;
    output Cx c;
  algorithm
    c := Cx(a.re*b.re - a.im*b.im, a.re*b.im + a.im*b.re);
  end '*';
end Cx;
function polar
  input Real len;
  input Real phi;
  output Cx c;
algorithm
  c := Cx(len*cos(phi), len*sin(phi));
end polar;
function angles
  input Integer m;
  output Real phi[m];
algorithm
  phi := {0.5*k for k in 1:m};
end angles;
model VectorizedRecordCalls
  parameter Integer m = 3;
  parameter Real orientation[m] = {0, 1, 2};
  Cx u = Cx(2, 0);
  Cx y[m];
  Cx z[m];
equation
  y = polar(fill(time, m), orientation);
  z = u*polar(fill(1, m), -angles(m));
end VectorizedRecordCalls;
"#;

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
fn a_vectorized_record_call_equates_every_element() {
    let compiled = Compiler::new()
        .model("VectorizedRecordCalls")
        .compile_str(SOURCE, "VectorizedRecordCalls.mo")
        .expect("model should be balanced and compile to a checked DAE");
    let probe = eval_dae_at(&compiled.dae, &SimOptions::default(), &[], 0.5)
        .expect("checked DAE should evaluate");
    let report = probe.report;
    assert!(report.error.is_none(), "eval error: {:?}", report.error);
    for k in 1..=3 {
        let phi = (k - 1) as f64;
        let expected = [
            (format!("y[{k}].re"), 0.5 * phi.cos()),
            (format!("y[{k}].im"), 0.5 * phi.sin()),
            (format!("z[{k}].re"), 2.0 * (-0.5 * k as f64).cos()),
            (format!("z[{k}].im"), 2.0 * (-0.5 * k as f64).sin()),
        ];
        for (name, expected) in expected {
            let found = value(&report, &name);
            assert!(
                (found - expected).abs() < 1e-12,
                "{name}: expected {expected}, found {found}"
            );
        }
    }
}
