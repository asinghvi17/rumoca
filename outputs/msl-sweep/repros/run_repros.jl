# Run each minimal repro through the sweep's stages and print what happens.
#   julia --project=outputs/msl-sweep/julia outputs/msl-sweep/repros/run_repros.jl [FILE.bmo ...]
include(joinpath(@__DIR__, "..", "worker.jl"))
files = isempty(ARGS) ? filter(endswith(".bmo"), readdir(@__DIR__; join = true)) : ARGS
for f in files
    print(rpad(basename(f), 24))
    stagename = "parse"
    try
        pkg = BaseModelica.parse_package(f, :antlr)
        stagename = "convert"
        sys = BaseModelica.baseModelica_to_ModelingToolkit(pkg; discrete_backend = :mtk, simplify = false)
        stagename = "simplify"
        ssys = DyadInterface.simplify_model(sys)
        stagename = "init/solve"
        prob = ODEProblem(ssys, [], (0.0, 1.0); missing_guess_value = MTKBase.MissingGuessValue.Constant(0.0))
        sol = solve(prob, Rodas5P(); reltol = 1e-8, abstol = 1e-10, dtmax = 0.01)
        tab = signal_table(ssys)
        vals = join([string(n, "(", t, ")=", round(values_at(sol, tab[n], [t])[1]; sigdigits = 5))
                      for n in sort!(collect(keys(tab))) if !occursin(r"^(der|Initial)", n) for t in (0.0, 0.25, 0.75, 1.0)], " ")
        println("OK  ", sol.retcode, "  ", vals)
    catch e
        println("FAIL in ", stagename, ": ", first(firstline(sprint(showerror, e)), 220))
    end
end
