model TrapezoidSource "Repro: MSL OpAmps examples' trapezoid input source"
  parameter Real Vin = 5;
  Modelica.Electrical.Analog.Sources.TrapezoidVoltage vIn(V = 2*Vin, offset = -Vin,
    rising = 0.2, width = 0.3, falling = 0.2, period = 1, nperiod = -1);
  Modelica.Electrical.Analog.Basic.Resistor r(R = 1);
  Modelica.Electrical.Analog.Basic.Ground g;
equation
  connect(vIn.p, r.p);
  connect(r.n, vIn.n);
  connect(vIn.n, g.p);
  annotation(experiment(StopTime = 2));
end TrapezoidSource;
