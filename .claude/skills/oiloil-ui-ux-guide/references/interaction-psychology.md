# Interaction Psychology (HCI Laws, Cognitive Biases, Flow)

Compact reference for review and design guidance.
Complements `design-psych.md` (Norman's conceptual model) with empirically-grounded laws and biases that directly inform design decisions.

## A) Classic HCI Laws

### Fitts's Law (费茨定律)

- Core idea: the time to reach a target is a function of target size and distance.
- Larger and closer targets are faster and easier to hit.

Practical rules:
- Primary CTA: make it the largest interactive element in its section and place it near the user's visual focus.
- Destructive actions: keep them small and spatially separated from the primary CTA to prevent slips.
- Touch targets: minimum 44×44 CSS px (web) / 48×48 dp (mobile); don't shrink hit area to the glyph.
- Edges and corners of the viewport are effectively infinite-size targets (screen edge stops the cursor) — use them for key navigation (e.g., fixed top nav, bottom tab bar).

Review question: Is the primary action button large enough and close to the user's focus? Are destructive actions physically separated from routine actions?

### Hick's Law (希克定律)

- Core idea: decision time increases logarithmically with the number of choices.
- More options → slower decisions → higher abandonment.

Practical rules:
- Limit visible choices: if a list/menu exceeds ~7 items, add grouping, search, or filtering.
- Use smart defaults to eliminate decisions entirely (the best choice is no choice).
- Progressive disclosure: show basic options first, reveal advanced options on demand.
- Avoid "paradox of choice" in onboarding: guide users through a recommended path instead of presenting all features at once.

Review question: Is the user facing too many options at once? Can grouping, search, or defaults reduce the decision burden?

### Miller's Law (米勒定律)

- Core idea: working memory holds roughly 7 ± 2 items.
- Exceeding this limit causes cognitive overload and errors.

Practical rules:
- Navigation / tab bars: keep to ≤ 7 top-level items; use grouping or "more" for the rest.
- Long forms: chunk fields into labeled groups (≤ 5–7 fields per group).
- Information display: break long lists into scannable sections with headings.
- Don't force users to remember information across screens — carry context forward.

Review question: Does a single screen require the user to hold more than 7 independent pieces of information in mind?

---

## B) Cognitive Biases in UI Design

### Anchoring Effect (锚定效应)

- Users are influenced by the first piece of information they see.
- The first number, option, or example sets a reference point for all subsequent judgments.

Practical rules:
- Pricing pages: show the recommended plan first (or in the center); it becomes the anchor.
- Form defaults: the pre-filled value becomes the user's baseline — choose it carefully.
- Progress indicators: showing "step 2 of 3" anchors the user's effort expectation.

### Default Effect (默认效应)

- Users disproportionately stick with the default option.
- Defaults are the most powerful design decision you can make.

Practical rules:
- Set defaults to the safest and most common choice.
- Never use defaults to trick users into unfavorable choices (dark pattern).
- When there is no safe default, force an explicit choice instead of pre-selecting.

### Peak-End Rule (峰终定律)

- Users judge an experience primarily by its most intense moment and its ending.
- A painful middle is forgiven if the peak and end are positive.

Practical rules:
- Invest in the completion/success screen — it's the last impression.
- Error recovery experience matters more than error prevention messaging for overall satisfaction.
- Celebrate meaningful milestones (first project created, first successful deploy).

### Loss Aversion (损失厌恶)

- The pain of losing something is ~2× stronger than the pleasure of gaining the same thing.
- Users are more motivated to avoid loss than to achieve gain.

Practical rules:
- Destructive actions: frame confirmation around what will be lost ("You will lose 12 files"), not just the action ("Confirm delete").
- Trial expiration: "Your data will be deleted in 3 days" is more motivating than "Upgrade to keep your data."
- Unsaved changes: warn clearly before navigation away; show exactly what will be lost.

### Inattentional Blindness (注意力盲区)

- When focused on a task, users fail to notice information outside their attention focus.
- Important alerts placed far from the user's current focus are effectively invisible.

Practical rules:
- Place critical feedback near the user's point of action (inline validation, not page-top banners).
- Don't rely on peripheral notifications for urgent information during focused tasks.
- If you must interrupt, use the user's current focus area (inline message or modal), not a distant toast.

---

## C) Interaction Flow & Rhythm

### Interruption Cost (中断成本)

- Every interruption (modal, page redirect, loading spinner) has a cognitive recovery cost.
- Users need time to re-orient after each interruption, and some never return.

Practical rules:
- Prefer inline interactions over modals; prefer modals over page redirects.
- If a sub-task can be completed in the current context, don't navigate away.
- Batch confirmations: one confirmation for a batch operation, not one per item.

Review question: How many page jumps or modal interruptions does it take to complete the primary task?

### Action Momentum (操作动量)

- Users build rhythm during sequential operations; design should sustain, not break, this rhythm.
- Unexpected pauses or confirmations in the middle of a flow feel jarring.

Practical rules:
- In batch/sequential workflows, don't require confirmation at every step.
- Tab order between form fields should follow the natural reading/input sequence.
- Auto-advance where appropriate (e.g., after selecting from a dropdown, focus moves to the next field).

### Reversibility Principle (可逆性原则)

- Users explore more confidently when they know actions can be undone.
- Irreversibility creates hesitation and anxiety.

Practical rules:
- Provide undo for common actions (delete, move, edit).
- Non-destructive actions should not require confirmation dialogs — let users act and undo.
- For truly irreversible actions, make the consequences explicit and require deliberate confirmation (e.g., type the name to confirm).

---

## D) Attention Economy

### Visual Weight Budget (视觉权重预算)

- A page has a finite attention budget. Emphasizing too many things = emphasizing nothing.
- Every bold element, bright color, or large size competes for the same limited attention.

Practical rules:
- One visual focal point per screen section (the primary CTA or key metric).
- Secondary information: reduce contrast, size, or weight to create clear hierarchy.
- If everything looks important, re-evaluate: what is the ONE thing the user should do or notice here?

Review question: Close your eyes, then open them — is the first thing you see the most important thing on the page?

### Scanning Patterns (扫描模式)

- Users don't read; they scan. Common patterns: F-shape (content pages) and Z-shape (landing pages).
- Key information must be on the scanning path or it will be missed.

Practical rules:
- Place critical information at the top-left and in headings (F-pattern entry points).
- In data tables, put the most important column on the far left.
- Use visual anchors (bold text, icons, color) to create "scan stops" at key information.
- Front-load sentences and labels: put the differentiating word first ("Save draft" vs "Draft — save").

Review question: If the user spends only 3 seconds scanning, can they extract the most critical information?
