model ClockedSample "Synchronous (clocked) sampling (MSL: Modelica.Clocked)"
  Real x(start = 1, fixed = true);
  Real xd;
equation
  der(x) = -x;
  xd = sample(x, Clock(0.1));
  annotation(experiment(StopTime = 1));
end ClockedSample;
