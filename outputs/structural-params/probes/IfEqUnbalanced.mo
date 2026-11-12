model IfEqUnbalanced
  parameter Boolean withY = true;
  Real x(start = 0, fixed = true);
  Real y(start = 0, fixed = true);
equation
  der(x) = 1;
  if withY then
    der(y) = x;
  else
    y = 0;
  end if;
end IfEqUnbalanced;
