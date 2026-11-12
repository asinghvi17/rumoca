model DerOfExpression "der() of an expression (MSL: FluxTubes hysteresis)"
  Real x(start = 1, fixed = true);
  Real y;
equation
  der(x) = -x;
  y = der(2*x + time);
  annotation(experiment(StopTime = 1));
end DerOfExpression;
