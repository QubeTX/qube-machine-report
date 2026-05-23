# Problem-Solving Framework

A systematic approach to diagnosing problems and selecting solutions. Use when something is
broken, wrong, or not working — when there is a perceived gap between the current state and
a desired outcome.

> **What is a problem?** A problem is a situation where there is a perceived gap between
> the current state and a desired outcome. It is characterized by uncertainty, difficulty,
> or complexity that necessitates resolution. It involves a question or challenge requiring
> action, decision-making, or thought. Problems often demand understanding their causes,
> exploring solutions, and applying actions to bridge the gap between what exists and what
> we desire.

## How this framework uses the canvas

Problem-Solving sessions often hinge on the *reframe* — finding the right problem
statement to work on. The canvas should preserve every problem statement attempted, not
just the final one. Earlier framings often turn out to be relevant later, especially if
the chosen frame fails to resolve the issue.

---

## Step 1 — Define the Problem

Get the problem stated clearly before trying to solve it. Most botched problem-solving
sessions skip this step or rush through it.

### Sub-questions to ask

**1.1.** Clearly describe the problem by stating:
- What is happening?
- What should be happening?
- What is the gap between the two?

**1.2.** Explain why the problem is important.
- What are the potential impacts if it's left unresolved?

**1.3.** Establish the timeframe.
- When did the problem start?
- When is a solution required by?

**1.4.** Describe how the problem occurred.
- Trace its origins.
- Identify contributing factors.

**1.5.** Identify all individuals or groups affected.
- Who is impacted, and how?

**1.6. Reframe the problem** — *the most important sub-step in the framework.*

A problem well-stated is half-solved. A problem poorly stated will produce solutions to
the wrong thing.

#### The four pitfalls in problem definition (Morgan Jones):

1. **No Focus** — problem statements that lack specificity. *"Sales are bad."*
2. **Misdirected Focus** — defined too narrowly, missing the real issue. *"We need to
   reduce email response time"* when the real issue is that customers don't trust the
   product.
3. **Assumption-Driven** — based on unfounded assumptions baked into the framing.
   *"How do we get engineers to work harder?"* assumes engineers aren't already.
4. **Solution-Driven** — defining the problem in terms of a preferred solution.
   *"How do we add more headcount to the team?"* presumes the answer is more headcount.

#### Five problem restatement techniques (Morgan Jones):

Run at least three of these. The exercise is divergent — generate multiple framings,
then pick.

1. **Paraphrase** — Restate using different wording. New words sometimes reveal new
   insights.
2. **180 Degrees** — Flip it. State the opposite, or ask *"how could we make this
   worse?"* The inverse often reveals what's actually broken.
3. **Broaden the Focus** — Place the problem in a larger context. *"How do we fix this
   bug?"* becomes *"Why do bugs of this type keep recurring?"*
4. **Redirect the Focus** — Shift to an adjacent problem. *"How do we keep customers from
   churning?"* might become *"How do we identify customers we shouldn't have sold to in
   the first place?"*
5. **Why Technique** — Continually ask "why" to drill into the essence of the problem.
   This is the seed of the Five Whys at Step 2.4.

> **Canvas move:** Capture every reframe attempt on the canvas, even the ones you
> reject. Earlier framings often become relevant when the chosen frame fails to resolve.

**1.7.** Visualize the problem.
- Create a diagram or sketch outlining the relationships, processes, or causes.
- See `references/visual-models/causality.md` for fishbone diagrams and causal flow
  diagrams.

> **Clarity in problem statements:** use simple, positive, active voice. *"Customer
> response time exceeds 24 hours during peak periods"* beats *"It is the case that
> customers may not always receive timely responses to their inquiries."*

---

## Step 2 — Analyze the Problem

Break it down. Find the root cause, not the surface symptom.

### Sub-questions to ask

**2.1.** Gather relevant information.
- Consult documentation. Conduct interviews. Observe the environment.

**2.2.** Break the problem into smaller components.
- What individual tasks or sub-issues contribute to the bigger issue?

**2.3.** Create a comprehensive list of questions.
- What unknowns need clarification?

**2.4. Ask "Why?" five times.**
- Sequentially question each prior answer to uncover the root cause.
- Don't stop at the first plausible "why."
- Build a **Causal Flow Diagram** (`references/visual-models/causality.md`) to make the
  why-chain visible.

**2.5.** Identify patterns, similarities, and differences.
- Compare aspects of this problem to known situations or past experiences.

**2.6.** Compare the current problem to similar issues encountered in the past.
- What solutions were effective then? What didn't work, and why?

**2.7.** Consider extreme and edge cases.
- What does the worst-case scenario look like?
- What does the best-case scenario look like?

> **The Five Whys (2.4) is the engine of this step.** It's easy to do badly. Users will
> stop at the second or third "why" because it feels like progress. Push for all five.
> The root cause is rarely the first thing surfaced.

> **Hypothesis testing move:** When there are multiple plausible root causes, use the
> **Hypothesis Testing matrix** (`references/visual-models/causality.md`). List the
> competing hypotheses, list the evidence, and rank by which hypothesis has the *least
> inconsistent* evidence. This is the core defense against satisficing — settling for the
> first satisfactory explanation.

> **Fishbone move:** When you need to brainstorm causes across multiple categories
> (people, process, materials, equipment, environment, etc.), use a **Fishbone (Ishikawa)
> diagram** (`references/visual-models/causality.md`).

---

## Step 3 — Brainstorm Solutions

**This is the divergent step.** Generate options without judging them yet.

### Sub-questions to ask

**3.1.** Generate a list of potential solutions without evaluating them initially.
- Encourage creativity. Bad ideas help good ones surface.

**3.2.** Consider alternative perspectives.
- How would someone with different expertise approach this?

**3.3.** Work backward from the desired outcome.
- What steps are needed to achieve the goal?

**3.4.** Create test scenarios or prototypes.
- What can you build quickly to validate assumptions?

**3.5.** Conduct thought experiments.
- Visualize how each proposed solution would play out in practice.

**3.6.** Use trial-and-error where feasible.
- Test different approaches in controlled settings.

**3.7.** Take breaks to gain fresh insight.
- Some answers only arrive after stepping away.

**3.8.** Reflect on how others might approach similar problems.
- Study existing solutions or consult experts.

> **Convergence/divergence move:** Step 3 is purely divergent. Apply the **Four
> Commandments of Divergent Thinking** from SKILL.md. No evaluation yet.

---

## Step 4 — Evaluate and Select a Solution

Pick. Test. Commit.

### Sub-questions to ask

**4.1.** Use a decision matrix.
- List potential solutions.
- Rank them against criteria: feasibility, cost, impact.
- See `references/visual-models/comparison.md` for the **Weighted Ranking** procedure.

**4.2.** Test promising solutions through small-scale trials.
- What's the cheapest way to find out if this works?

**4.3.** Reassess if assumptions are incorrect.
- Revisit earlier analysis steps if a trial surfaces something unexpected.

**4.4.** Choose the most effective, scalable, and realistic solution.
- Base the choice on evaluation results, not intuition alone.

> **Devil's Advocacy move:** Before committing, run the formal Devil's Advocacy procedure
> (`references/devils-advocacy.md`) on the chosen solution. The point is not to talk the
> user out of it — it's to surface what would have to be true for the chosen solution to
> fail, so they can monitor for those signals.

---

## Closing this framework — sanity check

Before finalizing the canvas:

- Does the chosen solution actually address the *problem statement* you settled on at
  Step 1.6, or has scope drifted?
- Did you stop at a plausible "why" instead of pushing to the root cause?
- Are you treating a recurring system problem as a one-time event problem? (If the
  symptom returns in 6 weeks, you didn't actually solve it.)
- What would have to be true in 30 days for this solution to have failed? How would you
  detect it early?

Confidence band: high if the solution was tested at small scale and the trial results
align with expectations. Lower if it relies on judgment about complex causation.

---

## Facilitation notes for this framework

- **Don't skip Step 1.6** ("What problem do I actually have?"). The most common failure
  is solving the wrong problem confidently.
- **Don't accept the first "why"** in Step 2.4. Push for five.
- **Don't evaluate during Step 3.** Generate first, judge second.
- **Step 4 often loops back to Step 1.** If a trial reveals the problem was misframed,
  go back to Step 1, not forward to a different solution.
- **When the problem is a system problem, not an event problem,** Steps 2.5–2.6
  (pattern matching against past experiences) are critical. A repeated outage at the
  same time of day every week is a system problem; treating it as an event will produce
  fixes that don't last.
- **For diagnostic problems** (we know something is wrong, we don't know what causes it),
  Hypothesis Testing is usually the right tool. For **structural problems** (we know what
  the issue is, we don't know how to fix it), Causal Flow + Weighted Ranking is usually
  better.
