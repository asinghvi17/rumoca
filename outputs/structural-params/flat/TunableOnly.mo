class TunableOnly
  parameter Real tau(start = 1) = 1;
  Real x(start = 1, fixed = true);
equation
  der(x) = (-(x / tau));
end TunableOnly;
