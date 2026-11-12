model Damper
  parameter Real d = 1;
  Real v(start = 1, fixed = true);
equation
  der(v) = -d * v;
end Damper;

model CondComp
  parameter Boolean useDamper = true;
  Damper dmp if useDamper;
  Real y(start = 0, fixed = true);
equation
  der(y) = 1;
end CondComp;
