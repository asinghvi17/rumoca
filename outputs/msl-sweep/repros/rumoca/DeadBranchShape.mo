model DeadBranchShape "Shape check in a branch the specialization never takes (MSL: Polyphase.Functions.symmetricOrientationMatrix)"
  function f
    input Integer m;
    output Real M[m, m];
  algorithm
    if m == 2 then
      M[1:m, 1:m] := {{1, 0}, {0, 1}};
    else
      M := identity(m);
    end if;
  end f;
  parameter Integer m = 3;
  parameter Real M[m, m] = f(m);
  Real x(start = 1, fixed = true);
equation
  der(x) = -M[1, 1]*x;
  annotation(experiment(StopTime = 1));
end DeadBranchShape;
