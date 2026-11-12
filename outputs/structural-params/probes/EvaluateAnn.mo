model EvaluateAnn
  parameter Real p = 2 annotation(Evaluate = true);
  parameter Real q = 3;
  parameter Real r = p * q;
  Real x(start = 0, fixed = true);
equation
  der(x) = r;
end EvaluateAnn;
