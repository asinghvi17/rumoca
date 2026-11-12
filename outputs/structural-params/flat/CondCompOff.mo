class CondCompOff
  parameter Boolean useDamper(start = false) = false;
  Real y(start = 0, fixed = true);
equation
  der(y) = 1;
end CondCompOff;
