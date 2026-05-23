# Visual Models — Comparison

Tools for comparing options against criteria, or weighing factors against each other.
Reach for these when you have **3+ options or factors and need to evaluate them
systematically**.

This file covers:

- **The 2×2 Matrix** — when two dimensions dominate
- **The Matrix (general N×M)** — for systematic categorization
- **Pros-Cons-and-Fixes** — Morgan Jones's improved pros/cons technique
- **Weighted Ranking** — when criteria have unequal importance
- **Force-Field Analysis** — drivers vs. resistors of a change

---

## The 2×2 Matrix

Use when **two dimensions dominate** the analysis and items can be sorted into the four
quadrants those dimensions create.

### When to reach for it

- Effort vs. Impact (which work to do first)
- Urgency vs. Importance (Eisenhower)
- Cost vs. Benefit (lightweight)
- Probability vs. Severity (risk)
- Strategic vs. Operational
- Reversible vs. Irreversible

### Markdown rendering

```markdown
|                | Low <X>          | High <X>          |
|----------------|------------------|-------------------|
| **High <Y>**   | <items in Q1>    | <items in Q2>     |
| **Low <Y>**    | <items in Q3>    | <items in Q4>     |
```

### Example: Effort vs. Impact

```markdown
|              | Low Effort                    | High Effort                |
|--------------|-------------------------------|----------------------------|
| **High Impact** | Quick wins (do first)      | Major projects (plan)      |
| **Low Impact**  | Time fillers (avoid)       | Money pits (kill)          |
```

### Pitfalls

- Forcing items into a 2×2 when three or more dimensions actually matter
- Choosing the two dimensions before checking whether they're truly the dominant ones
- Fuzzy boundaries — when items sit on the line, decide explicitly which side and why

---

## The Matrix (general N×M)

A general analytical matrix categorizes items along two axes where the cells contain
specific findings, not just classification.

### When to reach for it

- Comparing 3+ options across multiple criteria where rankings aren't yet weighted
- Analyzing patterns across two variables (e.g., bakery case from Morgan Jones: time of
  day × type of flour, with the cells being baking outcomes)
- Mapping evidence to hypotheses (the Hypothesis Testing matrix — see
  `causality.md`)
- Mapping options to outcomes with utility scores (Utility Matrix — see `probability.md`)

### Markdown rendering

```markdown
|              | Criterion A     | Criterion B     | Criterion C     |
|--------------|-----------------|-----------------|-----------------|
| **Option 1** | <finding>       | <finding>       | <finding>       |
| **Option 2** | <finding>       | <finding>       | <finding>       |
| **Option 3** | <finding>       | <finding>       | <finding>       |
```

### Why a matrix beats prose

The matrix forces every cell to be filled. Prose lets the user skip combinations they
haven't thought about. Empty cells in the matrix are themselves a finding — *"we don't
know how Option 2 performs on Criterion C"* — that prose hides.

---

## Pros-Cons-and-Fixes

Morgan Jones's improved pros/cons. Naive pros/cons lists default to negativity
(humans list more cons than pros for the same item). Pros-Cons-and-Fixes fights this by:

1. Listing pros first
2. Listing cons second
3. **Asking which cons can be fixed** — converting some cons back into pros
4. Comparing options on pros + unfixable cons only

### The six steps

1. **List all the Pros** — identify and enumerate every benefit
2. **List all the Cons** — state drawbacks without hesitation
3. **Review and Consolidate the Cons** — merge similar cons, eliminate duplicates
4. **Neutralize as Many Cons as Possible** — for each con: can we fix this? How?
   Convert fixable cons into mitigations.
5. **Compare the Pros and Unalterable Cons for All Options** — the unfixable cons are
   the real ones
6. **Pick One Option** — based on the comparative analysis

### Markdown rendering

```markdown
### Option: <name>

**Pros:**
- <pro 1>
- <pro 2>
- <pro 3>

**Cons:**
- <con 1>
- <con 2>
- <con 3>

**Fixes:**
- <con 1>: <how we'd fix or mitigate it>
- <con 2>: <how we'd fix or mitigate it>

**Unfixable cons:**
- <con 3>: <why it can't be fixed within constraints>

---

### Option: <next name>
...

---

### Comparison

| Option | Strongest pro | Unfixable con |
|--------|---------------|---------------|
| <Opt 1> | <pro> | <con> |
| <Opt 2> | <pro> | <con> |
```

### When to use

- Decision-Making Step 5 (Evaluate Options) — primary use
- Design Step 7 (Refine) — evaluating prototype iterations
- Problem-Solving Step 4 (Evaluate and Select) — comparing candidate solutions

### Pitfall

Treating all cons as fixable. Some cons are structural — they reflect real tradeoffs
that can't be designed away. Honesty about unfixable cons is what makes this technique
work; if everything ends up "fixable," the analysis was sloppy.

---

## Weighted Ranking

Use when criteria have unequal importance, and you need to make those weights explicit
to avoid implicit-weighting bias.

This is Morgan Jones's 9-step procedure.

### The nine steps

1. **List major ranking criteria** — what dimensions matter for this decision?
2. **Pair-rank the criteria** — for each pair, which is more important? Tally.
3. **Assign weights to the top criteria** — usually 2–5 criteria with weights summing to
   a clean number (e.g., 100 or 1.0)
4. **Construct a Weighted Ranking Matrix** — rows are options, columns are criteria
5. **Pair-rank items by each criterion** — for each criterion column, pair-rank the
   options. Record the votes.
6. **Multiply votes by respective criterion weights** — produces weighted scores per cell
7. **Sum weighted values for total scores** — total per option
8. **Determine final rankings** — based on total scores
9. **Perform a sanity check** — does the ranking match intuition? If not, investigate.
   The ranking might be right, but so might intuition — usually one of them is missing
   something.

### Markdown rendering

#### Criterion pair-ranking (Step 2)

```markdown
| Pair | Winner |
|------|--------|
| Speed vs. Cost | Cost (×) |
| Speed vs. Quality | Quality (×) |
| Cost vs. Quality | Quality (×) |
| ... | ... |
```

Tally the wins, weight by share.

#### Weighted Ranking Matrix (Step 4–7)

```markdown
| Option | Speed (w=20) | Cost (w=30) | Quality (w=50) | Total |
|--------|--------------|-------------|----------------|-------|
| Opt A  | 3 → 60       | 2 → 60      | 1 → 50         | 170   |
| Opt B  | 1 → 20       | 3 → 90      | 2 → 100        | 210   |
| Opt C  | 2 → 40       | 1 → 30      | 3 → 150        | 220   |
```

(Pair-ranked votes per option per criterion, multiplied by criterion weight, summed.)

### When to use

- Decision-Making Step 5 (Evaluate Options) — primary use
- Problem-Solving Step 4 (Evaluate and Select)
- Design Step 4.2 (Compare and Narrow Down Ideas)

### Sanity check (Step 9)

If the weighted ranking points to an option the user didn't expect, **don't immediately
adjust the weights to get the answer they wanted.** Instead:

- Surface the discrepancy: "The matrix says X, but you seem to be leaning toward Y."
- Ask: "Are the weights wrong, or is your intuition responding to something the matrix
  isn't capturing?"

If a criterion is missing from the matrix, add it and rerun. If the weights are wrong,
adjust them and explain *why*. Don't rationalize.

---

## Force-Field Analysis

Use when planning a change. Maps the **drivers** (forces pushing toward the change)
against the **resistors** (forces pushing against it). Often surfaces resistors that
weren't obvious until you mapped them.

### Markdown rendering

```markdown
### Goal: <the change being considered>

| Driving forces (push toward change) | Strength | Resistor (push against change) | Strength |
|-------------------------------------|----------|--------------------------------|----------|
| <force>                             | High     | <force>                        | High     |
| <force>                             | Medium   | <force>                        | High     |
| <force>                             | Low      | <force>                        | Medium   |

**Net assessment:** <driver-dominant | resistor-dominant | balanced>

**Strategy implications:**
- To strengthen drivers: <how>
- To weaken resistors: <how>
```

### When to use

- Design Step 2.2 (Determine Requirements and Constraints) — surfaces resistance early
- Decision-Making Step 5 — when the decision is to make a change
- Problem-Solving when the problem is *"why is this change failing?"* — usually
  resistors are stronger than acknowledged

### Insight

The most common finding from force-field analysis: **drivers were known, resistors were
underestimated.** Most failed changes fail not because the case for change was weak, but
because the resistance was stronger than mapped. Spend more time on the resistor column
than the driver column.

---

## Choosing between these tools

| Situation | Tool |
|---|---|
| Two clear dimensions dominate | 2×2 Matrix |
| Multiple options, multiple criteria, no weighting yet | General Matrix |
| Want to fight default negativity bias in evaluation | Pros-Cons-and-Fixes |
| Criteria clearly have unequal importance | Weighted Ranking |
| Planning a change; need to map resistance | Force-Field Analysis |
| Mapping evidence to competing explanations | Hypothesis Testing (see `causality.md`) |
| Outcomes are uncertain; need probability + value | Utility Matrix (see `probability.md`) |
