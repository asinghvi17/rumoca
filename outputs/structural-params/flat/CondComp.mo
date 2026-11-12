class CondComp
  parameter Boolean useDamper(start = true) = true;
  parameter Real 'dmp.d'(start = 1) = 1;
  Real 'dmp.v'(start = 1, fixed = true);
  Real y(start = 0, fixed = true);
equation
  der('dmp.v') = (-('dmp.d' * 'dmp.v'));
  der(y) = 1;
end CondComp;
