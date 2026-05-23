# CONTEXT.md Format

A `CONTEXT.md` is a living glossary of the bounded context's domain language. It captures terms as they are resolved during a grilling session. Keep it close to the domain experts' language — no implementation details.

---

## Format

```markdown
# [Context Name] Context

> One sentence describing what this context is responsible for.

## Glossary

### [Term]

[Definition in domain language. Precise. No implementation details.]

**Distinguishes from**: [Other terms it might be confused with, and how]
**Example**: [A concrete, realistic example]

### [Term]
...
```

---

## Worked Example

The following is a real example from this project's KPI domain:

```markdown
# SF1000 Context

> Calculates the SF1000 KPI: a warranty claims rate per 1000 vehicles
> for a given Months in Service (MIS) window.

## Glossary

### Warranty Part

A single warranty claim event for a specific part on a specific vehicle,
filed by a partner in a specific country.

**Distinguishes from**: Vehicle — one Vehicle may have many Warranty Parts.
**Example**: VIN ABC123, partner P1, claimed in Germany on 2023-07-02, MIS 8.

### Vehicle

A unique vehicle identified by its VIN hash, derived by grouping all
Warranty Parts that share the same VIN hash.

**Distinguishes from**: Warranty Part — a Vehicle is a logical grouping,
not a raw claim record.

### MIS (Months In Service)

The number of months between a vehicle's delivery date and its claim date.
Determines which "age band" a vehicle falls into for the SF1000 calculation.

**Distinguishes from**: Delay — MIS measures vehicle age; Delay measures
reporting lag.
**Example**: Delivered Jan 2023, claimed Jul 2023 → MIS = 6.

### Delay

The average lag (in months) between a claim date and the date the data
was uploaded into the system. Used to adjust the censoring date per vehicle.

**Distinguishes from**: MIS — Delay is about data latency, not vehicle age.

### Censoring Date

The effective observation cutoff date for a vehicle. A vehicle's warranty
claims after its censoring date are not yet observable and must be excluded.

Computed as: max(current_date − delay, newest_claim_date_for_vehicle).

**Distinguishes from**: Data Upload Date — the censoring date is a derived
per-vehicle value; the upload date is a raw field on each Warranty Part.

### Modified MIS

The fractional MIS a vehicle has been exposed to, calculated from its
censoring date and delivery date. Used to determine whether a vehicle is
eligible to enter the reference population for a given MIS band.

**Example**: Delivered Jan 2023, censoring date Aug 2023 → modified MIS ≈ 7.

### Reference Population

The set of vehicles that have been "on the road" long enough to have had
the opportunity to generate a claim at a given MIS. A vehicle enters the
reference population for MIS N when its modified MIS ≥ N.

**Distinguishes from**: Dynamical Reference Population — the vehicles in
the reference population that actually have a Warranty Part at exactly MIS N.

### SF1000

The KPI value: number of claims at MIS N per 1000 vehicles in the reference
population for MIS N.

Formula: (dynamical_reference_population_count / vehicle_reference_population_count) × 1000

Computed per MIS value; the total SF1000 is the sum across all MIS values.
```
