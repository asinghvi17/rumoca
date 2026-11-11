package RecordFunction "Record types used by functions: package-level record classes and a generated constructor"
  record Point
    Real x;
    Real y;
  end Point;

  function make
    input Real a;
    output Point p;
  algorithm
    p := Point(a, 2 * a);
  end make;

  function norm2
    input Real a;
    output Real n;
  protected
    Point p;
  algorithm
    p := make(a);
    n := p.x * p.x + p.y * p.y;
  end norm2;

  model Decay "x decays at a rate set through record-valued helpers"
    Real x(start = 1.0, fixed = true);
  equation
    der(x) = -norm2(0.5) * x;
    annotation(experiment(StopTime = 2));
  end Decay;
end RecordFunction;
