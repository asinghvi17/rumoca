class ReplaceableB
  Real 'law.f'(start = 0.0);
  Real 'law.u' = x;
  Real x(start = 1, fixed = true);
equation
  'law.f' = (2 * 'law.u');
  der(x) = (-'law.f');
end ReplaceableB;
