# Kathamo Agent Skills

Kathamo is an application-independent native Rust UI and UX framework. The skills in this directory teach developers and coding agents to build, review, and extend user interfaces **with Kathamo**. They are not application implementations.

| Skill | Use for |
| --- | --- |
| [kathamo-framework](kathamo-framework/SKILL.md) | Core, input, rendering, editor hosting, public framework APIs |
| [kathamo-ux-architect](kathamo-ux-architect/SKILL.md) | Workspace usability, visual hierarchy, interaction design |
| [kathamo-ui-builder](kathamo-ui-builder/SKILL.md) | Composing consuming applications with public Kathamo APIs |
| [kathamo-visual-qa](kathamo-visual-qa/SKILL.md) | Native interaction, accessibility, visual and performance verification |
| [kathamo-agent-handoff](kathamo-agent-handoff/SKILL.md) | Scoped multi-agent change management and handoff |

## Routing

- Designing a screen: read **ux-architect**, then **ui-builder**.
- Adding framework primitives: read **framework** and **visual-qa**.
- Reviewing a UI: read **visual-qa**; include **ux-architect** for usability.
- Multi-agent work: read **agent-handoff** before starting and at handoff.

Always inspect the checked-out source and current public API before using examples. A proposed interface must never be described as implemented until proven.

## Repository and privacy boundary

These skills use public Kathamo APIs and standard Rust tools. Keep private research, screenshots, session logs, and scratch files outside the public repository; no private Kalponic service or application is required to use them.

No third-party implementation or interface is a source file for this project. Kathamo uses original terminology, public contracts, components and design decisions.

The existing [AGENT_GUIDE.md](../AGENT_GUIDE.md) is additional context; verify its examples against the current code.
