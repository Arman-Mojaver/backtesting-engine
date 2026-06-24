use std::hint::black_box;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// A subset "survives" if the AND-ed mask has at least this many 1-bits.
/// In domain terms: the selected indicators must fire together on at least
/// MIN_POPCOUNT rows (timestamps) for the subset to be viable.
pub const MIN_POPCOUNT: u32 = 10;

pub struct BenchConfig {
    /// Per-column densities. Length determines column_count.
    /// e.g. [0.05, 0.90, 0.50] → 3 columns: 5% sparse, 90% baseline, 50% coinflip.
    pub densities: Vec<f64>,
    pub min_popcount: u32,
    pub seed: Option<u64>,
}

pub fn run_benchmark(config: BenchConfig) {
    let columns = generate_columns(&config.densities, config.seed);

    let ones_total: u32 = columns.iter().map(|c| c.count_ones()).sum();
    let actual_density = ones_total as f64 / (columns.len() * 128) as f64;

    eprintln!("╔══════════════════════════════════════════════════════════════╗");
    eprintln!("║  128-bit Bitwise DFS Enumeration Benchmark");
    eprintln!(
        "║  Columns: {} | Min popcount: {}",
        columns.len(),
        config.min_popcount
    );
    eprintln!("║  Avg density: {:.3}", actual_density);
    for (i, d) in config.densities.iter().enumerate() {
        let col_ones = columns[i].count_ones();
        eprintln!("║    col {:>2}: d={:.3} → {} ones", i, d, col_ones);
    }
    match subset_count(columns.len()) {
        Some(n) => {
            eprintln!("║  Total subsets: {}", n);
            if columns.len() > 24 {
                eprintln!("║  ⚠  This will take a while!");
            }
        }
        None => {
            eprintln!("║  Total subsets: (too large to represent)");
            eprintln!("║  ⚠  This will likely never finish!");
        }
    }
    eprintln!("╠══════════════════════════════════════════════════════════════╣");

    bench_dfs_full(&columns, config.min_popcount);
    bench_popcount_throughput(&columns);

    eprintln!("╚══════════════════════════════════════════════════════════════╝");
}

fn subset_count(n: usize) -> Option<u128> {
    if n >= 128 {
        return None;
    }
    Some((1u128 << n) - 1)
}

fn generate_columns(densities: &[f64], seed: Option<u64>) -> Vec<u128> {
    let mut rng: StdRng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => {
            let random_seed: u64 = rand::rng().random();
            StdRng::seed_from_u64(random_seed)
        }
    };

    densities
        .iter()
        .map(|&density| {
            let mut mask: u128 = 0;
            for bit in 0..128 {
                if rng.random::<f64>() < density {
                    mask |= 1u128 << bit;
                }
            }
            mask
        })
        .collect()
}

/// Brute-force: enumerate every non-empty column subset.
/// Each subset: AND the selected masks, then check popcount ≥ min_popcount.
fn bench_dfs_full(columns: &[u128], min_popcount: u32) {
    let mut surviving = 0u64;
    let mut nodes = 0u64;

    let start = Instant::now();
    dfs_enumerate(columns, 0, !0u128, min_popcount, &mut nodes, &mut surviving);
    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();

    let nodes_per_sec = if secs > 0.0 {
        nodes as f64 / secs
    } else {
        f64::INFINITY
    };

    eprintln!("║  DFS enumeration (all subsets)");
    eprintln!("║    Nodes visited:    {}", nodes);
    eprintln!(
        "║    Surviving (≥{}):   {} ({:.1}%)",
        min_popcount,
        surviving,
        (surviving as f64 / nodes as f64) * 100.0
    );
    eprintln!(
        "║    Throughput:       {:>10} nodes/sec",
        format_rate(nodes_per_sec),
    );
    eprintln!("║    Elapsed:          {:.3}s", secs);
}

fn dfs_enumerate(
    columns: &[u128],
    idx: usize,
    mask: u128,
    min_popcount: u32,
    nodes: &mut u64,
    surviving: &mut u64,
) {
    if idx >= columns.len() {
        return;
    }

    // Include this column
    let new_mask = mask & columns[idx];
    *nodes += 1;
    let pop = new_mask.count_ones();
    if pop >= min_popcount {
        *surviving += 1;
        dfs_enumerate(columns, idx + 1, new_mask, min_popcount, nodes, surviving);
    }
    // else: mask is too sparse — adding more columns can only remove 1s,
    //        so no deeper subset can ever reach min_popcount. Prune subtree.

    // Exclude this column — parent mask unchanged, still viable
    dfs_enumerate(columns, idx + 1, mask, min_popcount, nodes, surviving);
}

fn bench_popcount_throughput(columns: &[u128]) {
    const ITERATIONS: usize = 100_000;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for col in columns {
            black_box(col.count_ones());
        }
    }
    let elapsed = start.elapsed();
    let total_ops = columns.len() * ITERATIONS;
    let per_op = elapsed.as_secs_f64() / total_ops as f64;

    eprintln!("║  Popcount throughput");
    eprintln!("║    Ops:              {}", total_ops);
    eprintln!("║    Per op:           {:.2} ns", per_op * 1_000_000_000.0);
}

fn format_rate(rate: f64) -> String {
    if rate >= 1_000_000_000.0 {
        format!("{:.2}G", rate / 1_000_000_000.0)
    } else if rate >= 1_000_000.0 {
        format!("{:.2}M", rate / 1_000_000.0)
    } else if rate >= 1_000.0 {
        format!("{:.2}K", rate / 1_000.0)
    } else {
        format!("{:.0}", rate)
    }
}
