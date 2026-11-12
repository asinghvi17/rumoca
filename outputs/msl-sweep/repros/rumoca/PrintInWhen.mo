model PrintInWhen "print() as a when-equation (MSL: Electrical.QuasiStatic.Polyphase.Basic.MultiDelta in IMC_withLosses; Streams.close() in Utilities.Examples.readRealParameterModel)"
  import Modelica.Utilities.Streams.print;
  Real x(start = 0, fixed = true);
equation
  der(x) = 1;
  when initial() then
    print("started\n");
  end when;
  annotation(experiment(StopTime = 1));
end PrintInWhen;
