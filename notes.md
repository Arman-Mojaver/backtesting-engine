Use RUSTFLAGS="-C target-cpu=native" when releasing the binary, it optimises for M2 chips in your case
Before you go to production, remember that in the calculation of the roi and dd_max,
you need to consider the exit_date of each operation, you can not assume that the operations 
end in the order they were entered