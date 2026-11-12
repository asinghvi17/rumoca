model ConstFinal
  constant Real c = 2;
  final parameter Real k = 3;
  parameter Real tt = 1;
  Real x(start = 0, fixed = true);
equation
  der(x) = c * k * tt;
end ConstFinal;
