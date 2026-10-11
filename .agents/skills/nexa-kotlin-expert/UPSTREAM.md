# Upstream Compose skills

![Third-party skill sources](https://img.shields.io/badge/skill--sources-pinned-blue)

This metadata records the MIT-licensed upstream Compose skills vendored by the Nexa Kotlin
adapter. It pins sources for reproducible updates and preserves each author's license.

## Quick start

```text
Use the Nexa Kotlin expert adapter, then read the relevant vendored Compose skill and reference.
```

This Nexa skill vendors two independent Compose references. Ivan Morgillo's Compose Agent is the
primary authoring/review workflow, with focused Kotlin, coroutine/Flow, lifecycle, and UI guidance.
Adit Lal's Compose Expert supplies deeper UI and runtime guidance plus source receipts.

| Skill | Source | Pinned revision | License |
|---|---|---|---|
| Compose Agent 4.4.0 | [hamen/compose_skill](https://github.com/hamen/compose_skill/tree/75ba3d6d6552119f9a0aec84c1ab0a49a838bbdf/skills/compose-agent) | `75ba3d6d6552119f9a0aec84c1ab0a49a838bbdf` | MIT; see the vendored `LICENSE` |
| Compose Expert 2.4.0 | [aldefy/compose-skill](https://github.com/aldefy/compose-skill/tree/954ef54ea32288fbc90745f012d09d7b791f0d8a/skills/compose-expert) | `954ef54ea32288fbc90745f012d09d7b791f0d8a` | MIT; see the vendored `LICENSE` |

The skill's `references/source-code/` files include AndroidX and Compose Multiplatform source
material, each retaining its upstream copyright and Apache License 2.0 header. The Apache 2.0 text
is included at `references/source-code/LICENSE-APACHE-2.0.txt`.

Review the upstream repository and its current APIs before updating this pin. Keep the Nexa adapter
in `SKILL.md`: apply the guidance to the generated host and its actual dependency versions, while
preserving Nexa's typed AOT architecture and iOS/Android parity requirement.
