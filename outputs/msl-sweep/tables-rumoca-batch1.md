models: 534

## stage funnel (models that completed each stage)
| emit | 359 | 67.2% |
| parse | 239 | 44.8% |
| convert | 84 | 15.7% |
| simplify | 68 | 12.7% |
| init | 68 | 12.7% |
| solve | 60 | 11.2% |
| compare | 51 | 9.6% |

status: {'fail': 483, 'pass': 51}

## categories
| convert | BaseModelica.jl: array variables unsupported (array-preserving output) | BaseModelica.jl | 126 | Modelica.Blocks.Examples.BooleanNetwork1 |
| parse | rumoca: member access on a function-call result `f(...).'x'` is not Base Modelica (record-valued calls) | rumoca | 109 | Modelica.Electrical.Batteries.Examples.ShowImpedance |
| emit | Clock/synchronous features (Clock, superSample, hold, ...) | rumoca | 69 | Modelica.Clocked.Examples.Elementary.BooleanSignals.AssignClock |
| emit | rumoca: other | rumoca | 27 | Modelica.Electrical.PowerConverters.Examples.ACAC.SoftStarter |
| emit | rumoca: ED013 model algorithm with loops/break/multi-output (Digital) | rumoca | 16 | Modelica.Electrical.Analog.Examples.AD_DA_conversion |
| emit | rumoca: ED019 other unsupported Flat construct (function arrays/records/loops) | rumoca | 14 | Modelica.Blocks.Examples.Filter |
| convert | BaseModelica.jl: assert() in initial algorithm rejected | BaseModelica.jl | 13 | Modelica.Electrical.Analog.Examples.DemoPowerSupply |
| emit | rumoca: sample() not lowered (event sample(t0,dt) and clocked sample) | rumoca | 12 | Modelica.Clocked.Examples.CascadeControlledDrive.AbsoluteClocks |
| simplify | BaseModelica.jl: parameter bindings intermittently lose referenced parameters (nondeterministic) | BaseModelica.jl | 12 | Modelica.Electrical.Analog.Examples.HeatingNPN_NORGate |
| emit | rumoca: base-modelica-scalar cannot scalarize | rumoca | 9 | Modelica.Blocks.Examples.Noise.DrydenContinuousTurbulence |
| parse | rumoca: subscripted array literal [..][i,j] is not Base Modelica (Digital logic tables) | rumoca | 7 | Modelica.Electrical.Digital.Examples.Adder4 |
| convert | BaseModelica.jl: fill() sized by a parameter | BaseModelica.jl | 7 | Modelica.Electrical.PowerConverters.Examples.DCDC.ChopperBuckBoost.ChopperBuckBoost_DutyCycle |
| emit | rumoca: ED019 impure call statement (error/print) in a function | rumoca | 7 | Modelica.Fluid.Examples.BranchingDynamicPipes |
| emit | rumoca: record-valued equations/paths (Complex, records) | rumoca | 5 | Modelica.Electrical.Digital.Examples.VectorDelay |
| emit | rumoca: ED019 der() of an expression | rumoca | 5 | Modelica.Magnetic.FluxTubes.Examples.Hysteresis.HysteresisModelComparison |
| emit | rumoca: unevaluable array dimensions | rumoca | 4 | Modelica.Blocks.Examples.Rectifier12pulseFFT |
| compare | BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0) | BaseModelica.jl | 3 | Modelica.Blocks.Examples.BusUsage |
| convert | BaseModelica.jl: event operators initial()/terminal()/edge() unsupported | BaseModelica.jl | 3 | Modelica.Blocks.Examples.DemonstrateContinuousSignalExtrema |
| convert | BaseModelica.jl: sample(start, interval) with parameter-valued arguments (events.jl sample_args) | BaseModelica.jl | 3 | Modelica.Electrical.Analog.Examples.CauerLowPassSC |
| solve | solver failure (Unstable/MaxIters) | solver | 3 | Modelica.Electrical.Analog.Examples.OpAmps.ControlCircuit |
| compare | initialisation picks another root of the nonlinear init system (start values / homotopy) | BaseModelica.jl/MTK | 2 | Modelica.Electrical.Analog.Examples.DemonstrateLightning |
| simplify | delay() operator (needs a DDE / history) | BaseModelica.jl/MTK | 2 | Modelica.Electrical.Analog.Examples.Lines.CompareLosslessLines |
| parse | BaseModelica.jl: ANTLR (Python) recursion limit on deeply nested expressions | BaseModelica.jl | 2 | Modelica.Electrical.Analog.Examples.Lines.LightningSegmentedTransmissionLine |
| simplify | MTK structural simplification: cycle/loop error | MTK | 2 | Modelica.Electrical.Analog.Examples.Resistor |
| emit | rumoca: expandable connector augmentation | rumoca | 2 | Modelica.Electrical.Batteries.Examples.CCCV_Stack |
| solve | init/solve: NaN at t=0 (initial values) | BaseModelica.jl/MTK | 2 | Modelica.Electrical.Machines.Examples.DCMachines.DCSE_SinglePhase |
| emit | rumoca: ER002 unresolved (replaceable package) function | rumoca | 2 | Modelica.Fluid.Examples.Explanatory.MomentumBalanceFittings |
| compare | numerical mismatch: borderline, 1.04e-3 against the 1e-3 threshold (friction stick events) | BaseModelica.jl/MTK | 2 | Modelica.Mechanics.Rotational.Examples.CompareBrakingTorque |
| emit | rumoca: ER130 non-constant outer reference in nested function | rumoca | 2 | Modelica.Mechanics.Translational.Examples.Friction |
| parse | BaseModelica.jl: other parse error | BaseModelica.jl | 2 | Modelica.Media.Examples.SimpleLiquidWater |
| convert | convert: other | BaseModelica.jl | 1 | Modelica.Blocks.Examples.DemonstrateSignalExtrema |
| emit | External C functions | rumoca | 1 | Modelica.Blocks.Examples.Noise.ImpureGenerator |
| convert | BaseModelica.jl: `return` in functions unsupported | BaseModelica.jl | 1 | Modelica.Electrical.Analog.Examples.AmplifierWithOpAmpDetailed |
| solve | BaseModelica.jl: relation-defined Boolean takes its start value at init (b = x > 0) | BaseModelica.jl | 1 | Modelica.Electrical.Analog.Examples.CharacteristicIdealDiodes |
| convert | BaseModelica.jl: array sized by a parameter | BaseModelica.jl | 1 | Modelica.Electrical.Analog.Examples.Lines.CompareLineTrunks |
| solve | solve: other | solver | 1 | Modelica.Electrical.Analog.Examples.OpAmps.VoltageFollower |
| compare | numerical mismatch: early transient differs (undiagnosed) | BaseModelica.jl/MTK | 1 | Modelica.Electrical.Machines.Examples.DCMachines.DCPM_Drive |
| solve | MTK: initialization internal error | MTK | 1 | Modelica.Magnetic.FluxTubes.Examples.BasicExamples.QuadraticCoreAirgap |
| compare | numerical mismatch: stick/slip friction transient, 1.5% (tolerance-level) | BaseModelica.jl/MTK | 1 | Modelica.Mechanics.Translational.Examples.PreLoad |

## per sub-library
| sub-library | models | emit | parse | convert | simplify | solve | pass | top failure (count) |
|---|---|---|---|---|---|---|---|---|
| Electrical | 206 | 166 | 142 | 53 | 38 | 31 | 28 | BaseModelica.jl: array variables unsupported (array-preserving output) (73) |
| Clocked | 77 | 3 | 3 | 1 | 1 | 1 | 1 | Clock/synchronous features (Clock, superSample, hold, ...) (62) |
| Mechanics | 75 | 71 | 31 | 17 | 17 | 17 | 13 | rumoca: member access on a function-call result `f(...).'x'` is not Base Modelica (record-valued calls) (40) |
| Magnetic | 69 | 45 | 9 | 5 | 4 | 3 | 3 | rumoca: member access on a function-call result `f(...).'x'` is not Base Modelica (record-valued calls) (36) |
| Blocks | 31 | 26 | 26 | 5 | 5 | 5 | 4 | BaseModelica.jl: array variables unsupported (array-preserving output) (19) |
| Fluid | 23 | 16 | 0 | 0 | 0 | 0 | 0 | rumoca: member access on a function-call result `f(...).'x'` is not Base Modelica (record-valued calls) (16) |
| Media | 21 | 4 | 0 | 0 | 0 | 0 | 0 | rumoca: ED019 other unsupported Flat construct (function arrays/records/loops) (8) |
| Thermal | 16 | 16 | 16 | 2 | 2 | 2 | 1 | BaseModelica.jl: assert() in initial algorithm rejected (10) |
| StateGraph | 7 | 7 | 7 | 0 | 0 | 0 | 0 | BaseModelica.jl: array variables unsupported (array-preserving output) (7) |
| Math | 4 | 2 | 2 | 0 | 0 | 0 | 0 | rumoca: unevaluable array dimensions (2) |
| Utilities | 3 | 2 | 2 | 0 | 0 | 0 | 0 | BaseModelica.jl: array variables unsupported (array-preserving output) (2) |
| ComplexBlocks | 2 | 1 | 1 | 1 | 1 | 1 | 1 | rumoca: ED019 other unsupported Flat construct (function arrays/records/loops) (1) |

## emit by target
emit_array {'ok': 359, 'fail': 175}
emit_scalar {'fail': 402, 'ok': 131, 'timeout': 1}
(array, scalar): {('ok', 'fail'): 227, ('ok', 'ok'): 131, ('fail', 'fail'): 175, ('ok', 'timeout'): 1}
