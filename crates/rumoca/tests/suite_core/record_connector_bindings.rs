//! A declaration equation on a component whose type is a connector that only
//! specializes a record (MLS §4.5.1 short class definition, MLS §8.3.1 record
//! equation).
//!
//! MSL declares `connector ComplexOutput = output Complex` and binds such
//! outputs directly, e.g. `ComplexOutput i[2] = electroMagneticConverter.i` in
//! `Magnetic.QuasiStatic.FundamentalWave.BasicMachines.Components.
//! SaliencyCageWinding`. Instantiation projected a record binding onto the
//! fields only when the component's class was a `record`, so the connector's
//! binding was dropped and every field became an unknown without an equation:
//! ED001 "unbalanced model", balance -2 per scalar `Complex` output.

use rumoca::Compiler;
use rumoca_sim::{SimOptions, eval_dae_at};

const SOURCE: &str = r#"
within;
record Pair
  Real a;
  Real b;
end Pair;
connector PairOutput = output Pair;
connector PairSignal = PairOutput;
block Source
  PairOutput y;
equation
  y = Pair(2*time, 3);
end Source;
model RecordConnectorBindings
  Source source;
  Pair x[2];
  PairOutput out = source.y;
  PairSignal chained = Pair(time, -1);
  PairOutput outs[2] = x;
equation
  x[1] = Pair(1, 2);
  x[2] = Pair(3, 4);
end RecordConnectorBindings;
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
fn a_record_connector_declaration_equation_binds_every_field() {
    let compiled = Compiler::new()
        .model("RecordConnectorBindings")
        .compile_str(SOURCE, "RecordConnectorBindings.mo")
        .expect("model should be balanced and compile to a checked DAE");
    let probe = eval_dae_at(&compiled.dae, &SimOptions::default(), &[], 0.5)
        .expect("checked DAE should evaluate");
    let report = probe.report;
    assert!(report.error.is_none(), "eval error: {:?}", report.error);
    for (name, expected) in [
        ("out.a", 1.0),
        ("out.b", 3.0),
        ("chained.a", 0.5),
        ("chained.b", -1.0),
        ("outs[1].a", 1.0),
        ("outs[2].b", 4.0),
    ] {
        let found = value(&report, name);
        assert!(
            (found - expected).abs() < 1e-12,
            "{name}: expected {expected}, found {found}"
        );
    }
}
