model ArrayModel "Chain of first-order lags written with arrays"
  parameter Integer n = 3;
  parameter Real tau[n] = {1.0, 2.0, 3.0};
  Real x[n](each start = 0.0, each fixed = true);
  Real u = 1.0;
equation
  der(x[1]) = (u - x[1]) / tau[1];
  for i in 2:n loop
    der(x[i]) = (x[i - 1] - x[i]) / tau[i];
  end for;
end ArrayModel;
