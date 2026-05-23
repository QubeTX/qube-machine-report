# Working Canvas

The working canvas is the lossless ledger of a critical thinking session. It exists as a
markdown file the user can watch update during the session and return to afterward.

## The principle

**Externalization beats compression.** When information accumulates, the right response is
not to summarize (lossy) but to move structure into an external representation (lossless).
The chat is the conversation. The canvas is the territory.

The canvas is **append-only by default**. Items don't get overwritten when their status
changes — they get tagged with new status. Earlier framings stay visible because they often
become relevant again.

## Where the canvas lives

At session start, propose a path. Default options:

1. **User-specified path** — preferred. Ask: *"Where should I keep the working canvas?
   You can open it in another window and watch it update."*
2. **Convention default** — `~/critical-thinking-sessions/<YYYY-MM-DD>-<topic>.md` if the
   user wants the agent to pick.
3. **Mission Control task notes** — when the session is tied to a specific tracked task,
   maintain the canvas in the task's `result_notes` field via the Task List Manager MCP.
4. **Project-local** — `<project-root>/docs/thinking/<date>-<topic>.md` when the session
   is project work.

The agent updates the canvas every turn. If the user has it open in another editor, they'll
see the updates as they happen.

## Canvas template

```markdown
# Critical Thinking Session — <topic>

**Date:** YYYY-MM-DD
**Framework:** <Contemplating | Decision-Making | Design | Problem-Solving>
**Mode:** <Facilitate | Provoke | Recommend>
**Stacked skills:** <none | cto-advisor | agile | ...>

---

## Pre-Flight: Inputs Inspected

### Inputs brought to the session
- <input 1>: <type — external artifact, internal artifact, situation description>
- <input 2>: ...

### Source pass findings
- <what was noticed about credibility, stake, evidence vs. rhetoric, missing
  counter-evidence>

### What's already decided (not revisiting)
- <commitment 1>
- <commitment 2>

---

## Working Sections

### Facts
Append-only. Each fact tagged with confidence band.

| Fact | Confidence | Source / surfaced at |
|---|---|---|
| <fact> | High / Medium / Low | <step or input> |

### Assumptions
Append-only. Each assumption tagged with status.

| Assumption | Status | Surfaced at | Notes |
|---|---|---|---|
| <assumption> | open / tested / dismissed | <step> | <why dismissed, evidence> |

### Constraints
- <constraint>: <why it matters, who set it>

### Open questions
- <question>: <when it needs answering>

### Tensions
Two surfaced items pulling against each other and not yet reconciled.
- <item A> vs. <item B>: <why this matters>

### Deferred items
Things flagged but parked. Not lost.
- <item>: <why parked, when to revisit>

### Connections
Analogies, links to prior projects, chunks of related knowledge.
- <connection>

---

## Framework Steps

### Step 1: <step name>
**Sub-questions asked:** <list>
**Responses:** <user's responses, in their own words where possible>
**Insights:** <what surfaced>
**Mode:** Convergent / Divergent

### Step 2: <step name>
...

(continued for each step worked)

---

## Visual Models In Play

When a visual model gets used in this session, embed it here as markdown.

### <Model name> — for <which step>

<table | matrix | tree as markdown>

---

## Steel-Manned Dissent

Strongest opposing view considered, and how it was handled.

- **The case against:** <strongest argument>
- **What would have to be true for it to be correct:** <conditions>
- **How it was handled:** <accepted | modified original | rejected>
- **Confidence in the rejection (if rejected):** <band>

---

## Closing

### Sanity check
- Does the result make intuitive sense? <yes / no / uncertain — explain>
- What would I expect to be true if this conclusion is right? Is that actually true?
  <answer>

### Decision / Conclusion
<what was decided or where things landed>

### Confidence band on the conclusion
<High | Medium | Low | Speculation> — <reasoning>

### Next steps
- <action>: <owner>, <by when>

### Open questions
<things unresolved that need future attention>

### Spaced revisit
- **Revisit on:** <YYYY-MM-DD>
- **Why:** <what we expect to know by then>
- **Trigger:** <what event or signal should prompt earlier revisit>
```

## What must never be lost when updating the canvas

When updating the canvas mid-session, **preserve these things in their original form**:

- **Emotional charge** — *"I'm dreading X"*. Never paraphrase to *"concerns about X."*
- **Uncertainty markers** — *"I'm 60% confident."* Don't drop the percentage.
- **The user's exact wording** where it carried specific meaning. If the user calls it
  *"the conduit problem,"* keep that phrase, don't translate to *"the routing issue."*
- **Tacit reasoning** — *"I just have a bad feeling about this."* Capture it as written.
  Don't translate to *"the user has identified risk factors."*
- **Connective tissue** — *why* a constraint matters, not just *that* it does. Keep the
  reasoning, not just the conclusion.

## Canvas update rules

1. **Append, don't overwrite.** If a fact later turns out to be wrong, mark it as
   `dismissed` with a note — don't delete.
2. **Status tags are open vocabulary.** Common ones: `open`, `tested`, `confirmed`,
   `dismissed`, `deferred`, `revisit`. Use whatever fits.
3. **Confidence bands are required** on every fact, finding, and conclusion.
   `High / Medium / Low / Speculation`. Don't ship a claim without a band.
4. **Mode tags on each step** — note whether a step's work was Convergent or Divergent.
   Helps later when reviewing whether divergence was sufficient.
5. **Surfaced-at field** — when a fact, assumption, or insight is captured, note which
   step or input surfaced it. Provenance matters for revisits.
6. **Visual models embed inline.** When a 2×2 or matrix gets built, paste it as markdown
   into the "Visual Models In Play" section, not just into chat.

## How the canvas relates to chat

- **Chat is the conversation.** Brief, in-the-moment, responsive.
- **Canvas is the territory.** Comprehensive, append-only, navigable later.

The agent should never replace the canvas with a chat summary. If the user asks "where
are we?" — point to the canvas, optionally produce a *checkpoint card* (status snapshot)
in chat that references canvas sections, but don't rewrite the canvas content into chat.

## Resolution toggle

The user can ask for any topic at three resolutions:

- **Headline** — one sentence summary of where things stand on that topic
- **Structured** — checkpoint-card density: a paragraph with key points
- **Full** — the canvas section, uncompressed

Default at transitions: structured. The full version is always one click away (just open
the canvas file).

## Closing the canvas

At session end, the canvas is finalized by:

1. Adding the closing sections (sanity check, decision, confidence, next steps, open
   questions, spaced revisit)
2. Tagging final status on all open assumptions and questions
3. (Optional) Generating a separate executive summary document if formal deliverable
   is needed — but the canvas itself is the durable artifact.

The canvas lives. The user revisits it. Future sessions on related topics start by reading
the relevant prior canvases — that's how knowledge compounds across sessions.
