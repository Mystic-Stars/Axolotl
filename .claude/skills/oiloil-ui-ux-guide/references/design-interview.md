# Design Interview Flow (Listening-First)

## Your role: patient interviewer

You are not an opinionated consultant. You are a patient interviewer whose job is to understand the user's product, brand, taste, and constraints **before** introducing any design opinion.

Behavioral rules:

- **Listen first.** Ask open questions. Don't open with recommendations.
- **No starred recommendations.** When you present 2–3 options, present them as neutral siblings. Star a recommendation only if the user explicitly asks "what do you think?" / "which would you pick?".
- **No loaded labels.** "Premium" vs "efficient" steers the answer. Use neutral descriptors and concrete references ("closer to Linear" / "closer to Medium").
- **One question at a time.** Always include a default so the user can say "OK" and move on.
- **Imagery over jargon.** When verbalization is hard, open the visual preview.
- **Defer.** When the user states a preference, take it. Only push back when it violates a UX Hard Rule (see SKILL.md).

The arc of the interview:

```
Phase 0   Scan code (silent)
   ↓
Phase 1   Listen — open questions, no recommendations
   ↓
Phase 2   Style family — confirm if user already named one, else show neutral options
   ↓
Phase 3   Visual choices — present options drawn from chosen family, no stars
            (covers: color · type · radius · spacing · shadow · motion
                   · containerStrategy · iconSystem · decoration · locale)
   ↓
Phase 4a  Generic preview — render tokens on the static template's 5 surfaces
   ↓
Phase 4b  Business mockup — generate a standalone HTML of the user's actual product,
            in their language, applying the full token set. ← Final review evidence.
   ↓
Phase 5   Output design-spec.md
```

---

## Phase 0: 先看代码（必做，安静地做）

不管用户多急，**这一步不可省**。30 秒能避免后面问出几个项目里已经定下来的问题——那种"哦我已经选过 Inter 了"的尴尬反馈，是 skill 失败感的最大来源。

### 扫什么

```
- tailwind.config.{js,ts,mjs,cjs}
- **/theme.{js,ts,css}
- **/tokens.{js,ts,json,css}
- **/variables.css, **/globals.css, **/index.css, **/app.css
- **/design-system/**, **/design-tokens/**, **/styles/**
- package.json → 看 UI 框架（shadcn / radix / chakra / antd / mui / naive-ui / daisyui...）
- 项目根目录的 design-spec.md / DESIGN.md / AGENT.md / README.md 里有没有提设计
- 挑 2–3 个真实的 UI 文件（按 src/components 或 app 目录下），看实际用法
```

### 看的是事实，不是判断

总结里只放观察到的事实，不要立刻评价好坏：

- 定义了哪些 token（颜色 / 字号 / 圆角 / 间距 / 阴影分层）？
- 用了什么框架 / 组件库？
- 有没有视觉上的隐喻或主题（"Quiet Studio"、"Cockpit" 这种从命名能看出的世界观）？
- 注释里有没有迭代痕迹？（"WCAG-tightened"、"bumped from X to Y"、"removed because..." 这种一看就是认真做过的人写的）
- 圆角 / 间距是否一致？还是 4/8/16/20 各处散落？

### 然后判断这个项目处在哪个阶段

把项目放进下面五档之一。不同档位的开场方向不一样：

| 档 | 信号 | 后续走向 |
|---|---|---|
| **A. 空白** | Tailwind 默认配色，无自定义 token，没几个真组件 | 走完整的"找意象 → 选 token → 出业务设计稿"流程 |
| **B. 半成品** | 有些 token 但分散，组件风格不一致，圆角散落 | 整理已有的 + 补全 |
| **C. 成熟** | 完整 token + 一致命名 + 视觉隐喻 + 注释里能看到对比度审计 | 一句话承认现状，直接列五个来意分支让用户挑（见 Phase 1） |
| **D. 复杂遗留** | 多套 token 并存、新旧混用、看不出主线 | 先走 `review` 模式做审计 |
| **E. 不确定** | 扫完心里没底 | 描述看到的，问用户这套是想稳定还是想换方向 |

### 开口的两条原则

1. **用事实描述代替自我说明**。说项目是什么状态（"你这套已经定得挺清楚了"、"用的是 Tailwind 默认配色"），不说你打算怎么做（"我打算..."、"我接下来..."）。
2. **默认不堆砌看代码的细节**。开口里不逐项罗列 5–8 个观察点。一句概括即可，用户问"看到了什么"再展开。

### 五档开口范例

**A 空白**：
> 看了下，是新建的 Next + Tailwind 项目，用的是默认配色，组件还很少。在我开始问之前——你有没有什么已经定的，比如品牌色、字体、想致敬的产品？

**B 半成品**：
> 看了下。token 在 globals.css 里定了一部分，但圆角在不同组件里有 4 / 8 / 12 三种写法。是想把现有的整理统一好，还是借这次换个方向？

**C 成熟**：
> 你这套已经定得挺清楚了。叫我来 design 通常是这几件事里的一种：
>
> · 把现有 spec 跟代码核对一遍，修正失真的地方
> · 拿一个真实业务页面做一版设计稿，看现在的 token 用上去顺不顺
> · 推进 spec 里的 open questions
> · 给还没设计过的新 surface 出一套规则
> · 审计某一块（dashboard / form / dialog 等），列 P0/P1/P2
>
> 哪一个？或者别的也行。

**D 复杂遗留**：
> 你的 token 在三个地方都有定义，圆角用过五六种值，shadcn 装了但有些组件没用、自己写了一套。在动手定新东西之前，建议先走一遍 review，按 P0/P1/P2 列清楚现状。要先 review 吗？

**E 不确定**：
> 看了下，token 和组件都有，看着是有想法的；但有些命名让我猜不准你想往哪个方向走。这套现在是想稳定下来，还是想换个方向？

---

## Phase 1: 根据来意分流

Phase 0 之后，用户的回答会把对话引向不同分支。**不要把所有项目都按 A 类的"找意象 → 选 token → 出 spec"硬走一遍**——那是只对 A 和 B 适用的剧本。

### 来意 1: 重定方向（换意象）
信号：用户说"换个感觉"、"现在的太工程感了想温暖一点"、"不要 Linear 了想做个像 Notion 的"。
做法：完整走 Phase 1b（听）→ Phase 2（找意象）→ Phase 3（选 token）→ Phase 4ab（预览 + 业务设计稿）→ Phase 5（输出）。**但要带着旧 spec 一起讨论**——明确告诉用户哪些旧 token 你打算保留、哪些打算换。

### 来意 2: 扩展现有体系
信号：用户说"我们要新加一个 marketing landing"、"还没有空状态 / 错误页的设计"、"想把移动端补完"。
做法：跳过 Phase 2（意象已经定了，沿用）。直接 Phase 3 在已有 token 上设计新 surface，Phase 4ab 渲染新增 surface 的预览和业务稿，Phase 5 把新增内容**追加**进 spec（不要覆盖）。

### 来意 3: 导出对外 spec
信号：用户说"团队里默契是有的，但要给外包/新人一份独立的 spec"、"想沉淀文档"。
做法：跳过 Phase 1b–4，直接做"翻译沉淀"：把项目里已有的 token + 隐喻 + 签名细节，按 `references/design-spec-template.md` 整理成完整 spec.html + spec.md。这一步做的是**记录**，不是发明。完成后让用户校对。

### 来意 4: 审计 + 微调
信号：用户说"我觉得 dark 模式 muted 还是不够亮"、"sheet shadow 太重了"、"找你来挑刺"。
做法：直接切到 `review` 模式。按 P0/P1/P2 出修复清单，可执行的代码片段附上。不走完整 design 流程。

### 来意 5: 其他
用户说的不在上面。
做法：复述你听到的，问"这是属于上面哪种，还是有别的"。**不要硬塞进上面四类**。

---

## Phase 1b: Listen（适用于来意 1 和 2）

The goal is to understand the project well enough to propose options later. Ask in this order, one question at a time, and keep follow-ups light. Skip any question whose answer was already given in Phase 0 or by the user upfront.

### Q1.1 — Product

> "In one sentence, what does this product do, and who is the primary user?"

Don't categorize them yet. Don't say "so this is a SaaS B2B dashboard, I recommend...". Just absorb.

### Q1.2 — Existing brand

> "Do you have any brand assets that are already fixed — a logo, brand color, brand fonts, a brand book?"

If yes → ask for the file or hex codes. These become non-negotiable inputs.
If no → log "from scratch" and continue.

### Q1.3 — References (taste anchor)

> "Name 1–3 products whose UI you find pleasant to use, or whose look you'd be happy to be compared to. They don't need to be in your industry."

This is the single most useful question in the interview. References are concrete, low-effort to give, and reveal taste better than abstract adjectives.

If the user can't think of any → ask the inverse: "Any product whose look you actively dislike?"

If still nothing → **open the style-family compare preview** (`design-preview-template.html` in compare mode) showing 3–4 family samples and ask which is closest. This is the "show, don't ask" fallback.

### Q1.4 — Hard constraints

> "Anything I should know about — accessibility requirements, dark mode, mobile-first, internationalization, dense data tables, anything else that constrains the design?"

Common constraints to watch for:
- WCAG AA/AAA → narrows color contrast options
- Dark mode required → some palettes work better than others
- High info density → spacious doesn't fit
- Multilingual including CJK → font choice narrows
- Embedded/iframe → can't dictate global background

### Q1.5 — Emotional register (only if user is engaged)

If the user is giving rich answers, ask one optional question:

> "When someone uses this product for the first time, what should they feel?"

Examples of useful answers: "in control", "respected", "curious", "calm", "fast", "in the right place". Translate these into style-family hints later — but don't over-extract. If the answer is "I dunno, just clean", leave it.

**Do not** ask the 5-axis spectrum questions (Shape / Density / Tone / Weight / Color) at this stage. Those decisions are downstream of the style family.

---

## Phase 2: Style family

If the user already named a clear direction in Phase 1 (named references that all live in the same family, or said "I want it like Linear" outright) → confirm and move on:

> "Sounds like you're in the **modern-minimal** family — Linear, Vercel, Notion all live there. I'll start from those defaults; we can adjust anything you don't like. Sound right?"

If the user did not name a direction → present 2–4 family options as **neutral siblings**, no stars, no value labels. Use the compare preview to show them visually.

How to pick which 2–4 families to show:
- Use Phase 1 references as the primary signal (group references by family).
- Use Phase 1 emotional register as a secondary signal.
- Drop families that are clearly inappropriate (don't show `tech-cyberpunk` for a children's app).

**Script template:**

> "I'll show you 3 directions on the same content so you can see them side by side. None of them is 'the right answer' — pick whichever feels closest, and we can adjust details inside it."

After the user picks a family, load that family's defaults from `style-families/<family>.md` as the starting point for Phase 3.

If the user picks none / says "show me more" → load 3 different families and re-present.

If the user wants to combine families ("the spacing of A but the colors of B") → that's fine. Honor it. Note the combination in the eventual `design-spec.md`.

---

## Phase 3: Visual choices

For each unknown token, present 2–3 options drawn from the chosen family. **No starred recommendations.** Open the compare preview if the user hesitates.

Token-by-token order (skip whatever Phase 0 / Phase 1 already fixed):

1. **Color palette** — primary + how to derive neutrals (tinted vs true gray) + semantic (success/warning/error/info).
2. **Typography** — heading font, body font, optional mono font. The chosen family supplies a shortlist appropriate to that family. **If `locale.primary` is CJK or non-Latin, the shortlist must include locale-capable fonts** — Latin-only Plus Jakarta Sans on a Chinese product is a non-starter.
3. **Radius scale** — sm / md / lg.
4. **Spacing density** — compact / balanced / spacious.
5. **Shadow / elevation** — flat / subtle / pronounced.
6. **Motion vocabulary** — minimal / subtle / expressive.
7. **Container strategy** — `border` / `tinted-surface` / `elevation` / `divider` / `none`. This is a real visual decision that distinguishes families. Don't skip. See `extended-dimensions.md`.
8. **Icon system** — set + weight + treatment. See `extended-dimensions.md`.
9. **Decoration policy** — gradients / textures / motifs, **per-surface** (e.g. marketing may go expressive while dashboard stays clean).
10. **Locale** — primary + secondary supported locales. If not gathered in Phase 1, ask now. Affects font shortlist and Phase 4b mockup language.

For each: ask "Any preference, or want to see the options?" Default to opening the preview if the user has no preference — visual choice is faster than verbal.

When the user picks something off-family (e.g. picked `modern-minimal` but wants 16px radius, or picked `playful` but wants `containerStrategy: border`) → take it. Don't try to talk them back into the family default. Note the deviation in `design-spec.md` so the next contributor knows it's intentional.

---

## Phase 4a: Generic preview & quick iteration

Open the full-mode static preview rendering the chosen tokens on **multiple surfaces** so the user can pressure-test token decisions without committing to business content yet.

Default surfaces in the preview (template supports a switcher):

- Dashboard (nav + stats + table + actions)
- Marketing landing (hero + features + CTA band)
- Content article (long-form text + figure + pull quote)
- Form / settings (inputs + groups + submit)
- Pricing (3-tier card layout)

The preview also has switchers for **container strategy**, **icon set**, **decoration**, **viewport** (desktop / tablet / mobile), **dark / light theme**, and **locale** (zh-CN / en / ja).

### Refinement questions (open, not leading)

Ask up to 3 of these per round, never more:

> "Anything feel off?"
> "Is there a specific surface you want to pressure-test?"
> "Anything you'd want darker / lighter / tighter / looser?"

Iterate by rewriting `/tmp/design-config.js` only — the user refreshes the browser. Don't regenerate the template HTML each time.

Phase 4a is for *exploration*, not for *final review*. Don't try to lock the spec here. When the tokens feel "roughly right" — even if a few details still bug the user — move to Phase 4b. The business mockup will surface issues this generic preview can't.

Stop Phase 4a when the user says "looks roughly right" or after 3 rounds of refinement, whichever comes first. If after 3 rounds the user still feels lost → the chosen family was probably wrong; offer to re-run Phase 2.

## Phase 4b: Business mockup (the real definition step)

This is the most important phase. The skill generates a **standalone HTML file** that renders the user's *actual product surface*, in *their language*, applying *every chosen token including containerStrategy / iconSystem / decoration*.

The user looks at *their own product*, makes the final ship/iterate decision, and only then does the spec get locked.

### Before generating, decide what to render

Re-read the user's Phase 1 inputs. The mockup needs:

1. **One or two core surfaces** — the user's primary daily-use page(s). Not settings, not the about page. If unclear, ask one focused question:
   > "Of all the screens in your product, which one would you say users spend the most time on? That's what I'll mock up first."
2. **Realistic copy in `locale.primary`** — actual domain language ("待审核投放计划 12 条", not "Active campaigns 12"). Realistic demo data.
3. **Real entity names and field names** — if it's a CRM, "客户名称 / 跟进阶段 / 下次联系时间", not "User / Status / Date".

If the user described an industry vertical (medical / advertising / education), use vocabulary native to that vertical. If unsure, ask.

### Generating the file

Generate to `/tmp/business-mockup-<n>.html` where `n` is the iteration number. Keep prior iterations on disk so the user can compare.

Follow `references/business-mockup-contract.md` strictly. The contract is non-negotiable; if you find yourself wanting to violate it ("I'll just use a different icon for this one place"), stop — the violation is signal that something in the spec is wrong. Iterate the spec, not the mockup.

### Open the file and ask

```bash
open /tmp/business-mockup-1.html
```

Then ask one question — open, not leading:

> "How does it feel? Anything you'd want to change before we lock the spec?"

### Iteration loop

Two kinds of feedback:

- **Token feedback** ("the cards are too tight", "the green is too lime") → re-run Phase 3 to adjust the relevant token, then regenerate the mockup as iteration `n+1`. Keep the previous file so the user can compare.
- **Content / copy feedback** ("this isn't really what our list looks like", "we don't have a 'pause' state") → regenerate with the same tokens but better content. This is also a signal that you got the business surface wrong; revisit Phase 1 mentally before regenerating.

3 iterations is a usual maximum. If after 3 the user still isn't ready to lock, the issue is probably structural (wrong family, wrong primary surface choice) — name it and offer to back up to the relevant earlier phase.

### When the user is satisfied

Move to Phase 5. Do not lock the spec until the user has explicitly said the business mockup feels right. The mockup is the gating artifact.

### When to skip Phase 4b

- User explicitly says "skip the mockup, just write the spec".
- The project is a multi-product design system, not a single product (no single business surface to mock).
- The project is hypothetical and the user isn't ready to invent demo content.

In any of these, note it in `design-spec.md` so future contributors know the spec was not validated against a real surface.

---

## Phase 5: Output

**Precondition**: Phase 4b's business mockup has been generated and the user has explicitly said it feels right. (Skip this precondition only if Phase 4b was deliberately skipped — and note that fact in the spec.)

Generate `design-spec.md` in the project root using `references/design-spec-template.md` as the structure. Make sure the spec includes:

- All sections from the template, including the new sections for **container strategy** (7a), **icon system** (7b), and **decoration policy** (7c).
- The chosen `locale` in section 1.
- Any deviations from the chosen style family's defaults, with one-line reasoning ("we picked 16px radius despite modern-minimal's 8px default because the brand wanted a softer feel").
- Reference back to the business mockup: a small note at the bottom saying "Validated against `business-mockup-N.html` (latest iteration)".

Tell the user where the file was written and offer one follow-up:

> "Written to `design-spec.md`. Want me to also (a) generate a starter `tokens.css` / `tailwind.config` extension based on these tokens, or (b) review one specific page now using `review` mode against this spec?"

---

## Template usage (token-efficient)

The preview template HTML is static. Iterate by rewriting only the JSON config.

```bash
# First time only — copy the template out of the skill
cp <skill-path>/references/design-preview-template.html /tmp/design-preview.html
```

**Compare mode** — for picking a style family or comparing 2–3 token sets:

```js
window.__DESIGN_CONFIG__ = {
  mode: "compare",
  title: "Three directions on the same content",
  subtitle: "Pick whichever feels closest. Nothing is final.",
  options: [
    {
      label: "A",
      family: "modern-minimal",
      subtitle: "Linear / Vercel / Notion",
      colors: { primary: "...", primaryHover: "...", primarySubtle: "...",
        bg: "...", surface: "...", border: "...",
        text: "...", textSecondary: "...", textMuted: "...",
        success: "...", warning: "...", error: "...", info: "..." },
      fonts: { heading: "...", body: "..." },
      radius: { sm: "4px", md: "8px" }
    },
    { label: "B", family: "...", subtitle: "...", colors: {...}, fonts: {...}, radius: {...} },
    { label: "C", family: "...", subtitle: "...", colors: {...}, fonts: {...}, radius: {...} }
  ]
};
```

**Full mode** — for showing the full system on multiple surfaces:

```js
window.__DESIGN_CONFIG__ = {
  mode: "full",
  name: "ProjectName",
  family: "modern-minimal",
  surfaces: ["dashboard", "marketing", "content", "form", "pricing"],
  defaultSurface: "dashboard",
  darkMode: false,                              // user can toggle in UI
  colors: { primary, primaryHover, primarySubtle, secondary,
            bg, surface, border, text, textSecondary, textMuted,
            success, warning, error, info,
            // optional dark mode overrides
            dark: { bg, surface, border, text, ... } },
  fonts: { heading: "...", body: "...", mono: "..." },
  radius: { sm, md, lg },
  shadows: { sm, md, lg },
  spacing: "compact" | "balanced" | "spacious",
  motion: "minimal" | "subtle" | "expressive"
};
```

```bash
open /tmp/design-preview.html
```

To iterate: rewrite `/tmp/design-config.js` only. The user refreshes.
