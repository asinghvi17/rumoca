class ArrayModified
  parameter Integer 's.n'(start = 0) = 4;
  Real[4] 's.x'(each start = 1, each fixed = true);
equation
  der('s.x') = (-'s.x');
end ArrayModified;
