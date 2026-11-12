// model: RecordIfEquation.M
package RecordIfEquation "A record-valued if-expression with a call arm counts as one equation (ED001)"
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
  model M
    Point z;
  equation
    z = if time > 0.5 then make(time) else Point(0, 1);
    annotation(experiment(StopTime = 1));
  end M;
end RecordIfEquation;
