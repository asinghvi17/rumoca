model CallArrayElementMember "A member of an element of an array-of-records call result (MSL: QuasiStatic symmetricTransformationMatrix)"
  function rot
    input Integer m;
    output Complex t[m, m];
  algorithm
    for i in 1:m loop
      for j in 1:m loop
        t[i, j] := Complex(cos(i * j), sin(i * j));
      end for;
    end for;
  end rot;
  parameter Integer m = 2;
  parameter Complex sTM[m, m] = rot(m);
  Real y = sTM[1, 2].re * time;
  annotation(experiment(StopTime = 1));
end CallArrayElementMember;
