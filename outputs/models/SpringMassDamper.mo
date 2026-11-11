model SpringMassDamper "Mass on a spring with viscous damping"
  parameter Real m(unit = "kg") = 1.0 "Mass";
  parameter Real k(unit = "N/m") = 10.0 "Spring stiffness";
  parameter Real d(unit = "N.s/m") = 0.5 "Damping coefficient";
  Real x(start = 1.0, fixed = true, unit = "m") "Position";
  Real v(start = 0.0, fixed = true, unit = "m/s") "Velocity";
equation
  der(x) = v;
  m * der(v) = -k * x - d * v;
end SpringMassDamper;
