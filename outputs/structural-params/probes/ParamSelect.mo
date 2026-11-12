model ParamSelect
  parameter Integer kind = 1 "1 = linear, 2 = quadratic";
  Real x(start = 1, fixed = true);
equation
  if kind == 1 then
    der(x) = -x;
  else
    der(x) = -x * x;
  end if;
end ParamSelect;
