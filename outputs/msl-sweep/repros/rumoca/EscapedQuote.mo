model EscapedQuote "A description string with escaped quotes (MSL: Machines TerminalBox)"
  parameter String connection = "D" "Choose \"Y\" for star or \"D\" for delta connection";
  Real x(start = 1, fixed = true);
equation
  der(x) = -x;
  annotation(experiment(StopTime = 1));
end EscapedQuote;
