// model: FunctionLocalRecordConstant.M
package FunctionLocalRecordConstant "A function-local declaration binding reads a field of a package record constant (MSL: Media.Air.ReferenceMoistAir.Utilities.Ice09_Utilities.Basic.Gibbs in ReferenceAir.Inverse_sh_TX, MoistAir1, MoistAir2)"
  record Base
    Real pred;
  end Base;
  record IceConstants
    extends Base;
    Real p0;
  end IceConstants;
  constant IceConstants Constants(pred = 611.657, p0 = 101325);
  function gibbs
    input Real p;
    output Real g;
  protected
    final Real pi0 = Constants.p0/Constants.pred;
  algorithm
    g := p*pi0;
  end gibbs;
  model M
    Real y = gibbs(time);
    annotation(experiment(StopTime = 1));
  end M;
end FunctionLocalRecordConstant;
