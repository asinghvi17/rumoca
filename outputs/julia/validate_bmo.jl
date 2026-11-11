# Validate Base Modelica files with BaseModelica.jl (v1.10.1).
#
#   julia --project=outputs/julia outputs/julia/validate_bmo.jl outputs/bmo/*.bmo
#
# For every file, three independent checks:
#   antlr : the ANTLR grammar parses the file with zero syntax errors. The ANTLR
#           runtime only *prints* syntax errors to stderr and still returns a
#           tree, so the check counts parser/lexer errors through the parser
#           object and also requires that nothing was written to stderr
#           (captured at the file-descriptor level, where Python writes).
#   julia : the ParserCombinator parser (`parse_file_julia`) returns a package.
#   mtk   : `parse_basemodelica(file)` builds a ModelingToolkit system (the
#           conversion layer, which is narrower than either parser).
# Prints one TSV row per file and exits non-zero if any parse check failed.

using BaseModelica
const pyimport = BaseModelica.pyimport
const pyconvert = BaseModelica.pyconvert

function capture_fd2(f)
    path, io = mktemp()
    result = nothing
    err = nothing
    sys = pyimport("sys")
    redirect_stderr(io) do
        try
            result = f()
        catch e
            err = e
        end
        sys.stderr.flush()
        Base.Libc.flush_cstdio()
    end
    close(io)
    text = read(path, String)
    rm(path)
    return result, err, text
end

function antlr_check(file)
    BaseModelica.ANTLR_INITIALIZED[] || BaseModelica.init_antlr_parser()
    source = read(file, String)
    (counts, err, stderr_text) = capture_fd2() do
        stream = BaseModelica.antlr4[].InputStream(source)
        lexer = BaseModelica.BaseModelicaLexer[](stream)
        tokens = BaseModelica.antlr4[].CommonTokenStream(lexer)
        parser = BaseModelica.BaseModelicaParser[](tokens)
        tree = parser.baseModelica()
        # The tree must also convert to the package AST.
        BaseModelica.antlr_tree_to_ast(tree)
        (pyconvert(Int, parser.getNumberOfSyntaxErrors()),)
    end
    if err !== nothing
        return false, "exception: " * first(sprint(showerror, err), 300)
    end
    nerr = counts[1]
    ok = nerr == 0 && isempty(strip(stderr_text))
    return ok, ok ? "ok" : "syntax_errors=$(nerr) stderr=" * replace(first(strip(stderr_text), 300), '\n' => " | ")
end

function julia_check(file)
    try
        pkg = BaseModelica.parse_file_julia(file)
        return pkg isa BaseModelica.BaseModelicaPackage, "ok"
    catch e
        return false, replace(first(sprint(showerror, e), 300), '\n' => " | ")
    end
end

function mtk_check(file)
    try
        sys = parse_basemodelica(file; parser = :antlr)
        return true, "ok ($(length(BaseModelica.ModelingToolkitBase.equations(sys))) eqs)"
    catch e
        return false, replace(first(sprint(showerror, e), 200), '\n' => " | ")
    end
end

failed = false
println("file\tantlr\tjulia\tmtk")
for file in ARGS
    a_ok, a_msg = antlr_check(file)
    j_ok, j_msg = julia_check(file)
    m_ok, m_msg = mtk_check(file)
    global failed |= !(a_ok && j_ok)
    println(join([basename(file), (a_ok ? "PASS" : "FAIL: " * a_msg),
        (j_ok ? "PASS" : "FAIL: " * j_msg), (m_ok ? "PASS " * m_msg : "FAIL: " * m_msg)], "\t"))
end
exit(failed ? 1 : 0)
