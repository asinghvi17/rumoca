class EvaluateAnn
  parameter Real p(start = 2) = 2;
  parameter Real q(start = 3) = 3;
  parameter Real r(start = 6) = (p * q);
  Real x(start = 0, fixed = true);
equation
  der(x) = r;
end EvaluateAnn;
