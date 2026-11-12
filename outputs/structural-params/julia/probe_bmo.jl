# Load each emitted probe through BaseModelica.jl -> MTK; report parameter metadata
# and whether `remake`-ing a structural parameter has the effect the Modelica
# semantics would give.
#   julia --project=outputs/structural-params/julia outputs/structural-params/julia/probe_bmo.jl
using BaseModelica, SciMLBase, ModelingToolkit
using OrdinaryDiffEqRosenbrock: Rodas5P, solve
import ModelingToolkit as MTK
println("MTK ", pkgversion(MTK), "  BaseModelica ", pkgversion(BaseModelica))
const R = normpath(joinpath(@__DIR__, ".."))

load(f) = create_odeproblem(f; stop_time = 1.0)
final(prob) = (s = solve(prob, Rodas5P(); abstol = 1e-10, reltol = 1e-10); (string(s.retcode), round.(s.u[end]; digits = 5)))
pname(p) = string(p)

for dir in ("bmo", "bmo-scalar")
    println("\n######## ", dir)
    for f in sort(filter(endswith(".bmo"), readdir(joinpath(R, dir); join = true)))
        println("\n### ", dir, "/", basename(f))
        try
            prob = load(f)
            sys = prob.f.sys
            println("unknowns: ", join(string.(MTK.unknowns(sys)), ", "))
            for p in MTK.parameters(sys)
                println("  param ", p, "  tunable=", MTK.istunable(p, true), "  type=", MTK.Symbolics.symtype(p),
                        "  default=", get(MTK.bindings(sys), p, "-"))
            end
            println("  solve: ", final(prob))
        catch e
            println("  ERROR ", first(sprint(showerror, e), 250))
        end
    end
end

println("\n######## remake experiments (bmo-scalar form)")
function try_remake(file, pairs)
    prob = load(joinpath(R, "bmo-scalar", file))
    println("\n# ", file, "  nunknowns=", length(prob.u0), "  remake ", pairs)
    println("  base:    ", final(prob))
    try
        p2 = remake(prob; p = pairs)
        println("  remaked: ", final(p2), "  nunknowns=", length(p2.u0))
    catch e
        println("  remake ERROR ", first(sprint(showerror, e), 250))
    end
end
function symp(file, name)
    prob = load(joinpath(R, "bmo-scalar", file))
    only(p for p in MTK.parameters(prob.f.sys) if string(p) == name)
end
for (file, name, val) in (("ArraySize.bmo", "n", 5), ("ForParamRange.bmo", "m", 3), ("IfEqUnbalanced.bmo", "withY", false),
                          ("CondComp.bmo", "useDamper", false), ("ParamSelect.bmo", "kind", 2),
                          ("EvaluateAnn.bmo", "p", 5.0), ("TunableOnly.bmo", "tau", 2.0))
    try
        try_remake(file, [symp(file, name) => val])
    catch e
        println("\n# ", file, " ERROR ", first(sprint(showerror, e), 250))
    end
end

println("\n######## EvaluateAnn: dependent parameter r = p*q")
let prob = load(joinpath(R, "bmo-scalar", "EvaluateAnn.bmo")), sys = prob.f.sys
    ps = Dict(string(p) => p for p in MTK.parameters(sys))
    println("  parameters: ", keys(ps))
    for (k, v) in (("p", 5.0), ("q", 5.0))
        println("  remake ", k, "=>", v, ": ", final(remake(prob; p = [ps[k] => v])))
    end
    println("  remake p=>5, r=>15: ", final(remake(prob; p = [ps["p"] => 5.0, ps["r"] => 15.0])))
end
