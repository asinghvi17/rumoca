# Parse every re-emitted probe in bmo-marked/ (base-modelica) and bmo-marked/scalar/
# (base-modelica-scalar) with BaseModelica.jl's ANTLR parser (and its Julia parser for
# comparison), build the MTK system, and solve the scalar form, to check that
# `annotation(Evaluate = true)` on a parameter declaration is accepted.
#   julia --project=outputs/structural-params/julia outputs/structural-params/julia/probe_marked.jl
using BaseModelica, SciMLBase, ModelingToolkit
using OrdinaryDiffEqRosenbrock: Rodas5P, solve
import ModelingToolkit as MTK
println("MTK ", pkgversion(MTK), "  BaseModelica ", pkgversion(BaseModelica))
const R = normpath(joinpath(@__DIR__, "..", "bmo-marked"))

short(e) = first(sprint(showerror, e), 200)
for dir in (R, joinpath(R, "scalar"))
    println("\n######## ", relpath(dir, joinpath(R, "..")))
    for f in sort(filter(endswith(".bmo"), readdir(dir; join = true)))
        marked = count("annotation(Evaluate = true)", read(f, String))
        print(rpad(basename(f), 20), " marked=", marked)
        # `parse_package` is the parser alone (the AST); `parse_basemodelica` also
        # evaluates it to an MTK system, which refuses array variables.
        for parser in (:antlr, :julia)
            try
                BaseModelica.parse_package(f, parser)
                print("  parse(", parser, ")=ok")
            catch e
                print("  parse(", parser, ")=ERROR ", short(e))
            end
        end
        try
            BaseModelica.parse_basemodelica(f; parser = :antlr)
            print("  system=ok")
        catch e
            print("  system=ERROR ", short(e))
        end
        if dir != R
            try
                prob = create_odeproblem(f; stop_time = 1.0)
                s = solve(prob, Rodas5P(); abstol = 1e-10, reltol = 1e-10)
                print("  solve=", s.retcode, " ", round.(s.u[end]; digits = 5),
                      "  params=", join(string.(MTK.parameters(prob.f.sys)), ","))
            catch e
                print("  solve=ERROR ", short(e))
            end
        end
        println()
    end
end
