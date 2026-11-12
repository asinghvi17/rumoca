//! A dynamic if-equation whose branches are operator-record equations
//! (MLS §8.3.4, §8.4): every field is one scalar equation in every branch.
//!
//! MSL `Magnetic.QuasiStatic.FundamentalWave.Components.EddyCurrent` writes
//! `if G > 0 then (pi/2)*V_m = j*omega*G*Phi; else V_m = Complex(0, 0); end if;`.
//! Flatten keeps a parameter-guarded if-equation as the residual
//! `if G > 0 then lhs1 - rhs1 else lhs2 - rhs2`, and the operator-record pass
//! resolved each branch to one `Complex`-valued difference without restating
//! the equation field by field, so it stated one equation for the record:
//! ED001 "unbalanced model", balance -1 for
//! `Magnetic.QuasiStatic.FundamentalWave.Examples.Components.EddyCurrentLosses`.

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
model ConditionalRecordEquations
  parameter Real G = 1;
  Cx y = Cx(time, 1);
  Cx x;
  Cx z;
equation
  if G > 0 then
    Cx(2, 0)*x = y;
  else
    x = Cx(0, 0);
  end if;
  if time > 1 then
    z = y;
  elseif G > 2 then
    Cx(0, 1)*z = y;
  else
    Cx(4, 0)*z = y;
  end if;
end ConditionalRecordEquations;
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
fn a_conditional_record_equation_equates_every_field() {
    let compiled = Compiler::new()
        .model("ConditionalRecordEquations")
        .compile_str(SOURCE, "ConditionalRecordEquations.mo")
        .expect("model should be balanced and compile to a checked DAE");
    let probe = eval_dae_at(&compiled.dae, &SimOptions::default(), &[], 0.5)
        .expect("checked DAE should evaluate");
    let report = probe.report;
    assert!(report.error.is_none(), "eval error: {:?}", report.error);
    for (name, expected) in [
        ("x.re", 0.25),
        ("x.im", 0.5),
        ("z.re", 0.125),
        ("z.im", 0.25),
    ] {
        let found = value(&report, name);
        assert!(
            (found - expected).abs() < 1e-12,
            "{name}: expected {expected}, found {found}"
        );
    }
}
