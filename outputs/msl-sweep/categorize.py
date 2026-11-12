#!/usr/bin/env python3
"""Merge the sweep's per-model rows into results.tsv and categorise failures.

    python3 -I categorize.py WORK models.txt OUTDIR

Reads WORK/emit/*.tsv + WORK/emitlog/*.log (rumoca, emit.sh) and
WORK/rows/*.tsv + WORK/jlog/*.log (Julia stages, run_sweep.py), writes
OUTDIR/results.tsv and OUTDIR/categories.tsv and prints the tables summary.md
is built from.

Every failure gets a normalised signature (identifiers, numbers and paths
removed) and a root-cause category from the ordered RULES below.
"""
import collections
import os
import re
import sys

STAGES = ["emit", "parse", "convert", "simplify", "init", "solve", "compare"]
JCOLS = [
    "model", "bmo", "stage_reached", "failed_stage", "status", "error_type", "error",
    "n_unknowns_raw", "n_unknowns", "n_observed", "solver", "retcode", "tspan",
    "n_signals", "n_found", "max_rel_err", "max_rel_err_strict", "worst_signal",
    "missing_signals", "compare_note",
    "t_parse", "t_convert", "t_simplify", "t_init", "t_solve", "t_compare",
]
ANSI = re.compile(r"\x1b\[[0-9;]*m")

# --------------------------------------------------------------------- rumoca


def rumoca_message(log):
    """The first error diagnostic of a rumoca log, unwrapped onto one line."""
    text = ANSI.sub("", log)
    lines = text.split("\n")
    start = None
    for i, l in enumerate(lines):
        if re.search(r"rumoca::\S+::[A-Z]+\d+\s*$", l.strip()) or re.match(r"\s*(×\s*)?\[E[A-Z]*\d+\]", l):
            start = i
            break
    if start is None:
        for i, l in enumerate(lines):
            if re.search(r"\berror\b", l, re.I) and "warning" not in l:
                start = i
                break
    if start is None:
        nonblank = [l for l in lines if l.strip()]
        return nonblank[-1].strip() if nonblank else "(empty log)"
    head = lines[start].strip().lstrip("×").strip()
    msg = []
    first = head if not re.fullmatch(r"rumoca::\S+", head) else ""
    code = re.search(r"(rumoca::\S+::[A-Z]+\d+|\[E[A-Z]*\d+\])", head)
    if first:
        msg.append(first)
    for l in lines[start + 1:]:
        s = l.strip()
        if s.startswith("╭") or s.startswith("help:") or re.match(r"^\d+\s*│", s):
            break
        s = s.lstrip("│×").strip()
        if s:
            msg.append(s)
    out = " ".join(msg)
    out = re.sub(r"- (\w)", r"-\1", out)  # undo hyphenation at wraps: "base- modelica"
    if code and code.group(1) not in out:
        out = code.group(1) + " " + out
    ctx = template_context(text)
    return out + (f" [in {ctx}]" if ctx else "")


TEMPLATE_SITES = [
    ("param.shape_expr", "function parameter dimensions"),
    ("branch.condition", "when condition"),
    ("assert(", "assert"),
    ("value is not none", "declaration binding/start"),
    ("algorithm", "algorithm statement"),
    ("residual", "equation"),
    ("eq.", "equation"),
]


def template_context(text):
    """Which part of the base-modelica template was rendering when it failed."""
    m = re.search(r"╭─\[inline:(\d+)", text)
    if not m:
        return ""
    n = m.group(1)
    line = re.search(r"^\s*" + n + r"\s*│(.*)$", text, re.M)
    src = line.group(1) if line else ""
    for key, tag in TEMPLATE_SITES:
        if key in src:
            return tag
    return f"template line {n}"


def rumoca_inner(msg):
    """Strip the template wrapper layers down to the innermost reason."""
    m = re.sub(r"^rumoca::codegen::EC002\s*", "", msg)
    m = re.sub(r"template render error:\s*", "", m)
    for _ in range(6):
        m2 = re.sub(r"^could not render include: error in \"[^\"]*\" \(in [^)]*\):\s*", "", m)
        m2 = re.sub(r"^invalid operation:\s*", "", m2)
        m2 = re.sub(r"^flat residual equation (LHS|RHS) failed to render:\s*", "", m2)
        m2 = re.sub(r"^If expression invalid \w+:\s*", "", m2)
        if m2 == m:
            break
        m = m2
    return m


def normalise(s):
    s = re.sub(r"\(in (inline|[\w.-]+\.jinja):\d+\)", "", s)
    ctx = re.search(r" \[in [^\]]*\]$", s)
    tail = ctx.group(0) if ctx else ""
    s = s[: len(s) - len(tail)]
    s = re.sub(r"\{\s*\"(\w+)\".*", r"\1", s)  # JSON dumps of expressions -> variant name
    s = re.sub(r"`[^`]*`", "`…`", s)
    s = re.sub(r"'[^']*'", "'…'", s)
    s = re.sub(r"\"[^\"]*\"", "\"…\"", s)
    s = re.sub(r"/\S+", "<path>", s)
    s = re.sub(r"\b\d+(\.\d+)?(e[-+]?\d+)?\b", "N", s)
    s = re.sub(r"\s+", " ", s).strip()
    return s[:200] + tail


# ------------------------------------------------------------------ categories
# (stage regex, signature regex, category, owner). First match wins.
RULES = [
    # --- rumoca
    ("emit", r"\[in function parameter dimensions\]", "rumoca: function array-parameter dimensions ([:], size(...)) not rendered", "rumoca"),
    ("emit", r"variant:? StringConversion", "rumoca: String() conversion (assert/terminate messages)", "rumoca"),
    ("emit", r"BuiltinCall (Clock|SuperSample|SubSample|ShiftSample|BackSample|Hold|Previous|Interval|NoClock|FirstTick)\b|clock interval|cannot scalarize `…`.*[Cc]lock|ED018|ED009|clock schedule", "Clock/synchronous features (Clock, superSample, hold, ...)", "rumoca"),
    ("emit", r"BuiltinCall Sample", "rumoca: sample() not lowered (event sample(t0,dt) and clocked sample)", "rumoca"),
    ("emit", r"variant:? Colon", "rumoca: `:` slice (Colon) in expressions", "rumoca"),
    ("emit", r"Index missing", "rumoca: Index on a non-reference expression (missing base)", "rumoca"),
    ("emit", r"variant:? (\w+)", "rumoca: unhandled Expression variant", "rumoca"),
    ("emit", r"record-value|subscripted record path", "rumoca: record-valued equations/paths (Complex, records)", "rumoca"),
    ("emit", r"cannot scalarize|array_element|is not lowerable", "rumoca: base-modelica-scalar cannot scalarize", "rumoca"),
    ("emit", r"ER130|non-constant variable .* lexically", "rumoca: ER130 non-constant outer reference in nested function", "rumoca"),
    ("emit", r"ER002|unresolved function call", "rumoca: ER002 unresolved (replaceable package) function", "rumoca"),
    ("emit", r"ED019.*assigns a value of shape", "rumoca: ED019 shape check fires in function branches the specialization never takes", "rumoca"),
    ("emit", r"ED019.*as a statement without reading", "rumoca: ED019 impure call statement (error/print) in a function", "rumoca"),
    ("emit", r"ED019.*der\(\.\.\.\) must have exactly one", "rumoca: ED019 der() of an expression", "rumoca"),
    ("emit", r"ED019|semantic owner", "rumoca: ED019 other unsupported Flat construct (function arrays/records/loops)", "rumoca"),
    ("emit", r"ED013|algorithm in canonical DAE", "rumoca: ED013 model algorithm with loops/break/multi-output (Digital)", "rumoca"),
    ("emit", r"EF036|foreign library|external", "External C functions", "rumoca"),
    ("emit", r"EF020|expandable connector", "rumoca: expandable connector augmentation", "rumoca"),
    ("emit", r"ET004|unevaluable array dimensions", "rumoca: unevaluable array dimensions", "rumoca"),
    ("emit", r"timeout", "rumoca: timeout", "rumoca"),
    ("emit", r".", "rumoca: other", "rumoca"),
    # --- Julia stages (raw text includes the head of the jlog)
    ("parse", r"syntax error\(s\) line \d+, column \d+: (missing ';' at '\.'|mismatched input '\.')", "rumoca: member access on a function-call result `f(...).'x'` is not Base Modelica (record-valued calls)", "rumoca"),
    ("parse", r"missing ';' at '\['", "rumoca: subscripted array literal [..][i,j] is not Base Modelica (Digital logic tables)", "rumoca"),
    ("parse", r"RecursionError", "BaseModelica.jl: ANTLR (Python) recursion limit on deeply nested expressions", "BaseModelica.jl"),
    ("parse", r".", "BaseModelica.jl: other parse error", "BaseModelica.jl"),
    ("convert", r"no method matching Float64\(::Num\)", "BaseModelica.jl: sample(start, interval) with parameter-valued arguments (events.jl sample_args)", "BaseModelica.jl"),
    ("convert", r"no method matching fill\(::Num, ::Num\)", "BaseModelica.jl: fill() sized by a parameter", "BaseModelica.jl"),
    ("convert", r"initial algorithm sections may only contain assignments", "BaseModelica.jl: assert() in initial algorithm rejected", "BaseModelica.jl"),
    ("convert", r"array variable", "BaseModelica.jl: array variables unsupported (array-preserving output)", "BaseModelica.jl"),
    ("convert", r"function `…` is not supported", "BaseModelica.jl: event operators initial()/terminal()/edge() unsupported", "BaseModelica.jl"),
    ("convert", r"return is not supported", "BaseModelica.jl: `return` in functions unsupported", "BaseModelica.jl"),
    ("convert", r"zeros\(::Num\)", "BaseModelica.jl: array sized by a parameter", "BaseModelica.jl"),
    ("*", r"Bindings for parameters can only be functions|UndefVarError: `…` not defined in `ModelingToolkitBase`", "BaseModelica.jl: parameter bindings intermittently lose referenced parameters (nondeterministic)", "BaseModelica.jl"),
    ("simplify", r"is present in the system but .* is not an unknown", "delay() operator (needs a DDE / history)", "BaseModelica.jl/MTK"),
    ("simplify", r"cycle|loop", "MTK structural simplification: cycle/loop error", "MTK"),
    ("solve", r"get_initial_values|Cannot `convert` an object of type Nothing", "MTK: initialization internal error", "MTK"),
    ("solve", r"DtNaN at t=N\.N |DtNaN at t=0", "init/solve: NaN at t=0 (initial values)", "BaseModelica.jl/MTK"),
    ("solve", r"InitialFailure", "initialisation failure", "MTK"),
    ("*", r"Timeout", "timeout (5 min stage cap)", "?"),
    ("solve", r"retcode", "solver failure (Unstable/MaxIters)", "solver"),
]

# Failures diagnosed by hand (see summary.md): the compare stage only knows
# that the numbers differ. CharacteristicIdealDiodes solved in the first batch
# (no dtmax) with the wrong diode mode; with dtmax it chatters past the cap.
DIAGNOSED = {
    "Modelica.Blocks.Examples.BusUsage": "BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0)",
    "Modelica.Thermal.HeatTransfer.Examples.ControlledTemperature": "BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0)",
    "Modelica.Electrical.Analog.Examples.CharacteristicIdealDiodes": "BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0)",
    "Modelica.Mechanics.Translational.Examples.ElastoGap": "BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0)",
    "Modelica.Electrical.Analog.Examples.OpAmps.SignalGenerator": "initialisation picks another root of the nonlinear init system (start values / homotopy)",
    "Modelica.Electrical.Analog.Examples.DemonstrateLightning": "initialisation picks another root of the nonlinear init system (start values / homotopy)",
    "Modelica.Electrical.Machines.Examples.DCMachines.DCPM_Drive": "numerical mismatch: early transient differs (undiagnosed)",
    "Modelica.Mechanics.Rotational.Examples.CompareBrakingTorque": "numerical mismatch: borderline, 1.04e-3 against the 1e-3 threshold (friction stick events)",
    "Modelica.Mechanics.Translational.Examples.CompareBrakingForce": "numerical mismatch: borderline, 1.04e-3 against the 1e-3 threshold (friction stick events)",
    "Modelica.Mechanics.Translational.Examples.PreLoad": "numerical mismatch: stick/slip friction transient, 1.5% (tolerance-level)",
}


def categorise(stage, sig, raw, model=""):
    if model in DIAGNOSED:
        cat = DIAGNOSED[model]
        return cat, cat.split(":")[0] if cat.startswith("BaseModelica") else "BaseModelica.jl/MTK"
    text = sig + " " + raw
    for st, rx, cat, owner in RULES:
        if (st == stage or st == "*") and re.search(rx, text):
            return cat, owner
    return f"{stage}: other", {"parse": "BaseModelica.jl", "convert": "BaseModelica.jl",
                               "simplify": "MTK/DyadInterface", "init": "MTK",
                               "solve": "solver", "compare": "?"}.get(stage, "?")


# --------------------------------------------------------------------- driver


def read_tsv_row(path, ncols=None):
    with open(path, encoding="utf-8", errors="replace") as fh:
        f = fh.read().rstrip("\n").split("\t")
    if ncols:
        f += [""] * (ncols - len(f))
    return f


def sublib(model):
    p = model.split(".")
    return p[1], ".".join(p[1:3]) if p[2] != "Examples" else p[1]


def main():
    work, models_file, outdir = sys.argv[1:4]
    with open(models_file) as fh:
        models = [l.strip() for l in fh if l.strip()]
    rows = []
    for m in models:
        r = {"model": m}
        r["sublib"], r["sublib2"] = sublib(m)
        emit = {}
        for target in ("base-modelica", "base-modelica-scalar"):
            p = os.path.join(work, "emit", f"{m}.{target}.tsv")
            if not os.path.exists(p):
                emit[target] = None
                continue
            _, _, status, secs, sig = read_tsv_row(p, 5)[:5]
            log = ""
            lp = os.path.join(work, "emitlog", f"{m}.{target}.log")
            if os.path.exists(lp):
                with open(lp, errors="replace") as fh:
                    log = fh.read()
            msg = rumoca_inner(rumoca_message(log)) if status != "ok" else ""
            if status == "timeout":
                msg = "timeout 300s"
            emit[target] = (status, secs, msg)
        a, s = emit["base-modelica"], emit["base-modelica-scalar"]
        r["emit_array"] = a[0] if a else "not-run"
        r["emit_scalar"] = s[0] if s else "not-run"
        r["emit_array_err"] = normalise(a[2]) if a and a[0] != "ok" else ""
        r["emit_scalar_err"] = normalise(s[2]) if s and s[0] != "ok" else ""
        r["t_emit"] = (s or a or ("", "", ""))[1]
        jp = os.path.join(work, "rows", f"{m}.tsv")
        if a is None and s is None:
            r.update(stage_reached="", failed_stage="emit", status="not-run")
        elif (a is None or a[0] != "ok") and (s is None or s[0] != "ok"):
            src = a if a and a[0] != "not-run" else s
            r.update(stage_reached="none", failed_stage="emit", status="fail",
                     signature=normalise(src[2]), error=src[2][:300])
        elif os.path.exists(jp):
            j = dict(zip(JCOLS, read_tsv_row(jp, len(JCOLS))))
            r.update({k: j[k] for k in JCOLS[1:]})
            r["variant"] = "scalar" if "scalar" in r["emit_scalar"] and r["emit_scalar"] == "ok" else "array"
            r["signature"] = normalise(j["error_type"] + ": " + j["error"]) if j["status"] == "fail" else ""
            lp = os.path.join(work, "jlog", f"{m}.log")
            if j["status"] == "fail" and os.path.exists(lp):
                with open(lp, errors="replace") as fh:
                    r["jlog"] = " ".join(fh.read(1500).split())
        else:
            r.update(stage_reached="emit", failed_stage="", status="pending")
        if r["status"] == "fail":
            r["category"], r["owner"] = categorise(r["failed_stage"], r.get("signature", ""),
                                                  r.get("error", "") + " " + r.get("jlog", ""), m)
        rows.append(r)

    cols = ["model", "sublib", "sublib2", "stage_reached", "failed_stage", "status", "category",
            "owner", "signature", "max_rel_err", "max_rel_err_strict", "worst_signal", "n_signals",
            "n_found", "missing_signals", "emit_array", "emit_scalar", "emit_array_err",
            "emit_scalar_err", "bmo", "n_unknowns_raw", "n_unknowns", "solver", "retcode", "tspan",
            "compare_note", "t_emit", "t_parse", "t_convert", "t_simplify", "t_init", "t_solve",
            "t_compare", "error"]
    with open(os.path.join(outdir, "results.tsv"), "w") as fh:
        fh.write("\t".join(cols) + "\n")
        for r in rows:
            fh.write("\t".join(str(r.get(c, "")).replace("\t", " ").replace("\n", " ") for c in cols) + "\n")

    # ---- tables
    n = len(rows)
    print(f"models: {n}")
    order = ["none"] + STAGES
    reached_at_least = collections.Counter()
    for r in rows:
        if r["status"] in ("pending", "not-run"):
            continue
        if r["status"].startswith("pass"):
            done = STAGES
        else:
            fs = r["failed_stage"]
            done = STAGES[: STAGES.index(fs)] if fs in STAGES else []
        for st in done:
            reached_at_least[st] += 1
    print("\n## stage funnel (models that completed each stage)")
    for st in STAGES:
        c = reached_at_least[st]
        print(f"| {st} | {c} | {100 * c / n:.1f}% |")
    st = collections.Counter(r["status"] for r in rows)
    print("\nstatus:", dict(st))

    cats = collections.Counter()
    owners = {}
    examples = collections.defaultdict(list)
    for r in rows:
        if r["status"] == "fail":
            key = (r["failed_stage"], r["category"])
            cats[key] += 1
            owners[key] = r["owner"]
            examples[key].append(r["model"])
    with open(os.path.join(outdir, "categories.tsv"), "w") as fh:
        fh.write("failed_stage\tcategory\towner\tcount\texample_models\n")
        for (stg, cat), c in cats.most_common():
            fh.write(f"{stg}\t{cat}\t{owners[(stg, cat)]}\t{c}\t{', '.join(examples[(stg, cat)][:4])}\n")
    print("\n## categories")
    for (stg, cat), c in cats.most_common():
        print(f"| {stg} | {cat} | {owners[(stg, cat)]} | {c} | {examples[(stg, cat)][0]} |")

    print("\n## per sub-library")
    print("| sub-library | models | emit | parse | convert | simplify | solve | pass | top failure (count) |")
    print("|---|---|---|---|---|---|---|---|---|")
    groups = collections.defaultdict(list)
    for r in rows:
        groups[r["sublib"]].append(r)
    for lib, rs in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        def through(stage):
            k = STAGES.index(stage)
            return sum(1 for r in rs if r["status"].startswith("pass") or
                       (r["status"] == "fail" and r["failed_stage"] in STAGES and STAGES.index(r["failed_stage"]) > k))
        top = collections.Counter(r.get("category", "") for r in rs if r["status"] == "fail").most_common(1)
        topc = f"{top[0][0]} ({top[0][1]})" if top else ""
        print(f"| {lib} | {len(rs)} | {through('emit')} | {through('parse')} | {through('convert')} | "
              f"{through('simplify')} | {through('solve')} | "
              f"{sum(1 for r in rs if r['status'].startswith('pass'))} | {topc} |")

    print("\n## emit by target")
    for col in ("emit_array", "emit_scalar"):
        print(col, dict(collections.Counter(r[col] for r in rows)))
    both = collections.Counter((r["emit_array"], r["emit_scalar"]) for r in rows)
    print("(array, scalar):", dict(both))

if __name__ == "__main__":
    main()
