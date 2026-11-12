models: 534

## stage funnel (models that completed each stage)
| emit | 107 | 20.0% |
| parse | 103 | 19.3% |
| convert | 80 | 15.0% |
| simplify | 61 | 11.4% |
| init | 61 | 11.4% |
| solve | 55 | 10.3% |
| compare | 46 | 8.6% |

status: {'fail': 488, 'pass': 46}

## categories
| emit | rumoca: function array-parameter dimensions ([:], size(...)) not rendered | rumoca | 212 | Modelica.Blocks.Examples.BooleanNetwork1 |
| emit | rumoca: sample() not lowered (event sample(t0,dt) and clocked sample) | rumoca | 51 | Modelica.Blocks.Examples.DemoSignalCharacteristic |
| emit | Clock/synchronous features (Clock, superSample, hold, ...) | rumoca | 35 | Modelica.Clocked.Examples.Elementary.BooleanSignals.AssignClock |
| emit | rumoca: String() conversion (assert/terminate messages) | rumoca | 27 | Modelica.Blocks.Examples.PID_Controller |
| emit | rumoca: ED019 shape check fires in function branches the specialization never takes | rumoca | 20 | Modelica.Electrical.QuasiStatic.Machines.Examples.TransformerTestbench |
| emit | rumoca: record-valued equations/paths (Complex, records) | rumoca | 19 | Modelica.ComplexBlocks.Examples.TestConversionBlock |
| emit | rumoca: ED013 model algorithm with loops/break/multi-output (Digital) | rumoca | 16 | Modelica.Electrical.Analog.Examples.AD_DA_conversion |
| simplify | BaseModelica.jl: parameter bindings intermittently lose referenced parameters (nondeterministic) | BaseModelica.jl | 15 | Modelica.Electrical.Analog.Examples.HeatingNPN_NORGate |
| emit | rumoca: ED019 other unsupported Flat construct (function arrays/records/loops) | rumoca | 14 | Modelica.Blocks.Examples.Filter |
| convert | BaseModelica.jl: assert() in initial algorithm rejected | BaseModelica.jl | 13 | Modelica.Electrical.Analog.Examples.DemoPowerSupply |
| emit | rumoca: other | rumoca | 9 | Modelica.Electrical.PowerConverters.Examples.ACAC.SoftStarter |
| emit | rumoca: ED019 impure call statement (error/print) in a function | rumoca | 7 | Modelica.Fluid.Examples.BranchingDynamicPipes |
| convert | BaseModelica.jl: array variables unsupported (array-preserving output) | BaseModelica.jl | 5 | Modelica.Blocks.Examples.InverseModel |
| emit | rumoca: ED019 der() of an expression | rumoca | 5 | Modelica.Magnetic.FluxTubes.Examples.Hysteresis.HysteresisModelComparison |
| emit | rumoca: unevaluable array dimensions | rumoca | 4 | Modelica.Blocks.Examples.Rectifier12pulseFFT |
| compare | BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0) | BaseModelica.jl | 3 | Modelica.Blocks.Examples.BusUsage |
| convert | BaseModelica.jl: event operators initial()/terminal()/edge() unsupported | BaseModelica.jl | 3 | Modelica.Blocks.Examples.DemonstrateContinuousSignalExtrema |
| compare | initialisation picks another root of the nonlinear init system (start values / homotopy) | BaseModelica.jl/MTK | 2 | Modelica.Electrical.Analog.Examples.DemonstrateLightning |
| simplify | delay() operator (needs a DDE / history) | BaseModelica.jl/MTK | 2 | Modelica.Electrical.Analog.Examples.Lines.CompareLosslessLines |
| parse | BaseModelica.jl: ANTLR (Python) recursion limit on deeply nested expressions | BaseModelica.jl | 2 | Modelica.Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine |
| solve | solver failure (Unstable/MaxIters) | solver | 2 | Modelica.Electrical.Analog.Examples.OpAmps.ControlCircuit |
| simplify | MTK structural simplification: cycle/loop error | MTK | 2 | Modelica.Electrical.Analog.Examples.Resistor |
| emit | rumoca: expandable connector augmentation | rumoca | 2 | Modelica.Electrical.Batteries.Examples.CCCV_Stack |
| parse | rumoca: subscripted array literal [..][i,j] is not Base Modelica (Digital logic tables) | rumoca | 2 | Modelica.Electrical.Digital.Examples.Adder4 |
| solve | init/solve: NaN at t=0 (initial values) | BaseModelica.jl/MTK | 2 | Modelica.Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase |
| emit | rumoca: ER002 unresolved (replaceable package) function | rumoca | 2 | Modelica.Fluid.Examples.Explanatory.MomentumBalanceFittings |
| compare | numerical mismatch: borderline, 1.04e-3 against the 1e-3 threshold (friction stick events) | BaseModelica.jl/MTK | 2 | Modelica.Mechanics.Rotational.Examples.CompareBrakingTorque |
| emit | rumoca: ER130 non-constant outer reference in nested function | rumoca | 2 | Modelica.Mechanics.Translational.Examples.Friction |
| emit | External C functions | rumoca | 1 | Modelica.Blocks.Examples.Noise.ImpureGenerator |
| convert | BaseModelica.jl: `return` in functions unsupported | BaseModelica.jl | 1 | Modelica.Electrical.Analog.Examples.AmplifierWithOpAmpDetailed |
| solve | BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0) | BaseModelica.jl | 1 | Modelica.Electrical.Analog.Examples.CharacteristicIdealDiodes |
| convert | BaseModelica.jl: array sized by a parameter | BaseModelica.jl | 1 | Modelica.Electrical.Analog.Examples.Lines.CompareLineTrunks |
| compare | numerical mismatch: early transient differs (undiagnosed) | BaseModelica.jl/MTK | 1 | Modelica.Electrical.Machines.Examples.DCMachines.DCPM_Drive |
| emit | rumoca: base-modelica-scalar cannot scalarize | rumoca | 1 | Modelica.Electrical.PowerConverters.Examples.ACDC.RectifierBridge2Pulse.ThyristorBridge2Pulse_DC_Drive |
| solve | MTK: initialization internal error | MTK | 1 | Modelica.Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap |
| compare | numerical mismatch: stick/slip friction transient, 1.5% (tolerance-level) | BaseModelica.jl/MTK | 1 | Modelica.Mechanics.Translational.Examples.PreLoad |

## per sub-library
| sub-library | models | emit | parse | convert | simplify | solve | pass | top failure (count) |
|---|---|---|---|---|---|---|---|---|
| Electrical | 206 | 61 | 57 | 50 | 32 | 27 | 24 | rumoca: function array-parameter dimensions ([:], size(...)) not rendered (72) |
| Clocked | 77 | 3 | 3 | 1 | 1 | 1 | 1 | rumoca: sample() not lowered (event sample(t0,dt) and clocked sample) (29) |
| Mechanics | 75 | 18 | 18 | 17 | 17 | 17 | 13 | rumoca: function array-parameter dimensions ([:], size(...)) not rendered (54) |
| Magnetic | 69 | 5 | 5 | 5 | 4 | 3 | 3 | rumoca: function array-parameter dimensions ([:], size(...)) not rendered (30) |
| Blocks | 31 | 8 | 8 | 5 | 5 | 5 | 4 | rumoca: function array-parameter dimensions ([:], size(...)) not rendered (15) |
| Fluid | 23 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: String() conversion (assert/terminate messages) (10) |
| Media | 21 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: ED019 other unsupported Flat construct (function arrays/records/loops) (8) |
| Thermal | 16 | 12 | 12 | 2 | 2 | 2 | 1 | BaseModelica.jl: assert() in initial algorithm rejected (10) |
| StateGraph | 7 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: function array-parameter dimensions ([:], size(...)) not rendered (7) |
| Math | 4 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: unevaluable array dimensions (2) |
| Utilities | 3 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: String() conversion (assert/terminate messages) (1) |
| ComplexBlocks | 2 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: ED019 other unsupported Flat construct (function arrays/records/loops) (1) |

## emit by target
emit_array {'fail': 427, 'ok': 107}
emit_scalar {'fail': 435, 'ok': 98, 'timeout': 1}
(array, scalar): {('fail', 'fail'): 427, ('ok', 'ok'): 98, ('ok', 'fail'): 8, ('ok', 'timeout'): 1}
