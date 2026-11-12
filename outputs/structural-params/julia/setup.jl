using Pkg
Pkg.activate(@__DIR__)
Pkg.add(url = "https://github.com/asinghvi17/BaseModelica.jl", rev = "bm-events-and-parser-fixes")
Pkg.instantiate()
Pkg.status()
