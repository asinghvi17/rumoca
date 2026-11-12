class IfEqUnbalanced
  parameter Boolean withY(start = true) = true;
  Real x(start = 0, fixed = true);
  Real y(start = 0, fixed = true);
equation
  der(x) = 1;
  der(y) = x;
end IfEqUnbalanced;
