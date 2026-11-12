model ForParamRange
  parameter Integer m = 2;
  Real x[3](each start = 0, each fixed = true);
equation
  for i in 1:m loop
    der(x[i]) = i;
  end for;
  for i in m+1:3 loop
    der(x[i]) = 0;
  end for;
end ForParamRange;
