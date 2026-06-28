use crate::application::strategy_generator::generate_random_strategies_bulk;
use crate::config::environment::parquet_base_dir;
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

use super::resolve_environment;

pub fn execute(
    count: u64,
    instruments: Vec<String>,
    mms_count: u32,
    max_op_points: u32,
    seed: Option<u64>,
    parquet_dir: Option<String>,
) -> anyhow::Result<()> {
    let base_dir = match parquet_dir {
        Some(dir) => dir,
        None => parquet_base_dir(resolve_environment()),
    };
    let repo = ParquetStrategyRepository::new(base_dir.clone());

    let partitions = generate_random_strategies_bulk(
        count as usize,
        &instruments,
        mms_count as usize,
        max_op_points as usize,
        seed,
    );

    let total_strategies: usize = partitions.iter().map(|(_, _, s)| s.len()).sum();
    let total_partitions = partitions.len();

    eprintln!(
        "Generating {} strategies across {} partitions ...",
        total_strategies, total_partitions,
    );

    for (instrument, mms_id, strategies) in &partitions {
        repo.write(instrument, *mms_id, strategies)?;
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Generate complete");
    println!("  Base dir:       {}", base_dir);
    println!("  Total rows:     {}", total_strategies);
    println!("  Partitions:     {}", total_partitions);
    for (inst, mms, strs) in &partitions {
        println!("    {}/mms_id={}: {} rows", inst, mms, strs.len());
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    Ok(())
}
