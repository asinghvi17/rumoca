package RCCircuit "RC circuit built from connectors, partial models and extends"
  connector Pin
    Real v "Potential";
    flow Real i "Current";
  end Pin;

  partial model TwoPin
    Pin p;
    Pin n;
    Real v "Voltage drop";
    Real i "Current through";
  equation
    v = p.v - n.v;
    0 = p.i + n.i;
    i = p.i;
  end TwoPin;

  model Resistor
    extends TwoPin;
    parameter Real R = 1000.0 "Resistance";
  equation
    v = R * i;
  end Resistor;

  model Capacitor
    extends TwoPin(v(start = 0.0, fixed = true));
    parameter Real C = 1e-3 "Capacitance";
  equation
    C * der(v) = i;
  end Capacitor;

  model ConstantVoltage
    extends TwoPin;
    parameter Real V = 5.0 "Source voltage";
  equation
    v = V;
  end ConstantVoltage;

  model Ground
    Pin p;
  equation
    p.v = 0;
  end Ground;

  model Circuit "Step response of an RC low-pass"
    Resistor R1(R = 100.0);
    Capacitor C1(C = 0.01);
    ConstantVoltage V1(V = 10.0);
    Ground G;
  equation
    connect(V1.p, R1.p);
    connect(R1.n, C1.p);
    connect(C1.n, V1.n);
    connect(V1.n, G.p);
  end Circuit;
end RCCircuit;
