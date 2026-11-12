class ForParamRange
  parameter Integer m(start = 2) = 2;
  Real[3] x(each start = 0, each fixed = true);
equation
  for i in 1:2 loop
    der(x[i]) = i;
  end for;
  for i in 3:3 loop
    der(x[i]) = 0;
  end for;
end ForParamRange;
