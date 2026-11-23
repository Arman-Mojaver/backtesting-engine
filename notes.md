Before you go to production, remember that in the calculation of the roi and dd_max,
you need to consider the exit_date of each operation, you can not assume that the operations 
end in the order they were entered

Only the zone exit (Threshold) is implemented for Stochastic; 
a %K/%D cross would be a second Crossover-style variant, 
and a cross inside the zones needs new signal semantics

Only the zero cross (Oscillator) is implemented for CCI;
its classic +100/-100 zone exit needs negative thresholds, which Threshold does not support yet