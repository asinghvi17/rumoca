model QSIdealSwitch "Quasi-static ideal closing switch driven by a Boolean source (MSL: Electrical.QuasiStatic.SinglePhase.Ideal.IdealClosingSwitch/IdealCommutingSwitch in IMC_DOL, IMC_Transformer, IMC_YD, IMS_Start)"
  Modelica.Electrical.QuasiStatic.SinglePhase.Sources.VoltageSource vs(f = 50, V = 1, phi = 0);
  Modelica.Electrical.QuasiStatic.SinglePhase.Ideal.IdealClosingSwitch sw;
  Modelica.Electrical.QuasiStatic.SinglePhase.Basic.Resistor r(R_ref = 1);
  Modelica.Electrical.QuasiStatic.SinglePhase.Basic.Ground g;
  Modelica.Blocks.Sources.BooleanStep st(startTime = 0.5);
equation
  connect(vs.pin_p, sw.pin_p);
  connect(sw.pin_n, r.pin_p);
  connect(r.pin_n, vs.pin_n);
  connect(vs.pin_n, g.pin);
  connect(st.y, sw.control);
  annotation(experiment(StopTime = 1));
end QSIdealSwitch;
