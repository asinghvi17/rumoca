model RecordArrayFunctionResult "A record-array equation from a function returning an array of records (found while bisecting FundamentalWave; no MSL model in this batch needs it)"
  record R
    Real a;
    Real b;
  end R;
  function f
    input Real x;
    input Integer n;
    output R r[n];
  algorithm
    for k in 1:n loop
      r[k] := R(x*k, -x);
    end for;
  end f;
  R y[3];
equation
  y = f(time, 3);
  annotation(experiment(StopTime = 1));
end RecordArrayFunctionResult;
