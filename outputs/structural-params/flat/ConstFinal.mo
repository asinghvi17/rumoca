class ConstFinal
  constant Real c(start = 2) = 2;
  final parameter Real k(start = 3) = 3;
  parameter Real tt(start = 1) = 1;
  Real x(start = 0, fixed = true);
equation
  der(x) = ((c * k) * tt);
end ConstFinal;
