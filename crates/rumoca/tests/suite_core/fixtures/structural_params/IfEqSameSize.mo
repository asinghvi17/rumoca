model IfEqSameSize
  parameter Boolean fast = true;
  Real x(start = 0, fixed = true);
equation
  if fast then
    der(x) = 2;
  else
    der(x) = 1;
  end if;
end IfEqSameSize;
