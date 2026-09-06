# Design Psychology (inspired by *The Design of Everyday Things*)

Keep this as a compact reference. Use it when explaining *why* a design is confusing and how to fix it.
This is a paraphrased summary, not a verbatim excerpt.

## Affordances (示能性 / 可供性)

- An affordance is what an object *allows* a person to do.
- In UI, you mostly manage **perceived affordances**: what people *think* they can do.

Practical rule:
- If an action is important, it must be discoverable without hover, tooltips, or prior training.

## Signifiers (指示符)

- Signifiers are the cues that indicate possible actions.

Examples in UI:
- Button shape, link styling, icons + labels, hover/focus states, cursor changes, microcopy.

Practical rule:
- Use the smallest signifier that removes ambiguity. Default to labels for non-obvious actions.

## Mapping (映射) / Natural mapping

- Mapping is the relationship between controls and their effects.
- Natural mapping means the layout/relationship mirrors the real-world mental model.

Practical rules:
- Put controls near what they control.
- Use spatial grouping to show what belongs together.
- For multi-part objects, align actions with the part they affect (per-item actions next to the item).

## Constraints (约束)

- Constraints limit possible actions, preventing errors and reducing thinking.

Types you can use in UI:
- Physical constraints (not literal in UI, but you can simulate via disabled states)
- Logical constraints (only valid combinations are allowed)
- Semantic constraints (meaning-based limits)
- Cultural constraints (conventions users expect)

Practical rules:
- Prefer constraints + defaults over warnings.
- If you must block an action, explain the requirement and provide a path to satisfy it.

## Conceptual model (概念模型)

- Users form an internal model of how the system works.
- Your UI should make the correct model obvious.

Practical rules:
- Use consistent nouns/labels for objects.
- Use consistent verbs for actions.
- Show cause-effect clearly (do X -> see Y change).

## Feedback (反馈)

- Feedback tells people what happened after an action.

Practical rules:
- Always provide immediate feedback for interaction (press/hover/loading).
- If an operation takes time, show progress or a clear waiting state.
- After success/failure, clearly state the outcome and the next step.

## Gulfs of execution & evaluation (执行鸿沟 / 评估鸿沟)

- Execution gulf: user can’t figure out how to do what they want.
- Evaluation gulf: user can’t tell what happened or what state the system is in.

Practical diagnostic:
- If users hesitate before acting: reduce execution gulf (clear CTA, clearer signifiers, simpler choices).
- If users repeat actions / rage-click: reduce evaluation gulf (loading, disabled, progress, clearer results).

## Slips vs mistakes (失误 vs 错误)

- Slip: the goal is correct, the action execution goes wrong (fat-finger, wrong click).
- Mistake: the mental model/goal is wrong (user thinks it works differently).

Practical rules:
- Slips: add undo, confirmations for destructive actions, safer hit targets, better spacing.
- Mistakes: fix labeling, mapping, and conceptual model; add just-enough explanation.

## Knowledge in the world vs in the head (外部知识 vs 头脑知识)

- Good design puts knowledge in the world: visible options, clear labels, previews, examples.

Practical rule:
- Don’t force users to remember constraints. Surface them at the point of decision.

## Modes (模式) and mode errors

- Modes mean the same action produces different results depending on state.

Practical rule:
- Avoid modes; if unavoidable, make mode state extremely visible and easy to exit.
