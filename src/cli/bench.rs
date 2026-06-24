use clap::{value_parser, Arg, ArgMatches, Command};

use crate::engine::benchmark::{self, BenchConfig, MIN_POPCOUNT};

pub fn build_bench_cli() -> Command {
    Command::new("bench")
        .about("Run 128-bit bitwise DFS enumeration benchmarks (runs directly, no Docker)")
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .required(false)
                .value_parser(value_parser!(usize))
                .help("Number of columns with uniform density (ignored if --densities is set)")
                .conflicts_with("densities"),
        )
        .arg(
            Arg::new("density")
                .short('d')
                .long("density")
                .required(false)
                .default_value("0.3")
                .value_parser(value_parser!(f64))
                .help("Uniform density when using --count (default: 0.3)"),
        )
        .arg(
            Arg::new("densities")
                .long("densities")
                .required(false)
                .value_delimiter(',')
                .num_args(1..)
                .value_parser(value_parser!(f64))
                .help("Per-column densities, e.g. --densities 0.05,0.9,0.5,0.3"),
        )
        .arg(
            Arg::new("min-popcount")
                .short('m')
                .long("min-popcount")
                .required(false)
                .default_value(MIN_POPCOUNT.to_string())
                .value_parser(value_parser!(u32))
                .help("Minimum popcount for a subset to survive (default: 10)"),
        )
        .arg(
            Arg::new("seed")
                .short('s')
                .long("seed")
                .required(false)
                .value_parser(value_parser!(u64))
                .help("RNG seed for reproducible runs"),
        )
}

pub enum BenchCommand {
    Run(BenchConfig),
}

pub fn parse_bench(m: &ArgMatches) -> BenchCommand {
    let min_popcount = *m.get_one::<u32>("min-popcount").unwrap();
    let seed = m.get_one::<u64>("seed").copied();

    let densities = if let Some(dens) = m.get_many::<f64>("densities") {
        dens.copied().collect()
    } else {
        let count = *m.get_one::<usize>("count").unwrap_or(&10);
        let density = *m.get_one::<f64>("density").unwrap();
        vec![density; count]
    };

    BenchCommand::Run(BenchConfig {
        densities,
        min_popcount,
        seed,
    })
}

impl BenchCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            BenchCommand::Run(config) => {
                benchmark::run_benchmark(config);
                Ok(())
            }
        }
    }
}
