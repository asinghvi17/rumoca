model FunctionArrayInput "A function with an unsized array input (MSL: Polynomial, Vectors, Tables, ...)"
  function total
    input Real u[:];
    output Real y;
  algorithm
    y := sum(u);
  end total;
  parameter Real p[3] = {1, 2, 3};
  Real x(start = 0, fixed = true);
equation
  der(x) = total(p) - x;
end FunctionArrayInput;
