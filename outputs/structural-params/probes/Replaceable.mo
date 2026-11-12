partial model Law
  Real f;
  input Real u;
end Law;

model LawA
  extends Law;
equation
  f = u;
end LawA;

model LawB
  extends Law;
equation
  f = 2 * u;
end LawB;

model Replaceable
  replaceable model L = LawA constrainedby Law;
  L law(u = x);
  Real x(start = 1, fixed = true);
equation
  der(x) = -law.f;
end Replaceable;

model ReplaceableB
  extends Replaceable(redeclare model L = LawB);
end ReplaceableB;
