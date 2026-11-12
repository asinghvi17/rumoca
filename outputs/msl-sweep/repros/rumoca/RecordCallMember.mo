model RecordCallMember "A member read off a record-valued call (MSL: MultiBody Frames, Media states, Complex operators)"
  function polar
    input Real r;
    input Real phi;
    output Complex z;
  algorithm
    z := Complex(r * cos(phi), r * sin(phi));
  end polar;
  Complex z = polar(1, time);
  Real re = Modelica.ComplexMath.abs(polar(2, time));
  annotation(experiment(StopTime = 1));
end RecordCallMember;
