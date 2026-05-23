# Devil's Advocacy

Formal adversarial analysis. Use for high-stakes decisions, contested conclusions, or
when the user is leaning toward a position that hasn't been seriously challenged.

This is the structured version of the steel-manning routine in SKILL.md. Use steel-manning
for routine challenge; use Devil's Advocacy when the stakes warrant rigor.

## The principle (Morgan Jones)

The technique originates from the Roman Catholic Church's process for evaluating sainthood
nominees: an advocate argues *against* the canonization to surface weaknesses in the case
*for*. The structured opposition activates the same analytical tactics as the main
advocate, but in opposition — which reveals biases and blind spots that confirmation
cannot.

The point is **not to talk the user out of their position.** The point is to surface what
would have to be true for their position to be wrong, so they can:

1. Test those conditions explicitly
2. Monitor for those signals after committing
3. Strengthen the case for their position by addressing the strongest objections

## When to use Devil's Advocacy

- **High-stakes decisions** — the cost of being wrong is large or hard to reverse
- **The user is heavily committed** to a position before evaluation is complete
- **The decision rests on assumptions** that haven't been independently tested
- **Group dynamics suppressed dissent** in earlier discussions
- **Recommend mode** has been used and the user wants the case stress-tested before
  committing

When *not* to use it:

- Contemplating sessions where emotional weight is the issue
- Time-pressured calls with low reversibility cost
- When steel-manning has already been done thoroughly

## The formal procedure

### Step 1 — State the position to be challenged

Write the user's current position on the canvas, in the user's own words, exactly as
they would defend it. Get their confirmation: *"Is this the position you'd defend?"*

If they want to revise the position before challenge, that's a useful signal — the
position wasn't fully formed.

### Step 2 — Generate the strongest case against

This is the core of the technique. The advocate position should be the strongest
version of the opposing case, not a strawman. Work through:

- **What evidence contradicts the position?**
- **What assumptions would the position rely on that could be wrong?**
- **Who would lose if this position is correct? What would they argue?**
- **What's the most charitable version of the opposing view?**
- **What new evidence could exist that would falsify the position?**

If the agent can't generate a strong opposing case, that's itself a finding — either the
position is genuinely solid, or the agent has been captured by the user's framing. Push
harder before concluding the former.

### Step 3 — Identify the conditions for the opposing view to be correct

Make the opposition falsifiable. List the specific things that would have to be true for
the opposing view to win:

- *"For the opposing view to be correct, X would have to be true."*
- *"If we observed Y, it would support the opposition."*
- *"If Z happened in the next 3 months, that would mean the opposition was right."*

These conditions become **monitoring signals** after commitment — they tell the user
what to watch for.

### Step 4 — Test the conditions

For each condition surfaced in Step 3:

- Is this condition demonstrably false? (If yes, opposition fails on that point.)
- Is this condition demonstrably true? (If yes, opposition wins on that point.)
- Is this condition unknown? (Mark as a monitoring signal.)

### Step 5 — Decide the outcome

Based on Step 4, the original position will:

- **Survive intact** — the opposition's strongest case doesn't undermine it. Confidence
  in the position increases. Note the surviving conditions on the canvas.
- **Survive modified** — parts of the opposition's case are correct; the position needs
  adjustment. Specify the modification.
- **Get rejected** — the opposition's case is stronger than the original. Restart from
  the relevant framework step.

### Step 6 — Capture the monitoring signals

For positions that survive (modified or intact), the conditions from Step 3 that remain
*unknown* become monitoring signals. Add to the canvas:

- **Signals to watch for:**
  - <condition>: <how we'd notice it, when we'd revisit>

This is the most underrated output of Devil's Advocacy. Even when the original position
wins, the user now has a concrete list of things that would tell them they were wrong.

## Example structure on the canvas

```markdown
## Devil's Advocacy — <position being challenged>

**Position challenged:** <user's exact wording>

### Strongest opposing case
<charitable version of the case against>

### Conditions for opposition to be correct
1. <condition A>
2. <condition B>
3. <condition C>

### Test of conditions
| Condition | Status | Evidence |
|---|---|---|
| A | False | <evidence that disproves> |
| B | Unknown | <what we don't know> |
| C | Partially true | <nuance> |

### Outcome
<Survive intact | Survive modified | Rejected> — <reasoning>

### Monitoring signals (post-commitment)
- <unknown condition>: revisit when <event> or by <date>
```

## When the user resists Devil's Advocacy

Some users push back on adversarial analysis as confrontational or unproductive. Two
moves:

1. **Reframe the purpose.** It's not about talking them out of their position — it's
   about strengthening it by surfacing what could break it. Even positions that survive
   benefit from knowing where they're vulnerable.

2. **Run a lighter version.** If the formal procedure feels too heavy, run the
   steel-manning routine from SKILL.md instead. It's faster and less confrontational.

## When the agent resists Devil's Advocacy

If the agent finds itself hesitant to push back on the user — *"they seem confident,
maybe I shouldn't"* — that's the sycophancy failure mode. The whole point of Devil's
Advocacy is to overcome the social gradient toward agreement. Run it anyway.

## Devil's Advocacy vs. steel-manning

| Aspect | Steel-manning | Devil's Advocacy |
|---|---|---|
| Formality | Light, conversational | Formal procedure |
| Use case | Routine challenge | High-stakes decisions |
| Output | Adjusted position or confirmed position | Position + monitoring signals |
| Time cost | Low | Medium |
| Best for | Decision-Making Step 5, Design Step 7 | Decision-Making Step 6 (commit), Problem-Solving Step 4 |
