// model: ArrayLiteralIndex.Test
package ArrayLiteralIndex "Indexing a package-constant table with a run-time index (MSL: Digital logic tables)"
  package Tables
    constant Integer T[2, 2] = [1, 2; 3, 4];
  end Tables;
  model Test
    Integer i;
    Integer y;
  equation
    i = if time < 0.5 then 1 else 2;
    y = Tables.T[i, 2];
    annotation(experiment(StopTime = 1));
  end Test;
end ArrayLiteralIndex;
