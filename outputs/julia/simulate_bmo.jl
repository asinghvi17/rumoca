# Simulate Base Modelica files end to end: BaseModelica.jl -> ModelingToolkit
# -> ODEProblem -> Rodas5P.
#
#   julia --project=outputs/julia outputs/julia/simulate_bmo.jl FILE:STOPTIME ...
#
# Prints one TSV row per file: retcode, number of saved points, and the final
# value of up to four unknowns. Rumoca does not emit `experiment` annotations
# yet, so the stop time is given on the command line.

using BaseModelica
using OrdinaryDiffEqRosenbrock: Rodas5P, solve
using SciMLBase: remake
import ModelingToolkit

println("file\tstop\tretcode\tpoints\tfinal values")
for arg in ARGS
    file, stop = rsplit(arg, ':'; limit = 2)
    try
        prob = create_odeproblem(file)
        prob = remake(prob; tspan = (0.0, parse(Float64, stop)))
        sol = solve(prob, Rodas5P(); reltol = 1e-8, abstol = 1e-10)
        sys = prob.f.sys
        names = string.(ModelingToolkit.unknowns(sys))
        finals = join(["$(n)=$(round(v; sigdigits = 6))" for (n, v) in
            Iterators.take(zip(names, sol.u[end]), 4)], ", ")
        println(join([basename(file), stop, string(sol.retcode), length(sol.t), finals], "\t"))
    catch e
        println(join([basename(file), stop, "ERROR", 0,
            replace(first(sprint(showerror, e), 200), '\n' => " | ")], "\t"))
    end
end
