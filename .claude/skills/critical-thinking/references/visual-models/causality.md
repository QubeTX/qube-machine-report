# Visual Models — Causality

Tools for tracing causes, mapping cause-and-effect relationships, and testing competing
explanations. Reach for these when you need to understand **why** something is happening,
not just **what**.

This file covers:

- **Causal Flow Diagram** — mapping factors and their cause-effect links, including
  feedback loops
- **Fishbone (Ishikawa) Diagram** — categorizing potential causes by domain
- **Hypothesis Testing Matrix** — Morgan Jones's structured method for ranking competing
  explanations

---

## Causal Flow Diagram

Use when **multiple factors interact** to produce a problem, and you need to see the
shape of the system rather than just list its parts.

### The five steps (Morgan Jones)

1. **Identify Major Factors** — the dynamic factors driving the situation. (For a
   business problem: sales, profits, R&D investment, marketing, competition, etc.)
2. **Identify Cause-and-Effect Relationships** — for each pair of factors, does one
   cause the other? Build a Cause-and-Effect Table.
3. **Characterize Relationships** — classify each as Direct (D — when X goes up, Y goes
   up) or Inverse (I — when X goes up, Y goes down).
4. **Diagram Relationships** — visually represent. Arrows from cause to effect, labeled
   D or I.
5. **Analyze System Behavior** — review the system as a whole. Look for influential
   factors and feedback loops.

### Markdown rendering

```markdown
**Major factors:**
- A: <factor>
- B: <factor>
- C: <factor>
- D: <factor>

**Cause-and-Effect Table:**

| Cause | Effect | D/I | Notes |
|-------|--------|-----|-------|
| A | B | D | <reason> |
| B | C | I | <reason> |
| C | A | D | <reason> (feedback loop!) |

**Diagram:**

A ──D──→ B ──I──→ C
↑                 │
└──────D──────────┘
   (feedback)
```

### Feedback loops

The most important output of causal flow diagramming is identifying feedback loops:

- **Self-stabilizing (SS)** — the loop seeks equilibrium. When one factor moves, others
  push back. Often invisible until disrupted.
- **Unstable (U)** — the loop amplifies. Small changes spiral. These are usually the
  ones causing the problem.

When you find an unstable loop, you've usually found the leverage point. Disrupting one
link in an unstable loop is more effective than treating individual factors.

### When to use

- Problem-Solving Step 2 (Analyze) — primary use
- Design Step 6 (Test) — when prototype behavior is mysterious
- Pre-flight when an external artifact describes a system

### Pitfall

Diagrams that include every factor become unreadable and useless. Keep diagrams as simple
as necessary — typically 4–8 major factors. If you need more, sort first into
sub-systems and diagram each separately.

---

## Fishbone (Ishikawa) Diagram

Use when **brainstorming categories of causes** for a problem. Forces breadth — you can't
miss a category if you have a slot for each.

### Standard categories

For most problems, the "6Ms" cover the space:

- **People** (or Manpower) — human factors, skill, training
- **Process** (or Method) — how things are done
- **Materials** — inputs, ingredients, supplies
- **Machinery** (or Equipment) — tools, technology
- **Measurement** — how you know what's happening
- **Environment** (or Mother Nature) — conditions, context, external factors

For software / digital systems, common adaptations:

- **People** — team, users, stakeholders
- **Process** — methodology, workflows
- **Code / System** — the software itself
- **Data** — inputs, schemas, sources
- **Infrastructure** — servers, network, dependencies
- **External** — third-party services, API providers, market

### Markdown rendering

```markdown
## Problem: <effect to be analyzed>

### People
- <potential cause>
- <potential cause>

### Process
- <potential cause>

### Materials
- <potential cause>

### Machinery
- <potential cause>
- <potential cause>

### Measurement
- <potential cause>

### Environment
- <potential cause>
```

For visual fishbone in markdown:

```
                                              [Problem]
                                                 ↑
        People                  Process                Materials
        ──────                  ───────                ─────────
        ├─ <cause>             ├─ <cause>             ├─ <cause>
        └─ <cause>             └─ <cause>             └─ <cause>
                                  │
        Machinery               Measurement              Environment
        ─────────               ───────────              ───────────
        ├─ <cause>             ├─ <cause>               ├─ <cause>
        └─ <cause>             └─ <cause>               └─ <cause>
```

### When to use

- Problem-Solving Step 2 (Analyze) — when the problem could have causes from multiple
  domains and you don't want to miss one
- Pairs well with Five Whys — fishbone surfaces the breadth, Five Whys drills the depth
- Don't use when the cause is already known and you just need to evaluate solutions

### Pitfall

Listing everything that *could* cause the problem isn't analysis — it's brainstorming.
The fishbone is the divergent step. The convergent step is then ranking which causes
are most likely (often via Hypothesis Testing — see below).

---

## Hypothesis Testing Matrix

The most rigorous tool in this category. Use when there are **multiple competing
explanations** for a phenomenon, and you need to rank them by which has the *least
inconsistent* evidence.

This is Karl Popper's falsification logic operationalized: you can't prove a hypothesis
true, but you can disprove it. The hypothesis with the least disconfirming evidence wins
— for now.

### Why this beats satisficing

Morgan Jones names "satisficing" as a core failure mode: settling for the first
satisfactory explanation rather than evaluating alternatives. Hypothesis Testing fights
this by forcing you to rank multiple candidates against the same evidence.

### The eight steps (Morgan Jones)

1. **Generate hypotheses** — at least 3, ideally 5+. Include unconventional ones.
2. **Construct a hypothesis-testing matrix** — hypotheses across the top, evidence down
   the side.
3. **List significant evidence** — both present and *absent* evidence. (Evidence that
   *should* be there if a hypothesis is true, but isn't, is a strong falsifier.)
4. **Test the consistency of evidence with each hypothesis** — for each evidence-
   hypothesis cell: Consistent (C), Inconsistent (I), Neutral (N), or Ambiguous (A)?
5. **Refine the matrix** — add or remove hypotheses and evidence as you learn.
6. **Evaluate and refine hypotheses based on inconsistent evidence** — count Is per
   hypothesis. Inconsistent evidence is what disproves; consistent evidence merely fails
   to disprove.
7. **Rank hypotheses by the weakness of inconsistent evidence** — the hypothesis with
   the *fewest* and *weakest* inconsistent items wins.
8. **Perform a sanity check** — does the ranking match what you'd expect? If not,
   investigate before accepting.

### Markdown rendering

```markdown
### Question: <what we're trying to explain>

|                          | H1: <hypothesis> | H2: <hypothesis> | H3: <hypothesis> |
|--------------------------|------------------|------------------|------------------|
| E1: <evidence>           | C                | I                | C                |
| E2: <evidence>           | C                | A                | I                |
| E3: <evidence>           | I                | C                | C                |
| E4: <absence — <what>    | C                | I                | A                |
| E5: <evidence>           | A                | C                | C                |
| **Inconsistent count**   | 1                | 2                | 1                |

**Strength of inconsistencies:**
- H1's E3 is weak — could be explained by <reason>
- H3's E2 is strong — hard to explain away

**Ranking:** H1 most likely → H3 → H2

**Sanity check:** Does this match intuition? <yes / no — investigate>
```

Legend:
- **C** = Consistent (evidence fits the hypothesis)
- **I** = Inconsistent (evidence contradicts the hypothesis)
- **N** = Neutral (no relationship)
- **A** = Ambiguous (could be interpreted multiple ways)

### Why "absent evidence" matters

If H1 is true, X *should* be observable. We don't observe X. That's an inconsistency
even though no evidence is "present." This is one of the most powerful moves in the
technique — and one most informal analysis skips.

### When to use

- Problem-Solving Step 2 (Analyze) — when there are multiple plausible root causes
- Decision-Making Step 5 — when each option implies a different theory of the case
- Pre-flight Source Inspection — when an article advances a claim that should be tested
  against alternative explanations
- Design Step 6 (Test) — when a prototype isn't working and the cause is unclear

### Famous applications (from Morgan Jones)

- **Disney's America** (1990s theme park controversy) — failure to consider alternative
  hypotheses about local opposition led to a costly retreat.
- **U.S.S. Iowa explosion (1989)** — Navy initially settled on a single hypothesis
  (sailor sabotage); evidence later supported a different one (mechanical failure of
  powder bag).

In both cases, the failure was satisficing on an early hypothesis without testing
alternatives.

### Pitfall

The matrix is only as good as the hypotheses you list. If the true explanation isn't in
your H1–H3 list, no amount of evidence ranking will find it. The hypothesis-generation
step (Step 1) is divergent and deserves time.

---

## Choosing between these tools

| Situation | Tool |
|---|---|
| Multiple factors interact in a system | Causal Flow Diagram |
| Brainstorming categories of causes | Fishbone Diagram |
| Multiple competing explanations | Hypothesis Testing Matrix |
| Both: brainstorm causes + rank them | Fishbone, then Hypothesis Testing |
| Mapping a system that has feedback loops | Causal Flow Diagram with feedback notation |
