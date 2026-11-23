Before you go to production, remember that in the calculation of the roi and dd_max,
you need to consider the exit_date of each operation, you can not assume that the operations 
end in the order they were entered

Possible performance improvement: max_dd in PerformanceMetrics::from_operation_points() only ever grows, so once
rounded_percent(max_dd) exceeds max_draw_down_max the strategy is certain to fail admits_performance(),
and the loop could stop early instead of finishing the metrics and building a Strategy that gets dropped.
