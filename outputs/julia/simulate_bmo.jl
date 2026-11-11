# Simulate Base Modelica files end to end: BaseModelica.jl -> ModelingToolkit
# -> ODEProblem -> Rodas5P.
#
#   julia --project=outputs/julia outputs/julia/simulate_bmo.jl FILE[:STOPTIME] ...
#
# Prints one TSV row per file: the time span, retcode, number of saved points,
# and the final value of up to four unknowns. Without `:STOPTIME` the time span,
# `reltol` and `saveat` are the ones `create_odeproblem` read from the emitted
# `annotation(experiment(...))`; with it the stop time is overridden.

using BaseModelica
using OrdinaryDiffEqRosenbrock: Rodas5P, solve
using SciMLBase: remake
import ModelingToolkit

println("file\ttspan\tretcode\tpoints\tfinal values")
for arg in ARGS
    file, stop = occursin(r":[0-9.e+-]+$", arg) ? rsplit(arg, ':'; limit = 2) : (arg, nothing)
    try
        prob = create_odeproblem(file)
        if stop !== nothing
            prob = remake(prob; tspan = (prob.tspan[1], parse(Float64, stop)))
        end
        sol = solve(prob, Rodas5P(); abstol = 1e-10)
        sys = prob.f.sys
        names = string.(ModelingToolkit.unknowns(sys))
        finals = join(["$(n)=$(round(v; sigdigits = 6))" for (n, v) in
            Iterators.take(zip(names, sol.u[end]), 4)], ", ")
        println(join([basename(file), string(prob.tspan), string(sol.retcode), length(sol.t), finals], "\t"))
    catch e
        println(join([basename(file), something(stop, "-"), "ERROR", 0,
            replace(first(sprint(showerror, e), 200), '\n' => " | ")], "\t"))
    end
end
