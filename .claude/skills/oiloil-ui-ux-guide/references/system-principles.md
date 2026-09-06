# System-Level Guiding Principles

Use these as first-order constraints before choosing specific components or page patterns.

## Concept constancy
- Definition: The same business concept keeps the same name, meaning, and interaction semantics across the system.
- Review question: If a user learns this concept in one place, can they transfer that understanding everywhere else?

## Primary task focus
- Definition: Each screen has one dominant objective with the highest visual and interaction priority.
- Review question: Can users identify the most important action within 3 seconds?

## UI copy source discipline (for product development)
- Definition: visible UI copy should come from business content, not from implementation constraints or generation instructions.
- Preferred copy sources:
  - User task: what the user is trying to do.
  - System state: what is happening now (loading, empty, error, success, permission).
  - Result + next step: what changed and what users can do next.
  - Risk/trust context: only when it prevents mistakes or improves confidence.
- Internal-only sources (do not render directly in product UI by default):
  - Visual/style constraints (e.g., "minimal", "black-and-white", "modern").
  - Technical constraints and implementation notes.
  - Prompt instructions, review rubrics, and generation meta text.
- User-facing copy framing heuristic (general, not title-specific):
  - Applies to any prominent UI copy: titles, section headers, callouts, badges, CTA labels, and empty states.
  - Prefer user-outcome framing: describe the user's goal and the result they get.
  - Avoid self-referential/process framing for end-user product UI (e.g., "to showcase", "this page demonstrates", "showing the skill's value").
  - Exception: if the surface is explicitly a demo/tutorial/spec page for builders, self-referential/process copy can be acceptable when it improves understanding.

## State perceptibility (high-level, cross-system)
- Problem: users make errors when an important internal state is not perceivable (mode, scope, selection, unsaved changes, environment, permission).
- Principle: make state visible using the lowest-noise signal that reliably changes behavior.
- Preferred signals (in order):
  - Structural change: the layout/components clearly switch (read -> edit; list -> selection; view -> compare).
  - Control state: the control that changes behavior shows its state (tabs, toggles, segmented controls).
  - Inline signifiers: local cues near the affected area (selection count, scope chip, disabled reason).
  - Post-action feedback: clear results + next step (reduces evaluation gulf).
  - Only if needed: persistent banners/labels for high-risk, sticky modes.
- Avoid: redundant "status labels" that restate what the structure already makes obvious (they add noise but not clarity).
- Practical workflow:
  - First build a content model (task/state/result/risk).
  - Then apply visual constraints through layout, hierarchy, and component styling.
  - Run a final copy pass: if a sentence does not help task completion, state understanding, or trust, move it to internal notes.
- Review question: is each visible sentence useful for end users, or only useful for builders/reviewers?

## Help text layering (avoid "hint sprawl")
- Problem this prevents: dumping all tips onto the UI feels "safe", but it destroys hierarchy and increases scanning cost.
- Placement heuristic:
  - L0 (Always visible): only information needed to complete the task correctly.
  - L1 (Nearby): short guidance for high-risk / high-ambiguity inputs.
  - L2 (On demand): examples, advanced details, "learn more".
  - L3 (After action): result, error, recovery, and next step.
- Copy budget heuristic:
  - Prefer one clear helper line over multiple repetitive hints.
  - If a page needs many persistent hints, improve IA or defaults first.

## Feedback loop closure
- Definition: Every user action must complete a full loop: received, in progress, result, and clear next step.
- Review question: At any moment, can users tell what the system is doing and what they should do next?

## Prevention first + recoverability
- Definition: Reduce error probability before submission, and provide recovery paths for high-risk outcomes.
- Review question: Is the path designed to be easy to do right and safe to recover when wrong?

## Progressive complexity
- Definition: Show minimum-required controls by default; reveal advanced capability only when context requires it.
- Review question: Can novices complete the core task quickly without limiting expert throughput?

## Action perceptibility (affordance + signifiers)
- Definition: Interactive targets and likely outcomes are perceivable from structure and visual cues, without guesswork.
- Review question: Without reading help text, can users predict what is actionable and what will happen?

## Cognitive load budget
- Definition: Limit new rules, terms, and interaction modes per screen; prioritize reuse over novelty.
- Review question: As information grows, does comprehension cost stay stable?

## Evolution with semantic continuity
- Definition: Introduce new components/patterns only when existing ones cannot solve the problem, and keep semantic compatibility.
- Review question: Is this necessary innovation or avoidable interaction drift?
