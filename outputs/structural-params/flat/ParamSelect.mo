class ParamSelect
  parameter Integer kind(start = 1) = 1 "1 = linear, 2 = quadratic";
  Real x(start = 1, fixed = true);
equation
  0 = (if (kind == 1) then (der(x) - (-x)) else (der(x) - (-(x * x))));
end ParamSelect;
