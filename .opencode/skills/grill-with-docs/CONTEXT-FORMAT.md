# Glossary Format (for `domain-model.md`)

The glossary in `.opencode/context/project-intelligence/domain-model.md` is a living dictionary of the project's domain language. It captures terms as they are resolved during a grilling session. Keep it close to the domain experts' language — no implementation details.

---

## Format

Each glossary term in `domain-model.md`'s **Ubiquitous Language** section follows this pattern:

```markdown
| **Term** | Definition in domain language. Precise. |
|-----------|------------------------------------------|

Or, for terms needing deeper explanation:

### [Term]

[Definition in domain language. Precise. No implementation details.]

**Distinguishes from**: [Other terms it might be confused with, and how]
**Example**: [A concrete, realistic example]
```

---

## Worked Example

The following is drawn from this project's `domain-model.md`:

```markdown
### OperationPoint

A single recorded trade operation (long or short) for a specific instrument.
Contains the trade direction (result), take-profit and stop-loss levels,
risk allocation, and the Money Management Strategy it belongs to.

**Distinguishes from**: ResampledPointD1 — an OperationPoint is a trade event;
a ResampledPointD1 is a price observation.
**Example**: Long EURUSD, result=+1, tp=10, sl=5, risk=0.01, mms_id=1, timestamp=1000.

### ResampledPointD1

A daily resampled OHLCV candle for a single instrument. Captures the open,
high, low, close prices, and volume.

**Distinguishes from**: OperationPoint — ResampledPointD1 represents market data,
not trade decisions.
**Example**: EURUSD O=1.05 H=1.10 L=0.95 C=1.08 V=1000, ts=1000.

### Instrument

A tradable forex pair or CFD (e.g. EURUSD, USDCAD). Every OperationPoint and
ResampledPointD1 is tagged with an instrument string, and cross-instrument
mixing is a validation error.

**Distinguishes from**: Underlying Asset — the Instrument is the trading pair
identifier, not the economic asset behind it.
**Example**: EURUSD, USDCAD.

### MMS (Money Management Strategy)

A strategy configuration that defines how capital is allocated across operations.
Each OperationPoint belongs to exactly one MMS, identified by
`money_management_strategy_id`. Strategy creation requires selecting which MMS
to use.

**Distinguishes from**: Trading Strategy — MMS governs risk allocation;
Trading Strategy governs entry/exit rules.
**Example**: mms_id=1 uses 1% risk per trade; mms_id=2 uses 2%.

### Indicator

A technical analysis indicator (MACD, RSI) with its parameters, used as a
signal source for strategy decisions. Indicators are identified by type and
a unique identifier string.

**Distinguishes from**: IndicatorParameters — Indicator is the entity with id/type/identifier;
IndicatorParameters is its embedded configuration variant.
**Example**: MACD(ema 26, ema 12), RSI(n=14, threshold=70/30).

### Validation

A pipeline of checks that ensures input data is consistent before strategy
computation. Validates that: no repo is empty, all indicators are the same type,
operation points and resampled points share the same instrument, all operations
belong to the expected MMS, and timestamps are aligned.

**Distinguishes from**: Business Logic — validation gates inputs; strategy
logic computes results.
**Example**: If long_ops use EURUSD but resampled points use USDCAD,
CombinedValidationError::InstrumentMismatch is returned.
```
