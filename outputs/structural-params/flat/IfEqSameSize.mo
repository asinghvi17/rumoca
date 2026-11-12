class IfEqSameSize
  parameter Boolean fast(start = true) = true;
  Real x(start = 0, fixed = true);
equation
  0 = (if fast then (der(x) - 2) else (der(x) - 1));
end IfEqSameSize;
