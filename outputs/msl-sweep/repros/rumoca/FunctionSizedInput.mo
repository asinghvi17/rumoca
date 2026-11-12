model FunctionSizedInput "A function input sized by another input (MSL: Blocks.Tables, Math.Matrices)"
  function dot
    input Real a[:];
    input Real b[size(a, 1)];
    output Real y;
  algorithm
    y := a*b;
  end dot;
  parameter Real p[2] = {1, 2};
  Real x(start = 0, fixed = true);
equation
  der(x) = dot(p, p) - x;
end FunctionSizedInput;
