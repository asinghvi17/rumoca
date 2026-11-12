# Long-lived worker for the MSL sweep: runs the Julia stages for one model at a
# time so package loading and compilation are paid once per worker, not per model.
#
#   julia --project=outputs/msl-sweep/julia worker.jl WARMUP.bmo
#
# Reads jobs from stdin, one per line:
#   MODEL \t BMO_FILE \t REF_DIR (or -) \t ROW_FILE \t LOG_FILE
# and for each job prints `@@STAGE <name>` before every stage, then writes one
# TSV row to ROW_FILE and prints `@@DONE`. The driver (run_sweep.py) enforces the
# per-stage and per-model timeouts by killing this process; a stage that never
# reports back is recorded as a timeout there.
#
# Stages: parse -> convert -> simplify -> init (ODEProblem) -> solve -> compare.

using BaseModelica
using DyadInterface
using ModelingToolkit
using OrdinaryDiffEq
using SciMLBase
using DelimitedFiles

const MTKBase = BaseModelica.ModelingToolkitBase

stage(s) = (println(stdout, "@@STAGE ", s); flush(stdout))

clean(s) = replace(String(s), r"\e\[[0-9;]*m" => "", '\t' => ' ', '\r' => ' ')
firstline(s) = clean(first(split(strip(clean(s)), '\n'; limit = 2)))

function errtext(e, bt)
    msg = clean(sprint(showerror, e))
    frames = String[]
    for fr in stacktrace(bt)
        f = string(fr.file)
        if occursin(r"BaseModelica|ModelingToolkit|DyadInterface|SciMLBase|OrdinaryDiffEq|Symbolic", f)
            push!(frames, string(fr.func, " @ ", basename(f), ":", fr.line))
        end
        length(frames) >= 8 && break
    end
    return msg, frames
end

"Map reference signal name => getter over (sol, times)."
function signal_table(sys)
    tab = Dict{String, Any}()
    for v in ModelingToolkit.unknowns(sys)
        tab[string(ModelingToolkit.getname(v))] = v
    end
    for eq in ModelingToolkit.observed(sys)
        tab[string(ModelingToolkit.getname(eq.lhs))] = eq.lhs
    end
    for p in ModelingToolkit.parameters(sys)
        n = string(ModelingToolkit.getname(p))
        haskey(tab, n) || (tab[n] = p)
    end
    return tab
end

function read_reference(refdir, model)
    csv = joinpath(refdir, split(model, '.')[end] * ".csv")
    sigf = joinpath(refdir, "comparisonSignals.txt")
    isfile(csv) && isfile(sigf) || return nothing
    data, header = readdlm(csv, ','; header = true)
    header = [strip(String(h), '"') for h in vec(header)]
    signals = [strip(l) for l in eachline(sigf) if !isempty(strip(l)) && strip(l) != "time"]
    return (; t = Float64.(data[:, 1]), header, data, signals)
end

function values_at(sol, sym, ts)
    try
        return Float64.(collect(sol(ts; idxs = sym).u))
    catch
        v = Float64(sol.ps[sym])
        return fill(v, length(ts))
    end
end

"""
Max error over reference samples in the time overlap, relative to the reference
signal's range. `strict` evaluates the solution at the sample time; `tube` takes
the best of t and t ± 1e-3·span, which forgives event-time jitter on
discontinuous signals. Samples at duplicated reference times (event left/right
limits) are skipped.
"""
function compare(sol, sys, ref)
    tab = signal_table(sys)
    t0, t1 = max(ref.t[1], sol.t[1]), min(ref.t[end], sol.t[end])
    dup = falses(length(ref.t))
    for i in 2:length(ref.t)
        if ref.t[i] == ref.t[i - 1]
            dup[i] = dup[i - 1] = true
        end
    end
    keep = [i for i in eachindex(ref.t) if t0 <= ref.t[i] <= t1 && !dup[i]]
    ts = ref.t[keep]
    δ = 1.0e-3 * (t1 - t0)
    worst_strict, worst_tube, worst_sig = 0.0, 0.0, ""
    missing_sigs = String[]
    nfound = 0
    for s in ref.signals
        col = findfirst(==(s), ref.header)
        sym = get(tab, s, nothing)
        if col === nothing || sym === nothing
            push!(missing_sigs, s)
            continue
        end
        r = Float64.(ref.data[keep, col])
        local a, lo, hi
        try
            a = values_at(sol, sym, ts)
            lo = values_at(sol, sym, clamp.(ts .- δ, t0, t1))
            hi = values_at(sol, sym, clamp.(ts .+ δ, t0, t1))
        catch
            push!(missing_sigs, s)
            continue
        end
        nfound += 1
        rng = maximum(r; init = -Inf) - minimum(r; init = Inf)
        denom = rng > 0 ? rng : max(maximum(abs, r; init = 0.0), 1.0e-12)
        e_strict = maximum(abs.(a .- r); init = 0.0) / denom
        e_tube = maximum(min.(abs.(a .- r), abs.(lo .- r), abs.(hi .- r)); init = 0.0) / denom
        isfinite(e_strict) || (e_strict = Inf)
        if e_tube > worst_tube || (e_tube == worst_tube && e_strict > worst_strict)
            worst_sig = s
        end
        worst_strict = max(worst_strict, e_strict)
        worst_tube = max(worst_tube, e_tube)
    end
    return (; nsig = length(ref.signals), nfound, missing_sigs, worst_strict, worst_tube,
        worst_sig, overlap = (t0, t1), refend = ref.t[end])
end

const COLS = [
    "model", "bmo", "stage_reached", "failed_stage", "status", "error_type", "error",
    "n_unknowns_raw", "n_unknowns", "n_observed", "solver", "retcode", "tspan",
    "n_signals", "n_found", "max_rel_err", "max_rel_err_strict", "worst_signal",
    "missing_signals", "compare_note",
    "t_parse", "t_convert", "t_simplify", "t_init", "t_solve", "t_compare",
]

function run_job(model, bmo, refdir, rowfile, logfile)
    row = Dict{String, Any}(c => "" for c in COLS)
    row["model"] = model
    row["bmo"] = basename(bmo)
    reached = "emit"
    cur = "parse"
    log = IOBuffer()
    timed(f, name) = begin
        stage(name)
        cur = name
        t = time()
        v = f()
        row["t_" * name] = round(time() - t; digits = 2)
        reached = name
        v
    end
    try
        pkg = timed(() -> BaseModelica.parse_package(bmo, :antlr), "parse")
        sys = timed("convert") do
            BaseModelica.baseModelica_to_ModelingToolkit(pkg; discrete_backend = :mtk, simplify = false)
        end
        row["n_unknowns_raw"] = length(ModelingToolkit.unknowns(sys))
        exp = BaseModelica.experiment_settings(sys)
        tstops = sort!(collect(BaseModelica.tstops_collection))
        ssys = timed(() -> DyadInterface.simplify_model(sys), "simplify")
        row["n_unknowns"] = length(ModelingToolkit.unknowns(ssys))
        row["n_observed"] = length(ModelingToolkit.observed(ssys))
        tspan = exp === nothing ? (0.0, 1.0) : (Float64(exp.StartTime), Float64(exp.StopTime))
        tol = exp === nothing ? 1.0e-4 : Float64(exp.Tolerance)
        row["tspan"] = string(tspan)
        prob = timed("init") do
            ODEProblem(ssys, [], tspan;
                missing_guess_value = MTKBase.MissingGuessValue.Constant(0.0))
        end
        # Tighter than most experiment tolerances so the comparison measures the
        # translation, not the integrator.
        reltol = min(tol, 1.0e-6)
        # Cap the step at the output interval, as Modelica tools do with their
        # output grid. Without it a system with few or no differential states is
        # stepped across the whole span and its algebraic signals are only
        # linearly interpolated between a handful of points.
        interval = exp === nothing || exp.Interval === nothing ? (tspan[2] - tspan[1]) / 500 : Float64(exp.Interval)
        row["tspan"] = string(tspan, " dtmax=", interval)
        ref = refdir == "-" ? nothing : read_reference(refdir, model)
        # For systems with no differential states also step exactly onto the
        # reference sample times: Rosenbrock's dense output of algebraic variables
        # is not error-controlled there, so the comparison reads step values.
        mm = prob.f.mass_matrix
        nodiff = prob.u0 === nothing || isempty(prob.u0) || (mm isa AbstractMatrix && all(iszero, mm[i, i] for i in axes(mm, 1)))
        row["compare_note"] = nodiff ? "no differential states" : ""
        reftimes = ref === nothing || !nodiff ? Float64[] : ref.t
        stops = unique!(sort!(filter(x -> tspan[1] < x < tspan[2], vcat(tstops, reftimes))))
        sol = timed("solve") do
            kw = (; reltol, abstol = 1.0e-8, tstops = stops, maxiters = 10^7, dtmax = interval)
            s = solve(prob, Rodas5P(); kw...)
            row["solver"] = "Rodas5P"
            if !SciMLBase.successful_retcode(s) && s.retcode != SciMLBase.ReturnCode.InitialFailure
                println(log, "Rodas5P retcode ", s.retcode, " at t=", s.t[end], "; retrying FBDF")
                s2 = solve(prob, FBDF(); kw...)
                if SciMLBase.successful_retcode(s2) || s2.t[end] > s.t[end]
                    s = s2
                    row["solver"] = "FBDF"
                end
            end
            row["retcode"] = string(s.retcode)
            if !SciMLBase.successful_retcode(s)
                error("solver retcode $(s.retcode) at t=$(s.t[end]) of $(tspan[2])")
            end
            s
        end
        if ref === nothing
            reached = "solve"
            row["failed_stage"] = "compare"
            row["status"] = "fail"
            row["error_type"] = "NoReference"
            row["error"] = "no reference CSV/comparisonSignals"
        else
            c = timed(() -> compare(sol, ssys, ref), "compare")
            row["n_signals"] = c.nsig
            row["n_found"] = c.nfound
            row["max_rel_err"] = c.worst_tube
            row["max_rel_err_strict"] = c.worst_strict
            row["worst_signal"] = c.worst_sig
            row["missing_signals"] = join(first(c.missing_sigs, 6), ",") *
                (length(c.missing_sigs) > 6 ? ",…" : "")
            c.refend < tspan[2] && (row["compare_note"] = strip(row["compare_note"] * " reference ends at $(c.refend)"))
            if c.nfound == 0
                row["status"] = "fail"
                row["failed_stage"] = "compare"
                row["error_type"] = "SignalsNotFound"
                row["error"] = "none of $(c.nsig) comparison signals found"
                reached = "solve"
            elseif c.worst_tube <= 1.0e-3 && isempty(c.missing_sigs)
                row["status"] = "pass"
            elseif c.worst_tube <= 1.0e-3
                row["status"] = "pass-partial"
                row["error_type"] = "SignalsMissing"
                row["error"] = "$(length(c.missing_sigs)) of $(c.nsig) comparison signals not found"
            else
                row["status"] = "fail"
                row["failed_stage"] = "compare"
                row["error_type"] = "NumericalMismatch"
                row["error"] = "max rel err $(round(c.worst_tube; sigdigits = 3)) on $(c.worst_sig)"
                reached = "solve"
            end
        end
    catch e
        bt = catch_backtrace()
        msg, frames = errtext(e, bt)
        row["failed_stage"] = cur
        row["status"] = "fail"
        row["error_type"] = string(nameof(typeof(e)))
        row["error"] = first(firstline(msg), 400)
        println(log, "stage: ", cur, "\n", typeof(e), "\n", first(msg, 6000))
        println(log, "--- frames\n", join(frames, "\n"))
    end
    row["stage_reached"] = reached
    open(io -> println(io, join([clean(string(row[c])) for c in COLS], '\t')), rowfile, "w")
    open(io -> write(io, take!(log)), logfile, "w")
    println(stdout, "@@DONE")
    flush(stdout)
    return nothing
end

if abspath(PROGRAM_FILE) == @__FILE__
    if !isempty(ARGS) && isfile(ARGS[1])
        # Warm up: compile the whole path once on a small model.
        tmp = mktempdir()
        run_job("Warmup", ARGS[1], "-", joinpath(tmp, "row"), joinpath(tmp, "log"))
    end
    println(stdout, "@@READY")
    flush(stdout)
    for line in eachline(stdin)
        f = split(line, '\t')
        length(f) == 5 || continue
        run_job(String.(f)...)
        GC.gc()
    end
end
