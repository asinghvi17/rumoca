class ArraySize
  parameter Integer n(start = 3) = 3;
  Real[3] x(each start = 0, each fixed = true);
equation
  for i in 1:3 loop
    der(x[i]) = ((-(i * x[i])) + 1);
  end for;
end ArraySize;
