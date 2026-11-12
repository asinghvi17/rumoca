model ForRange
  constant Integer n = 3;
  parameter Integer m = 2;
  Real x[3](each start = 0, each fixed = true);
equation
  for i in 1:n loop
    der(x[i]) = i;
  end for;
end ForRange;
