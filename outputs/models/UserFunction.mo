package UserFunction "Model calling user-defined functions"
  function cubicDrag "Drag force with a cubic correction term"
    input Real v;
    input Real c1;
    input Real c3 = 0.1;
    output Real f;
  protected
    Real v3;
  algorithm
    v3 := v * v * v;
    f := c1 * v + c3 * v3;
  end cubicDrag;

  function saturate
    input Real u;
    input Real lim;
    output Real y;
  algorithm
    if u > lim then
      y := lim;
    elseif u < -lim then
      y := -lim;
    else
      y := u;
    end if;
  end saturate;

  model Decay
    parameter Real c = 0.4;
    parameter Real limit = 2.0;
    Real x(start = 1.0, fixed = true);
    Real y;
  equation
    der(x) = -cubicDrag(x, c);
    y = saturate(5 * x, limit);
  end Decay;
end UserFunction;
