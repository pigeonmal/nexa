# Upstream Swift skills

![Third-party skill sources](https://img.shields.io/badge/skill--sources-pinned-blue)

This metadata records the two MIT-licensed upstream skills vendored by the Nexa Swift adapter.
It pins sources for reproducible updates and preserves each author's license.

## Quick start

```text
Use the Nexa Swift expert adapter, then read the relevant vendored upstream skill and reference.
```

This Nexa skill vendors two upstream skills by Paul Hudson, the creator of Hacking with Swift.
Their source instructions and reference files are kept intact under `upstream/`.

| Skill | Source | Pinned revision | License |
|---|---|---|---|
| SwiftUI Pro 2.0.0 | [twostraws/swiftui-agent-skill](https://github.com/twostraws/swiftui-agent-skill/tree/2.0.0/swiftui-pro) | `f9800713b24580bc444931949aad4519128605e8` | MIT; see the vendored `LICENSE` |
| Swift Concurrency Pro 1.0 | [twostraws/Swift-Concurrency-Agent-Skill](https://github.com/twostraws/Swift-Concurrency-Agent-Skill/tree/1.0.0/swift-concurrency-pro) | `6982bd3fb3033202e2e5d73038ca321db2e1cf34` | MIT; see the vendored `LICENSE` |

Review the source repository before updating a pin. Keep the Nexa adapter in `SKILL.md`: upstream
guidance must be interpreted against Nexa's actual toolchain, AOT output, and cross-platform parity
requirements.
