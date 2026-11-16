//! Composed array attributes keep a positive rank proof (MLS §7.2.5,
//! §10.4 and §10.6), without guessing a comprehension element's exact rank.

use super::{recompiles, render_ok};

#[test]
fn composed_comprehension_attributes_export_and_recompile() {
    for value in [
        "2.0*C",
        "C*2.0",
        "C/2",
        "C+b",
        "C .* b",
        "C .+ 1",
        "C .^ 1",
        "sin(C)",
        "abs(C)",
        "noEvent(C)",
        "if p then C else b",
        "2*{i for i in 1:3}",
        "({{a[i]*j for j in 1:3} for i in 1:1})[1]",
        "C-b",
        // Neighbours: both operand orders, other elementwise operators,
        // nested wrappers, scalar-returning vectorized calls and selections.
        "b+C",
        "b-C",
        "b .* C",
        "C .- 1",
        "1 .+ C",
        "C ./ 2",
        "2 .* C",
        "2 .^ C",
        "sin(2*C+b)",
        "if p then 2*C else sin(C)",
        "if p then b else C",
        "f(C)",
        "f(2*C)+b",
        "min(C,b)",
        "max(C,b)",
        "smooth(1,C)",
        "homotopy(C,b)",
        "(2*C)[1:3]",
        "-(2*C)",
        "vector(2*C)",
        "{{a[i]*j for j in 1:3} for i in 1:3}*b",
        "b*{{a[i]*j for j in 1:3} for i in 1:3}",
    ] {
        for attribute in ["start", "min", "max", "nominal"] {
            check_attribute("[3]", value, attribute);
        }
    }
}

#[test]
fn composed_multidimensional_comprehensions_keep_selected_axes() {
    for (dims, value) in [
        ("[3,2]", "2*{{a[i],2*a[i]} for i in 1:3}"),
        ("[2,3]", "sin({a[i]*j for i in 1:3, j in 1:2})"),
        ("[2,3]", "transpose({{a[i],2*a[i]} for i in 1:3})"),
        ("[3]", "(sin({{a[i]*j for j in 1:3} for i in 1:2}))[1,:]"),
        ("[3]", "(2*{{{a[i],2*a[i]}} for i in 1:3})[:,1,1]"),
    ] {
        check_attribute(dims, value, "start");
    }
}

#[test]
fn comprehension_reductions_still_replicate_scalar_attributes() {
    for value in ["sum(C)", "product(C)", "min(C)", "max(C)"] {
        let value = value.replace('C', "{a[i] for i in 1:3}");
        let source = format!(
            "model Reduced parameter Real a[3]={{1,2,3}};
             Real x[3](start={value}); equation der(x)=-x; end Reduced;"
        );
        let out = render_ok(&source, "Reduced", "base-modelica");
        assert!(out.contains("start = fill("), "{out}");
        recompiles(&out, "Reduced");
    }
}

fn check_attribute(dims: &str, value: &str, attribute: &str) {
    let value = value.replace('C', "{a[i] for i in 1:3}");
    let value = if attribute == "nominal" {
        format!("abs({value}) .+ 1")
    } else {
        value
    };
    let source = format!(
        "function f input Real u; output Real y; algorithm y:=2*u; end f;
         model Composed
           parameter Real a[3]={{1,2,3}};
           parameter Real b[3]={{0.1,0.2,0.3}};
           parameter Boolean p=true;
           Real x{dims}({attribute}={value});
         equation der(x)=-x;
         end Composed;"
    );
    let out = render_ok(&source, "Composed", "base-modelica");
    assert!(
        !out.contains(&format!("{attribute} = fill(")),
        "{value}: {out}"
    );
    assert!(out.contains(&format!("{attribute} = ")), "{value}: {out}");
    recompiles(&out, "Composed");
}
