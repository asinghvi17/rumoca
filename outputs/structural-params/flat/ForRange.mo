class ForRange
  constant Integer n(start = 3) = 3;
  parameter Integer m(start = 2) = 2;
  Real[3] x(each start = 0, each fixed = true);
equation
  for i in 1:3 loop
    der(x[i]) = i;
  end for;
end ForRange;
