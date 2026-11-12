model ArraySize
  parameter Integer n = 3;
  Real x[n](each start = 0, each fixed = true);
equation
  for i in 1:n loop
    der(x[i]) = -i * x[i] + 1;
  end for;
end ArraySize;
