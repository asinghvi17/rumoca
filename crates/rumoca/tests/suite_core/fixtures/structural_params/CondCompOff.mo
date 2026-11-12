model Damper
  parameter Real d = 1;
  Real v(start = 1, fixed = true);
equation
  der(v) = -d * v;
end Damper;

model CondCompOff
  parameter Boolean useDamper = false;
  Damper dmp if useDamper;
  Real y(start = 0, fixed = true);
equation
  der(y) = 1;
end CondCompOff;
