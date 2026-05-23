# Visual Models — Structure

Tools for organizing information so its shape becomes visible. Reach for these when items
are accumulating without organization, or when sequence and relationships matter more
than evaluation.

This file covers:

- **Sorting** — categorizing items into groups
- **Chronology / Timeline** — when sequence matters
- **Scenario Tree** — branching consequences of decisions and events
- **Concept Map** — non-hierarchical relationships between ideas

---

## Sorting

Often underused. Many problems clarify themselves when their components are sorted into
the right groups.

### When to reach for it

- A long, undifferentiated list of items that need analysis
- Working with data that has natural categories you haven't named yet
- Before any evaluation — sorting is a divergent move that often reveals patterns

### How to sort

1. **Identify the dimension to sort on.** Sometimes obvious; sometimes not. Common
   dimensions: type, source, time, severity, audience, owner, status.
2. **Don't assume a single sort is enough.** Sometimes you need to sort by Dimension A,
   then within each group, sort by Dimension B.
3. **Watch for items that don't fit.** Items that resist your categories are signals —
   either the categories are wrong, or those items are exceptions worth understanding.

### Example

A long list of bugs might sort by:

- **Severity:** critical / high / medium / low
- **Component:** auth / data / UI / infrastructure
- **Reporter:** customer / internal / monitoring

A 2-dimensional sort (severity × component) often reveals that critical bugs cluster in
one component, which is more actionable than the flat list.

### Markdown rendering

For one dimension:

```markdown
### Critical
- Bug A
- Bug B

### High
- Bug C
- Bug D
```

For two dimensions, use a matrix (see `comparison.md`).

---

## Chronology / Timeline

Use when **sequence and timing matter** for understanding the situation. Reordering
events into chronological order often reveals patterns that the original narrative
obscured.

### When to reach for it

- Investigating an incident or failure
- Reconstructing how a problem developed
- Understanding decision-maker history before predicting their behavior (Morgan Jones's
  "Placement" technique)
- Mapping milestones across a long-running project

### The two-step procedure (Morgan Jones)

1. **List events with their dates.** Capture every potentially significant event, even
   if you're not sure it matters.
2. **Build the visual timeline.** Cross off events as you place them. Arrange
   horizontally or vertically.

### Markdown rendering

```markdown
| Date | Event | Source / Notes |
|------|-------|----------------|
| 2026-03-15 | <event> | <context> |
| 2026-04-02 | <event> | <context> |
| 2026-04-10 | <event> | <context> |
```

Or visual:

```markdown
2026-03-15 ─────●  <event>
                │
2026-04-02 ─────●  <event>
                │
2026-04-10 ─────●  <event>
```

### Insight

Many "this came out of nowhere" stories aren't accurate. Reconstructing a chronology
usually reveals signals that were available earlier but not noticed. The pattern of
*when* signals appeared often matters more than *what* they were.

---

## Scenario Tree

Use when **a decision has branching consequences** that themselves lead to further
decisions. Maps the full outcome space, not just the immediate next step.

### Two key properties (Morgan Jones)

A well-constructed scenario tree has branches that are:

- **Mutually exclusive** — choosing one option means not choosing the others
- **Collectively exhaustive** — the branches cover all possible outcomes; no scenario
  falls outside

If your tree has overlapping branches or missing scenarios, the analysis is incomplete.

### The four-step procedure

1. **Identify the problem.**
2. **List major factors / issues** (decisions and events).
3. **Define alternatives for each factor / issue.**
4. **Construct the tree** ensuring each branching is mutually exclusive and collectively
   exhaustive.

### Markdown rendering

```markdown
Decision: Hire full-time engineer?

├── Yes → 
│       ├── Engineer is strong → Project ships on time
│       └── Engineer is weak → Project delayed, costs increase
│
└── No → 
        ├── Existing team can absorb → Slower but no hiring cost
        └── Existing team overloaded → Burnout, attrition risk
```

For deeper trees, use indentation or a more compact list:

```markdown
1. Hire full-time
   1.1. Strong hire → On time
   1.2. Weak hire → Delayed, +cost
2. Don't hire
   2.1. Team absorbs → Slower
   2.2. Team overloaded → Burnout risk
```

### When to use

- Decision-Making Step 5 (when consequences chain)
- Design Step 4 (concepts with branching feasibility)
- Problem-Solving Step 4 (solutions whose effects depend on conditions)
- Probability Tree (when probabilities matter — see `probability.md`)

### Pitfalls

- **Imbalanced depth.** Some branches get explored deeply, others get a single line.
  Either explore all to similar depth, or explicitly mark the unexplored branches as
  *"out of scope for this session."*
- **Missing branches.** *"Maintain status quo"* is often missing from decision trees.
  Add it explicitly.
- **Confusing decisions and events.** A decision is something you choose; an event is
  something that happens to you. Mark them differently.

---

## Concept Map

Use when ideas have **non-hierarchical relationships** — networks rather than trees.
Concept maps make cross-references visible.

### When to reach for it

- The session's canvas has many cross-references between items
- Multiple frameworks or domains intersect
- Helping the user see how their existing knowledge connects to a new idea

### Markdown rendering

ASCII concept maps work surprisingly well:

```markdown
                    Sycophancy
                   /          \
              AI agrees      User overconfidence
                  \              /
              Output-competence
                  decoupling
                      |
              ┌───────┴───────┐
              ↓               ↓
        Workplace slop   Cross-domain
                          impersonation
                              |
                         (the more dangerous
                          failure mode)
```

For complex maps, render as a list of relationships:

```markdown
**Nodes (concepts in play):**
- Sycophancy
- AI agreement bias
- User overconfidence
- Output-competence decoupling
- Workplace slop
- Cross-domain impersonation

**Edges (relationships):**
- Sycophancy → AI agreement bias (manifests as)
- AI agreement bias + User overconfidence → Output-competence decoupling (combine to produce)
- Output-competence decoupling → Workplace slop (one consequence)
- Output-competence decoupling → Cross-domain impersonation (more dangerous consequence)
```

### When to use

- Pre-flight when an external artifact is rich and interconnected — map its concepts
  before evaluating
- Mid-session when the canvas's cross-references become tangled
- Closing when teaching the user the conceptual structure of what was decided

### Concept maps vs. trees

| Use a tree when | Use a concept map when |
|---|---|
| Hierarchy is clear | Relationships go in many directions |
| One root, many leaves | Multiple roots, multiple terminals |
| Each node has one parent | Nodes have multiple connections |
| Decisions and consequences | Concepts and influences |

---

## Choosing between these tools

| Situation | Tool |
|---|---|
| Items pile up without organization | Sorting |
| Sequence or timing matters | Chronology / Timeline |
| Decisions cascade into further decisions | Scenario Tree |
| Concepts cross-reference in a network | Concept Map |
| Both: events over time + branching outcomes | Combine timeline + tree |
