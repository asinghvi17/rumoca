model RecordEquation "A record-valued equation (MSL: ComplexBlocks, QuasiStatic)"
  Complex z;
equation
  z = Complex(cos(time), sin(time));
  annotation(experiment(StopTime = 1));
end RecordEquation;
