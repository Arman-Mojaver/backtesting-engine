---
name: deep-dive
description: >
  Analyze the last commit and generate a runnable walkthrough script that helps developers
  understand implementation details. Creates instantiations of new/modified structures with
  seed data and print statements. Use when user says "deep-dive", "deep dive", "understand
  the implementation", "walkthrough", "show me how this works", "help me understand this
  commit", or invokes /deep-dive.
---

# Deep Dive Skill

Generate an educational walkthrough script for the last commit to help developers understand implementation details beyond the feature description.

## When to Use

- After implementing a feature
- When you understand what a feature does but not how it's implemented
- When you need hands-on understanding of code changes
- To learn from recent commits by seeing real instantiations

## Process

### Step 1: Check for Uncommitted Changes

**IMPORTANT**: Before analyzing, check `git status` for uncommitted changes.

If uncommitted changes exist:
```
⚠️  WARNING: You have uncommitted changes.

This skill may add temporary print statements to your codebase for better understanding.
These will be mixed with your current uncommitted changes.

Please choose:
1. Commit your current changes first
2. Stash your changes (git stash)
3. Acknowledge and proceed (print statements will be added to existing changes)

What would you like to do?
```

Wait for user confirmation before proceeding. If they choose option 3, continue with awareness that changes will be mixed.

### Step 2: Analyze the Last Commit

Run:
```bash
git show HEAD --stat
git show HEAD
```

Identify:
- New classes, structs, functions, and data structures added
- Existing structures that were modified
- The programming language(s) involved
- Key methods and their purposes
- Dependencies and relationships between structures

### Step 3: Generate Walkthrough Script

Create a standalone script (saved to `examples/{timestamp}.{ext}`) that:

**Directory convention:** Always save walkthrough scripts to the `examples/` directory (e.g., `examples/deep-dive-001-helper-functions.rs`). Cargo auto-discovers `.rs` files in `examples/` — no `Cargo.toml` changes needed. Use descriptive filenames: `examples/deep-dive-{NNN}-{slug}.rs`. When done, simply delete the file: `rm examples/deep-dive-*.rs`.


**IMPORTANT: Split Large Changes into Multiple Scripts**

If the commit contains extensive changes (>200 lines, multiple unrelated features, or complex multi-component changes), split the walkthrough into multiple focused scripts instead of one massive script.

**Why?** A 1000-line script that takes 2 hours to understand defeats the purpose of learning. Each script should be digestible in 10-15 minutes.

**How to split:**
- **By component**: One script per class/module/struct
- **By feature**: Group related changes that work together
- **By abstraction level**: Separate high-level API from low-level internals
- **By workflow**: One script per complete workflow/use case

**Script size guidelines:**
- **Target**: 50-150 lines per script
- **Maximum**: 200 lines (split if larger)
- **Execution time**: 2-5 minutes to run
- **Reading time**: 10-15 minutes to understand line-by-line

**Output format for multiple scripts:**
```
## Deep Dive Walkthrough Generated

📊 Commit: abc1234 - Add backtesting strategy system
📏 Total changes: 450 lines across 8 files
📦 Split into 3 focused scripts (10-15 min each)

### Script 1: Strategy Validation (Core Logic)
📄 examples/20260807-001-strategy-validation.rs
Focus: StrategyValidator struct and validation rules
Run: cargo run --example deep-dive-001-strategy-validation

### Script 2: Strategy Execution (Main Flow)
📄 examples/20260807-002-strategy-execution.rs
Focus: StrategyRunner and execution flow
Run: cargo run --example deep-dive-002-strategy-execution

### Script 3: Strategy Results (Data Layer)
📄 examples/20260807-003-strategy-results.rs
Focus: BacktestResult struct and result aggregation
Run: cargo run --example deep-dive-003-strategy-results

### Suggested order:
1. Start with Script 1 (foundation concepts)
2. Then Script 2 (main workflow)
3. Finally Script 3 (supporting features)
```

**When NOT to split:**
- Small, focused changes (<100 lines)
- Single class/struct with related methods
- Changes that are tightly coupled and hard to separate

#### A. Includes Hardcoded Seed Data
```rust
// Seed data for walkthrough
let raw_points = vec![
    RawPoint { timestamp: 1000, price: 100.0, volume: 500 },
    RawPoint { timestamp: 1001, price: 101.5, volume: 450 },
    RawPoint { timestamp: 1002, price: 99.8, volume: 600 },
];

let config = StrategyConfig {
    indicator_period: 14,
    threshold: 0.5,
};
```

#### B. Instantiates All Changed Structures at Multiple Levels
- **Top-level**: Create instances of public structs/enums that were added or modified
- **Internal components**: Instantiate internal helper structs and access internal state
- **Dependencies**: Show how internal components are created and used by the main structures
- **Different abstraction levels**: Demonstrate both high-level usage and low-level internals

Example:
```rust
// Top-level: Public API
let mut generator = SignalGenerator::new(config);
println!("Created SignalGenerator with config: {:?}", generator.config);

// Internal level: Access internal state (if pub(crate) or via accessor)
println!("Internal indicator cache: {:?}", generator.indicator_cache);
println!("Helper struct: {:?}", generator.signal_filter);

// Dependency level: Show how internal components work
let indicator = MovingAverage::new(14);
println!("Indicator internal state: {:?}", indicator.window);
let value = indicator.calculate(&raw_points);
println!("Indicator calculated value: {}", value);
```

#### C. Traverses Different Code Paths
- **Happy path**: Normal, expected usage
- **Edge cases**: Boundary conditions, empty inputs, Option::None
- **Error conditions**: Invalid inputs, Result::Err, panic scenarios
- **Branching logic**: All if/else branches, match arms, pattern matches
- **Early returns**: Conditions that cause early exits with ? operator

Example:
```rust
// Happy path
println!("=== HAPPY PATH: Valid signal generation ===");
let mut generator = SignalGenerator::new(config.clone());
let signal1 = generator.generate(&raw_points);
println!("Generated signal: {:?}", signal1);

// Edge case: Empty data
println!("\n=== EDGE CASE: Empty price data ===");
let empty_points: Vec<RawPoint> = vec![];
let signal2 = generator.generate(&empty_points);
println!("Generated signal: {:?}", signal2);

// Error condition: Insufficient data
println!("\n=== ERROR CONDITION: Insufficient data for indicator ===");
let short_points = vec![RawPoint { timestamp: 1000, price: 100.0, volume: 500 }];
let result = generator.generate_with_validation(&short_points);
match result {
    Ok(signal) => println!("Signal: {:?}", signal),
    Err(e) => println!("Error: {}", e),
}

// Branching: Different signal types
println!("\n=== BRANCHING: Different signal types ===");
for signal_type in [SignalType::Buy, SignalType::Sell, SignalType::Hold] {
    let signal = Signal::new(signal_type, 100.0, 0.8);
    match signal {
        Signal::Buy { price, confidence } => println!("BUY at {} with {} confidence", price, confidence),
        Signal::Sell { price, confidence } => println!("SELL at {} with {} confidence", price, confidence),
        Signal::Hold => println!("HOLD - no action"),
    }
}
```

#### D. Includes Internal/Private Methods
Even if methods are not publicly exposed (private or pub(crate)), include them to show:
- Internal validation logic
- Private helper methods (marked with _ or pub(crate))
- Internal state transformations
- Hidden decision points

Example:
```rust
// Access internal methods (even if private or pub(crate))
// Note: May need to add #[cfg(test)] or make pub(crate) for testing
println!("Calling internal validate_signal()...");
let is_valid = generator.validate_signal(&signal);
println!("  → Internal validation returned: {}", is_valid);

println!("\nCalling internal calculate_confidence()...");
let confidence = generator.calculate_confidence(&raw_points);
println!("  → Internal confidence calculation: {}", confidence);

println!("\nChecking internal state...");
println!("  → signal_count: {}", generator.signal_count);
println!("  → last_error: {:?}", generator.last_error);
println!("  → indicator_cache len: {}", generator.indicator_cache.len());

// Access internal enum variants
println!("\nInternal SignalFilter state:");
println!("  → threshold: {}", generator.signal_filter.threshold);
println!("  → smoothing_window: {:?}", generator.signal_filter.smoothing_window);
```

#### E. Educational Comments
Add comments explaining:
- Why this structure exists
- What problem it solves
- How the pieces connect
- What to pay attention to

#### F. Interactive Numbered Sections

Every walkthrough script MUST be interactive — the developer chooses which section to run via a command-line argument.

**Structure:**
- `fn main()` parses `std::env::args()` and dispatches to a numbered section function
- Running without arguments (or with `list`) prints all available sections
- Sections are numbered sequentially starting from 1
- Each section is a standalone function that can be run independently

**Searchable titles:** Each section function MUST have a comment with its exact title, formatted so the developer can copy it from the terminal output and search the file to find the responsible code:

```rust
// ============================================================
// SECTION 1: Valid Price Data - Happy Path Signal Generation
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_valid_data() {
    println!("============================================================");
    println!("SECTION 1: Valid Price Data - Happy Path Signal Generation");
    println!("============================================================");
    // ... section logic (max 20-25 printed lines) ...
}
```

**Dispatch pattern:**
```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_valid_data(),
        "2" => section_2_insufficient_data(),
        "3" => section_3_empty_data(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-{timestamp} <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Valid Price Data: Happy path signal generation");
    println!("  2    - Insufficient Data: Validation failure handling");
    println!("  3    - Empty Data: Edge case with empty input");
}
```

**Why this matters:** When the developer sees output in the terminal, they can copy the section title, search the file, and jump straight to the code that generated it. No more hunting through a 200-line script.

#### G. Section Output Limit — Max 20-25 Printed Lines

Each section function must produce **no more than 20-25 lines** of terminal output (including the title banner and separator lines). This keeps each section focused and immediately digestible.

**Why?** If a section prints 50+ lines, the developer can't easily see what changed without scrolling. A tight output forces you to focus on what matters most.

**Counting printed lines:** Every `println!()` call = one line. Title banners (`====` line + title + `====` line) count as 3 lines.

**If a section needs more than 25 lines:**
- Split it into two numbered sections (e.g., Section 4a and 4b)
- Remove verbose explanatory output — let code comments carry the explanation
- Move less-critical output into a separate section

#### H. CLI/API Detection — Print Instructions, Don't Execute

If the commit introduces or modifies a **CLI command** or **API endpoint**, the walkthrough script must **NOT** execute those commands or make real HTTP calls. Instead, it prints the exact commands the developer should run themselves.

**Why?** CLI commands may have side effects (writing files, modifying state). API calls hit real servers. The walkthrough script must be safe and side-effect-free.

**Detection rules — a commit is flagged as CLI/API if it touches:**
- `clap`/`structopt` command structs or `#[derive(Parser)]`
- Route handlers (`actix-web`, `axum`, `warp`) or `#[get("/...")]`, `#[post("/...")]`
- `main.rs` argument parsing
- Any handler that reads from `std::env::args()` or takes `Args` as input

**What the section does instead:**
- Prints the exact CLI command or curl command to run (formatted for copy-paste)
- Shows expected input and output
- Describes the flow step-by-step
- **Never** calls `Command::new(...).output()`, `reqwest::get(...)`, or similar

**Example for CLI:**
```rust
fn section_4_running_backtest_cli() {
    println!("Run this command in your terminal:");
    println!();
    println!("  backtest run --config configs/sma-cross.json --output ./results/");
    println!();
    println!("What happens:");
    println!("  1. Loads strategy config from configs/sma-cross.json");
    println!("  2. Fetches 500 days of OHLCV data for BTC/USD");
    println!("  3. Runs SMA crossover strategy (50-period / 200-period)");
    println!("  4. Writes BacktestResult to ./results/backtest-20260807.json");
}
```

**Example for API:**
```rust
fn section_5_create_strategy_api() {
    println!("Start the server first:");
    println!();
    println!("  cargo run -- serve --port 8080");
    println!();
    println!("Then send a POST request:");
    println!();
    println!("  curl -X POST http://localhost:8080/api/strategies \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{\"name\": \"SMA Cross\", \"params\": {\"fast\": 50, \"slow\": 200}}'");
    println!();
    println!("Expected response (201 Created):");
    println!("  {");
    println!("    \"id\": \"strat_abc123\",");
    println!("    \"name\": \"SMA Cross\",");
    println!("    \"created_at\": \"2026-08-07T12:00:00Z\"");
    println!("  }");
}
```


### Step 4: Optionally Add Print Statements to Codebase

If the commit modifies complex logic that benefits from runtime visibility:

1. Add temporary println!/log statements directly into the modified functions
2. Mark them clearly: `println!("DEBUG [deep-dive]: {}", value);`
3. Note in the output that temporary debug statements were added
4. Provide instructions to remove them after understanding

**Only do this if:**
- User acknowledged uncommitted changes (Step 1)
- The code structure makes it hard to understand without runtime visibility
- The benefit outweighs the cleanup effort

### Step 5: Provide Instructions

Output should include:

```
## Deep Dive Walkthrough Generated

📄 Script: examples/20260807-123456.rs

### What this script does:
[2-3 sentence explanation]

### How to run:
```bash
cargo run --example deep-dive-20260807-123456
```

### What you'll see:
- Instantiation of [structure names]
- [Key operations being demonstrated]
- Output showing [what the developer will learn]

### After running:
- Remove temporary debug statements (if added to codebase)
- Delete the script when done: rm examples/20260807-123456.rs
```

## Examples

### Example 1: New Rust Struct (Interactive Sections)

**Commit adds:**
```rust
pub struct SignalGenerator {
    config: GeneratorConfig,
    signal_count: u32,
}

impl SignalGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            config,
            signal_count: 0,
        }
    }
    
    fn validate_points(&self, points: &[RawPoint]) -> bool {
        points.len() >= self.config.min_points as usize
    }
    
    pub fn generate(&mut self, points: &[RawPoint]) -> Option<Signal> {
        if !self.validate_points(points) {
            return None;
        }
        
        self.signal_count += 1;
        Some(Signal {
            id: self.signal_count,
            price: points.last()?.price,
            signal_type: SignalType::Buy,
        })
    }
}
```

**Detection:** No CLI/API patterns → standard interactive script with numbered sections.

**Generated script:**
```rust
// examples/20260807-signal-generator.rs
//! Deep Dive: SignalGenerator Implementation
//! Purpose: Generates trading signals from raw price points with validation

use std::env;
use backtesting_engine::application::signal_generator::{SignalGenerator, GeneratorConfig};
use backtesting_engine::domain::{RawPoint, Signal, SignalType};

// ---- Shared test fixtures ----
fn make_config() -> GeneratorConfig {
    GeneratorConfig { min_points: 3, threshold: 0.5 }
}
fn make_valid_points() -> Vec<RawPoint> {
    vec![
        RawPoint { timestamp: 1000, price: 100.0, volume: 500 },
        RawPoint { timestamp: 1001, price: 101.5, volume: 450 },
        RawPoint { timestamp: 1002, price: 102.0, volume: 600 },
    ]
}
fn make_insufficient_points() -> Vec<RawPoint> {
    vec![RawPoint { timestamp: 1000, price: 100.0, volume: 500 }]
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_valid_data(),
        "2" => section_2_insufficient_data(),
        "3" => section_3_empty_data(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20260807-signal-generator <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Valid Price Data: Happy path signal generation");
    println!("  2    - Insufficient Data: Validation failure handling");
    println!("  3    - Empty Data: Edge case with empty input");
}

// ============================================================
// SECTION 1: Valid Price Data - Happy Path Signal Generation
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_valid_data() {
    println!("============================================================");
    println!("SECTION 1: Valid Price Data - Happy Path Signal Generation");
    println!("============================================================");

    let config = make_config();
    let points = make_valid_points();

    println!("Config: min_points={}, threshold={}", config.min_points, config.threshold);
    println!("Input: {} price points", points.len());

    let mut gen = SignalGenerator::new(config);
    println!("Created SignalGenerator. signal_count: {}", gen.signal_count);

    let signal = gen.generate(&points);
    println!("generate() returned: {:?}", signal);
    println!();
    println!("  -> Validation passed (3 points >= min_points: 3)");
    println!("  -> Signal generated with price: {:.2}", points.last().unwrap().price);
    println!("  -> signal_count incremented to: {}", gen.signal_count);
}

// ============================================================
// SECTION 2: Insufficient Data - Validation Failure
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_insufficient_data() {
    println!("============================================================");
    println!("SECTION 2: Insufficient Data - Validation Failure");
    println!("============================================================");

    let config = make_config();
    let points = make_insufficient_points();

    println!("Config: min_points={} (requires at least {} points)", config.min_points, config.min_points);
    println!("Input: {} price point", points.len());

    let mut gen = SignalGenerator::new(config);
    let signal = gen.generate(&points);
    println!("generate() returned: {:?}", signal);
    println!();
    println!("  -> Validation FAILED (1 point < min_points: 3)");
    println!("  -> Returns None - no signal generated");
    println!("  -> signal_count unchanged: {}", gen.signal_count);
}

// ============================================================
// SECTION 3: Empty Data - Edge Case
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_empty_data() {
    println!("============================================================");
    println!("SECTION 3: Empty Data - Edge Case");
    println!("============================================================");

    let config = make_config();
    let points: Vec<RawPoint> = vec![];

    println!("Config: min_points={}", config.min_points);
    println!("Input: {} price points", points.len());

    let mut gen = SignalGenerator::new(config);
    let signal = gen.generate(&points);
    println!("generate() returned: {:?}", signal);
    println!();
    println!("  -> Validation FAILED (0 points < min_points: 3)");
    println!("  -> Returns None");
    println!("  -> signal_count unchanged: {}", gen.signal_count);
}
```

### Example 2: Modified Function

**Commit modifies:**
```rust
pub fn calculate_position_size(
    account_balance: f64,
    risk_per_trade: f64,
    strategy_type: StrategyType,
) -> f64 {
    // Added aggressive strategy support
    match strategy_type {
        StrategyType::Conservative => account_balance * risk_per_trade * 0.5,
        StrategyType::Moderate => account_balance * risk_per_trade,
        StrategyType::Aggressive => account_balance * risk_per_trade * 1.5, // NEW!
    }
}
```

**Generated script:**
```rust
// examples/20260807-position-sizing.rs
//! Deep Dive: calculate_position_size Changes
//! Purpose: Added aggressive strategy support (1.5x position size) alongside existing strategies

use backtesting_engine::application::position_sizing::calculate_position_size;
use backtesting_engine::domain::StrategyType;

fn main() {
    println!("{}", "=".repeat(60));
    println!("DEEP DIVE: calculate_position_size Enhancement");
    println!("{}", "=".repeat(60));

    // Seed data
    let account_balance = 10_000.0;
    let risk_per_trade = 0.02; // 2%

    // Test all strategy types
    let test_cases = vec![
        (StrategyType::Conservative, "Conservative - 50% of base risk"),
        (StrategyType::Moderate, "Moderate - 100% of base risk"),
        (StrategyType::Aggressive, "Aggressive - 150% of base risk (NEW!)"),
    ];

    for (strategy_type, description) in test_cases {
        println!("\n🎯 {}", description);
        println!("{}", "-".repeat(60));
        println!("Input: balance=${:.2}, risk={:.1}%, strategy={:?}", 
                 account_balance, risk_per_trade * 100.0, strategy_type);
        
        let position_size = calculate_position_size(
            account_balance,
            risk_per_trade,
            strategy_type,
        );
        
        println!("Output: ${:.2}", position_size);
        
        match strategy_type {
            StrategyType::Conservative => {
                let expected = account_balance * risk_per_trade * 0.5;
                println!("  → Conservative: {:.2} * {:.2} * 0.5 = {:.2}", 
                         account_balance, risk_per_trade, expected);
            }
            StrategyType::Moderate => {
                let expected = account_balance * risk_per_trade;
                println!("  → Moderate: {:.2} * {:.2} = {:.2}", 
                         account_balance, risk_per_trade, expected);
            }
            StrategyType::Aggressive => {
                let expected = account_balance * risk_per_trade * 1.5;
                println!("  → Aggressive: {:.2} * {:.2} * 1.5 = {:.2}", 
                         account_balance, risk_per_trade, expected);
            }
        }
    }

    // Edge case: Zero balance
    println!("\n⚠️  EDGE CASE: Zero account balance");
    println!("{}", "-".repeat(60));
    let zero_balance = 0.0;
    let position = calculate_position_size(zero_balance, risk_per_trade, StrategyType::Aggressive);
    println!("Input: balance=${:.2}, risk={:.1}%, strategy=Aggressive", 
             zero_balance, risk_per_trade * 100.0);
    println!("Output: ${:.2}", position);
    println!("  → Formula still works: 0.0 * 0.02 * 1.5 = 0.0");

    println!("\n{}", "=".repeat(60));
    println!("WHAT CHANGED:");
    println!("- Added Aggressive strategy tier with 1.5x multiplier");
    println!("- Conservative remains at 0.5x (half position size)");
    println!("- Moderate remains at 1.0x (base position size)");
    println!("- Aggressive uses 1.5x (50% larger positions)");
    println!("{}", "=".repeat(60));
}
```

### Example 3: New CLI Command (CLI Detection)

**Commit adds:**
```rust
#[derive(Parser)]
pub struct RunCommand {
    #[arg(short, long)]
    pub config: String,
    #[arg(short, long, default_value = "./results/")]
    pub output: String,
}

impl RunCommand {
    pub fn execute(&self) -> Result<BacktestResult> {
        let config = StrategyConfig::from_file(&self.config)?;
        let runner = BacktestRunner::new(config);
        let result = runner.run()?;
        result.save_to(&self.output)?;
        Ok(result)
    }
}
```

**Detection:** This commit touches `#[derive(Parser)]` and a CLI run command → **CLI detected**. The script MUST print instructions, never execute.

**Generated script:**
```rust
// examples/20260807-backtest-cli.rs
//! Deep Dive: backtest run CLI Command
//! Purpose: Demonstrates the new CLI interface for running backtests
//! WARNING: CLI detected - this script prints commands, never executes them

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match section {
        "1" => section_1_basic_backtest(),
        "2" => section_2_config_options(),
        "3" => section_3_error_handling(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20260807-backtest-cli <section>");
    println!();
    println!("  CLI DETECTED - all sections show commands to run manually");
    println!();
    println!("  list - Show this list");
    println!("  1    - Basic Backtest: Run with minimal config");
    println!("  2    - Config Options: All CLI flags explained");
    println!("  3    - Error Handling: Missing or invalid config files");
}

// ============================================================
// SECTION 1: Basic Backtest - Run with Minimal Config
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_basic_backtest() {
    println!("============================================================");
    println!("SECTION 1: Basic Backtest - Run with Minimal Config");
    println!("============================================================");
    println!();
    println!("Run this command in your terminal:");
    println!();
    println!("  backtest run --config configs/sma-cross.json");
    println!();
    println!("What happens:");
    println!("  1. Parses CLI args via clap:");
    println!("     --config = \"configs/sma-cross.json\"");
    println!("     --output = \"./results/\" (default)");
    println!("  2. Loads StrategyConfig from the config file");
    println!("  3. Runs the simulation on historical data");
    println!("  4. Saves BacktestResult JSON to ./results/");
    println!();
    println!("Expected terminal output:");
    println!("  [INFO] Loading config: configs/sma-cross.json");
    println!("  [INFO] Strategy: SMA Cross (50/200)");
    println!("  [INFO] Running backtest... done (2.3s)");
    println!("  [INFO] 23 trades | Sharpe 1.42 | MaxDD -12.3%");
}

// ============================================================
// SECTION 2: Config Options - All CLI Flags
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_config_options() {
    println!("============================================================");
    println!("SECTION 2: Config Options - All CLI Flags");
    println!("============================================================");
    println!();
    println!("Full command with all flags:");
    println!();
    println!("  backtest run \\");
    println!("    --config configs/sma-cross.json \\");
    println!("    --output ./my-results/ \\");
    println!("    --log-level debug \\");
    println!("    --dry-run");
    println!();
    println!("Flag reference:");
    println!("  -c, --config PATH     Strategy config file (required)");
    println!("  -o, --output DIR      Results directory (default: ./results/)");
    println!("  --log-level LEVEL     Log verbosity: trace|debug|info|warn|error");
    println!("  --dry-run             Validate config without running");
    println!();
    println!("Tip: Use --dry-run to validate config syntax before a full run.");
}

// ============================================================
// SECTION 3: Error Handling - Missing Config
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_error_handling() {
    println!("============================================================");
    println!("SECTION 3: Error Handling - Missing Config");
    println!("============================================================");
    println!();
    println!("Try these error scenarios in your terminal:");
    println!();
    println!("1. MISSING CONFIG FILE:");
    println!("   backtest run --config nonexistent.json");
    println!();
    println!("   Expected error:");
    println!("   Error: config file not found: nonexistent.json");
    println!();
    println!("2. NO CONFIG FLAG:");
    println!("   backtest run");
    println!();
    println!("   Expected error:");
    println!("   error: the following required arguments were not provided:");
    println!("     --config <CONFIG>");
    println!();
    println!("3. INVALID JSON:");
    println!("   backtest run --config malformed.json");
    println!();
    println!("   Expected error:");
    println!("   Error: failed to parse config: malformed.json");
    println!("   Caused by: expected value at line 5 column 1");
}
```

## Language Detection

This is a **Rust project**. All walkthrough scripts should be written in Rust:

- **Primary**: `.rs` files with `println!()` macros
- **Execution**: `cargo run --example <script-name>` or compile with `rustc`
- **Dependencies**: Use `use` statements to import from the project's modules
- **Formatting**: Use Rust's Debug trait (`{:?}`) for structs/enums, Display (`{}`) for simple values

**Note**: If the commit includes other languages (e.g., Python scripts, shell scripts), create separate walkthrough scripts for each language, but prioritize Rust for the core backtesting engine code.

## Best Practices

1. **Make it runnable** - The script should work immediately without additional setup
2. **Interactive sections** - Use numbered sections (`./script 1`, `./script 2`) with arg dispatch
3. **Searchable titles** - Every section prints an exact title the developer can copy and search
4. **Limit output** - Each section produces no more than 20-25 printed lines
5. **Use realistic data** - Seed data should resemble real usage
6. **Show edge cases** - Demonstrate both happy paths and edge cases
7. **Explain the "why"** - Not just what happens, but why it matters
8. **Keep it focused** - Only demonstrate changes from the commit, not the entire codebase
9. **CLI/API safety** - Print commands to run, never execute CLI or make real API calls
10. **Mark temporary changes** - If adding debug statements, mark them clearly for cleanup

## Output Format

```
## Deep Dive Walkthrough Generated

📄 Script: examples/{timestamp}.rs
📊 Commit: {commit-hash} - {commit-message}
🔧 Changes analyzed: {number} files modified

### What this script demonstrates:
[Clear explanation of what the developer will learn]

### How to run:
```bash
# Run all sections (shows the menu):
cargo run --example deep-dive-{timestamp}

# Run a specific section:
cargo run --example deep-dive-{timestamp} 1
cargo run --example deep-dive-{timestamp} 2
cargo run --example deep-dive-{timestamp} 3
```

### Available sections:
1. [Section title — copy this to search the file!]
2. [Section title — copy this to search the file!]
3. [Section title — copy this to search the file!]

### CLI/API detection:
[If detected: "CLI/API commands detected — script prints instructions, never executes"]

### Temporary changes:
[If debug statements were added to codebase, list them here with file paths]

### After understanding:
Delete walkthrough scripts: `rm examples/deep-dive-*.rs`
```

## When NOT to Use

- For trivial changes (typo fixes, comment updates)
- When you already understand the implementation
- For refactoring with no behavioral changes (unless complex)
- When the commit is too large (>500 lines changed) - suggest breaking it down first

---

**Deep Dive** - Bridge the gap between "what it does" and "how it works"!
