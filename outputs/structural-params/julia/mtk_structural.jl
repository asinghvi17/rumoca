# What MTK itself offers for structural / non-tunable parameters.
#   julia --project=outputs/structural-params/julia outputs/structural-params/julia/mtk_structural.jl
using ModelingToolkit, SciMLBase, SciCompDSL
using ModelingToolkit: t_nounits as t, D_nounits as D
import ModelingToolkit as MTK
using OrdinaryDiffEqRosenbrock: Rodas5P, solve
fin(p) = round.(solve(p, Rodas5P(); abstol=1e-10, reltol=1e-10).u[end]; digits=5)
println("MTK ", pkgversion(MTK))

println("\n## 1. parameter with tunable=false: kept in the parameter object, settable via remake")
@parameters k = 1.0 [tunable = false]
@parameters tau = 1.0
@variables x(t) = 1.0
@named s1 = System([D(x) ~ -k * x / tau], t)
s1 = mtkcompile(s1)
prob = ODEProblem(s1, [], (0.0, 1.0))
println("  tunable_parameters: ", MTK.tunable_parameters(s1), "   all: ", MTK.parameters(s1))
println("  p object: ", prob.p)
println("  base ", fin(prob), "  remake k=2: ", try fin(remake(prob; p = [k => 2.0])) catch e; "ERROR " * first(sprint(showerror, e), 200) end)

println("\n## 2. runtime-parameter array size is rejected")
try
    @parameters n::Int = 3
    @variables y(t)[1:n]
    println("  built y with n symbolic: ", y)
catch e
    println("  ERROR ", first(sprint(showerror, e), 250))
end

println("\n## 3. @mtkmodel @structural_parameters: array size fixed at construction")
@mtkmodel Chain begin
    @structural_parameters begin
        n = 3
    end
    @parameters begin
        a = 1.0
    end
    @variables begin
        x(t)[1:n] = ones(n)
    end
    @equations begin
        D(x) ~ -a * x
    end
end
for n in (3, 5)
    @named c = Chain(n = n)
    cs = mtkcompile(c)
    pr = ODEProblem(cs, [], (0.0, 1.0))
    println("  n=", n, ": nunknowns=", length(pr.u0), "  parameters=", MTK.parameters(cs))
end

println("\n## 4. @mtkmodel conditional component via structural parameter")
@mtkmodel Damp begin
    @parameters begin d = 1.0 end
    @variables begin v(t) = 1.0 end
    @equations begin D(v) ~ -d * v end
end
@mtkmodel Top begin
    @structural_parameters begin useDamper = true end
    @variables begin y(t) = 0.0 end
    @components begin
        if useDamper
            dmp = Damp()
        end
    end
    @equations begin D(y) ~ 1 end
end
for u in (true, false)
    @named tp = Top(useDamper = u)
    println("  useDamper=", u, ": unknowns ", MTK.unknowns(mtkcompile(tp)))
end
