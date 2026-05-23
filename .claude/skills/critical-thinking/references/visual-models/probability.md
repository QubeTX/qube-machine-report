# Visual Models — Probability and Utility

Tools for analyzing decisions under uncertainty. Reach for these when **outcomes aren't
guaranteed** and you need to weigh probability and value together.

This file covers:

- **Probability Tree** — for chained uncertain events
- **Utility Tree** — when both probability and value matter (decision-maker's perspective)
- **Utility Matrix** — when you have many options and want compact analysis

These tools are technical. Use them when the stakes warrant the rigor. For lighter
analysis, Pros-Cons-and-Fixes or Weighted Ranking (see `comparison.md`) is usually
sufficient.

---

## Probability concepts (briefly)

Two key distinctions:

### Mutually exclusive vs. conditionally dependent events

- **Mutually exclusive** — events that cannot occur simultaneously. (A coin lands heads
  *or* tails.) Probabilities sum to 1.0.
- **Conditionally dependent** — outcome of one event affects another. (You succeed in
  the first interview *and then* succeed in the second.) Multiply probabilities along the
  branch.

### Subjective probability

When data is unavailable, probability is estimated. Be explicit about this — judgment-
based probabilities have real uncertainty themselves. Tag the confidence band when using
them.

### Use numerical, not adjectival

"Likely" means very different things to different people. When you assign probability,
use a number (or a percentile range): **30%**, **75%**, **5–10%**. Even rough numbers
beat fuzzy adjectives because they force the user to commit to a magnitude.

---

## Probability Tree

Use when **events chain** and you need to compute the probability of compound scenarios.

### The six steps (Morgan Jones)

1. **Identify the problem.**
2. **Define major decisions and events** — distinguish what you choose from what happens
   to you.
3. **Construct a scenario tree** of alternative scenarios.
4. **Assign probabilities** ensuring they sum to 1.0 at each branching node.
5. **Calculate conditional probabilities** for end-state scenarios — multiply along the
   path.
6. **Solve probability questions** based on the events.

### Markdown rendering

```markdown
### Decision: <what's being analyzed>

Pursue Path A (probability 0.6 of choosing)
├── Outcome X (P=0.7 given A) → Compound P = 0.6 × 0.7 = 0.42
└── Outcome Y (P=0.3 given A) → Compound P = 0.6 × 0.3 = 0.18

Pursue Path B (probability 0.4 of choosing)
├── Outcome X (P=0.5 given B) → Compound P = 0.4 × 0.5 = 0.20
└── Outcome Y (P=0.5 given B) → Compound P = 0.4 × 0.5 = 0.20

Total P(X) = 0.42 + 0.20 = 0.62
Total P(Y) = 0.18 + 0.20 = 0.38
(Sums to 1.0 ✓)
```

### When to use

- Decisions with chained uncertain outcomes
- Risk analysis — what's the probability of worst-case across all paths?
- Pre-mortem analysis — given my plan, what's the probability of failure modes?

### Pitfall

People are bad at probability. Estimates often violate basic rules (probabilities at a
node summing to more than 1.0, ignoring base rates, etc.). Sanity-check any tree where
totals don't sum cleanly.

---

## Utility Tree

Use when **probability matters AND outcomes have different values**. Combines the
probability tree with utility scores per outcome.

### Key concepts (Morgan Jones)

- **Utility** = the benefit (or cost) of an outcome from a specific perspective. Scored
  0–100 typically.
- **Options** = alternative actions you can take. Mutually exclusive.
- **Outcomes** = what results from each option. Should be collectively exhaustive.
- **Perspective** = whose viewpoint we're scoring from. Critical to name explicitly —
  the same outcome has different utility for different stakeholders.
- **Expected Value (EV)** = Probability × Utility. The decision criterion.

### The eight steps

1. **Identify the options and outcomes.**
2. **Determine the perspective** of the analysis. (CEO? Customer? You personally?)
3. **Construct a scenario tree** of options and outcomes.
4. **Assign utility values** (0–100) to each end-state, from the chosen perspective.
5. **Assign probabilities** to each outcome path, ensuring they sum to 1.0 per option.
6. **Calculate expected values** — multiply utility by probability for each scenario.
7. **Rank alternative options** based on summed expected values.
8. **Perform a sanity check** — does the ranking match intuition?

### Markdown rendering

```markdown
### Decision: Pursue acquisition vs. organic growth?
### Perspective: CEO

**Option 1: Acquire Company X**
├── Successful integration (P=0.5, U=90) → EV = 45
├── Partial success (P=0.3, U=50) → EV = 15
└── Failed integration (P=0.2, U=10) → EV = 2
**Total EV (Option 1):** 62

**Option 2: Organic growth**
├── Strong execution (P=0.4, U=70) → EV = 28
├── Average execution (P=0.5, U=50) → EV = 25
└── Weak execution (P=0.1, U=20) → EV = 2
**Total EV (Option 2):** 55

**Ranking:** Option 1 (acquire) > Option 2 (organic), 62 vs. 55
**Sanity check:** Close call. The ranking could flip if probability estimates shift
even slightly. <flag for further analysis>
```

### Why separating utility from probability matters

The mistake people make: confusing what they want (utility) with what they think will
happen (probability). Optimists overestimate the probability of high-utility outcomes.
Pessimists do the opposite. Separating the two forces honesty about both dimensions.

### When to use

- Decision-Making Step 5 — high-stakes decisions with uncertain outcomes
- Anticipating others' decisions (assess from *their* perspective; their actions reveal
  their utility function)

---

## Utility Matrix

Same analytical content as the Utility Tree, but in matrix form. **Often clearer when
you have many options**, because the matrix makes utility differences across options
visually immediate.

### Why the matrix often beats the tree

- More compact when there are 4+ options
- Easier arithmetic — rows are options, columns are outcomes
- Easier to compare options side-by-side
- Can be extended to multiple perspectives (one matrix per perspective, then merged)

### The eight steps (same content as tree)

1. Identify options and outcomes.
2. Determine perspective.
3. Construct the utility matrix.
4. Assign utility values.
5. Assign probabilities (sum to 1.0 across outcomes for each option).
6. Calculate expected values per cell, sum per option.
7. Rank options by total EV.
8. Sanity check.

### Markdown rendering

```markdown
### Decision: <name>
### Perspective: <whose>

|              | Outcome A   | Outcome B   | Outcome C   | Total EV |
|--------------|-------------|-------------|-------------|----------|
| **Option 1** | U=70, P=0.5, EV=35 | U=40, P=0.3, EV=12 | U=10, P=0.2, EV=2 | 49 |
| **Option 2** | U=60, P=0.4, EV=24 | U=70, P=0.4, EV=28 | U=30, P=0.2, EV=6 | 58 |
| **Option 3** | U=80, P=0.3, EV=24 | U=50, P=0.5, EV=25 | U=20, P=0.2, EV=4 | 53 |

**Ranking:** Option 2 (58) > Option 3 (53) > Option 1 (49)

**Sanity check:** <does this match intuition?>
```

### Multi-perspective utility analysis

When the decision affects multiple stakeholders with different utility functions, build
one matrix per perspective, then merge.

The 13-step procedure (Morgan Jones, advanced):

1. Identify options and outcomes.
2. Identify and *weight* the perspectives. (Weights sum to 1.0.)
3. Construct identical utility matrices for each perspective.
4. Assign utility values per perspective.
5. Assign probabilities per outcome.
6. Compute expected values per matrix.
7. Sum expected values per option per perspective.
8. Create a merged matrix with total EVs from each perspective.
9. Enter values into the merged matrix.
10. Multiply total EVs by perspective weights.
11. Sum weighted values per option.
12. Rank options by total weighted EV.
13. Sanity check.

### Markdown rendering — merged matrix

```markdown
### Merged matrix (multi-perspective)

|              | CEO (w=0.4)  | Engineer (w=0.3) | Customer (w=0.3) | Weighted Total |
|--------------|--------------|-------------------|------------------|----------------|
| **Option 1** | 70 → 28      | 40 → 12           | 60 → 18          | 58             |
| **Option 2** | 60 → 24      | 80 → 24           | 50 → 15          | 63             |
| **Option 3** | 80 → 32      | 50 → 15           | 70 → 21          | 68             |

**Ranking:** Option 3 > Option 2 > Option 1
```

### When to use multi-perspective analysis

- Decisions affecting multiple stakeholders with conflicting interests
- Architecture / strategic decisions where short-term and long-term perspectives differ
- Anything where a single perspective hides relevant tradeoffs

---

## Choosing between these tools

| Situation | Tool |
|---|---|
| Just chained probabilities, no value question | Probability Tree |
| Probability + value, few options | Utility Tree |
| Probability + value, many options | Utility Matrix |
| Multiple stakeholder perspectives | Multi-perspective Utility Matrix |
| Outcomes are certain or near-certain | Use Weighted Ranking instead (see `comparison.md`) |
| Decision is light-stakes | Use Pros-Cons-and-Fixes instead (see `comparison.md`) |

### Pitfall across all of these tools

False precision. A utility matrix with neat numbers (30 × 0.4 = 12) feels rigorous, but
the underlying utility scores and probabilities are often subjective estimates with
substantial error bars. Tag confidence bands on the inputs, not just the conclusion.

The matrix is a tool for thinking, not a calculator producing truth. When the result
points to an option the user didn't expect, treat it as an invitation to inspect the
inputs, not as a mandate.
