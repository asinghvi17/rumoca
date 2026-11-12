model Sub
  parameter Integer n = 2;
  Real x[n](each start = 1, each fixed = true);
equation
  der(x) = -x;
end Sub;

model ArrayModified
  Sub s(n = 4);
end ArrayModified;
