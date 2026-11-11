model BouncingBall "Ball bouncing on the ground with restitution"
  parameter Real e = 0.8 "Coefficient of restitution";
  parameter Real g = 9.81 "Gravity";
  Real h(start = 1.0, fixed = true) "Height";
  Real v(start = 0.0, fixed = true) "Velocity";
  discrete Integer bounces(start = 0, fixed = true) "Number of bounces";
  discrete Real tLast(start = 0.0, fixed = true) "Time of last bounce";
equation
  der(h) = v;
  der(v) = -g;
  when h < 0 then
    reinit(v, -e * pre(v));
    bounces = pre(bounces) + 1;
    tLast = time;
  end when;
end BouncingBall;
