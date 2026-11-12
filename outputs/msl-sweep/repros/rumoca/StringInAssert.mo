model StringInAssert "String() in an assert message (MSL: Limiter, PID, Media)"
  parameter Real k = 2;
  Real x(start = 1, fixed = true);
equation
  der(x) = -k*x;
  assert(k > 0, "k = " + String(k) + " must be positive");
  annotation(experiment(StopTime = 1));
end StringInAssert;
