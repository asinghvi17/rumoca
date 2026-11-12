model InitialEnumerationIf "Enumeration assigned in an initial if-equation (MSL: PowerConverters.ACAC.Control.SoftStartControl in ACAC.SoftStarter)"
  type Mode = enumeration(Off, On);
  parameter Boolean start = true;
  Real x(start = 0.2, fixed = true);
  Mode mode;
initial equation
  if start then
    if x < 1 then
      mode = Mode.On;
    else
      mode = Mode.Off;
    end if;
  else
    mode = Mode.Off;
  end if;
equation
  der(x) = 1;
  when x >= 1 then
    mode = Mode.Off;
  end when;
  annotation(experiment(StopTime = 1));
end InitialEnumerationIf;
