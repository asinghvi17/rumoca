model EventSample "Event sampling with sample(start, interval) (MSL: Blocks.Discrete, Sources.SampleTrigger)"
  Integer n(start = 0, fixed = true);
equation
  when sample(0, 0.1) then
    n = pre(n) + 1;
  end when;
  annotation(experiment(StopTime = 1));
end EventSample;
