# Nexa DevParity Capture Protocol and Infrastructure

This directory defines the reproducible test manifest schema, test case inventory, and capture protocol for validating 100% behavioral and visual parity between Nexa Ahead-Of-Time (AOT) builds and Nexa DevRuntime interactive sessions.

---

## 1. Overview & Core Contract

| Aspect | AOT Mode | DevRuntime Mode |
|---|---|---|
| **Compilation** | Direct native SwiftUI (`nexa-backend-swift`) or Compose (`nexa-backend-kotlin`) | Dynamic Dev IR lowering (`nexa-dev-ir`) served over WebSocket |
| **Runtime Target** | Statically generated release binary | Debug host with dynamic renderer (`NexaDevRenderer`) |
| **Observation** | Native layout geometry, accessibility tree, and presented frames | Same layout geometry, accessibility tree, and presented frames |
| **Verification Gate** | Pixel-exact or justified tolerance comparison after presented-frame fence |

> [!IMPORTANT]
> A transport acknowledgement (`Acknowledge`) proves only that a module or patch was committed to memory. Screenshot harnesses must fence on `FramePresented` to ensure the native rendering framework has actually completed frame presentation for the target revision.

---

## 2. Capture Manifest Schema

Each parity capture must produce a companion `<phase>.manifest.json` describing the exact provenance and environment:

```json
{
  "fixture": "render_parity",
  "platform": "ios",
  "mode": "dev",
  "source_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "compiler_build": "0.1.0",
  "host_capability_hash": "a1b2c3d4e5f6",
  "module_revision": "sha256:7b54...",
  "device": "iPhone 18 Pro Max",
  "os_version": "iOS 18.2",
  "ui_locale": "en-US",
  "font_scale": 1.0,
  "density": 3.0,
  "appearance": "light",
  "event_sequence": [
    "launch",
    "phase:navigation",
    "wait:frame_presented"
  ],
  "capture_phase": "navigation"
}
```

### Manifest Fields:

| Field | Type | Description |
|---|---|---|
| `fixture` | `string` | Name of the test fixture (e.g. `render_parity`, `navigation-v2-duplicate-push`) |
| `platform` | `string` | `ios` or `android` |
| `mode` | `string` | `aot`, `dev_cold`, or `dev` (hot reload / incremental) |
| `source_sha256` | `string` | SHA-256 hash of the root `.nx` application source |
| `compiler_build` | `string` | Nexa compiler version or commit identifier |
| `host_capability_hash` | `string` | Fingerprint of native host template and plugin capabilities |
| `module_revision` | `string` | Module revision token under test |
| `device` | `string` | Connected simulator, emulator, or hardware device identifier |
| `os_version` | `string` | Host operating system and platform version |
| `ui_locale` | `string` | Active locale code (e.g. `en-US`, `ar-SA`) |
| `font_scale` | `float` | Platform font scale multiplier |
| `density` | `float` | Screen pixel density ratio |
| `appearance` | `string` | `light` or `dark` |
| `event_sequence` | `array<string>` | Ordered list of actions preceding this capture |
| `capture_phase` | `string` | Named phase identifier |

---

## 3. Protocol Synchronization

When running hot reload parity evaluations:
1. Export `NEXA_DEV_FRAME_FENCE=1` to arm frame synchronization.
2. The dev server transmits `ServerMessage::FrameFence { enabled: true }` to connected runtimes.
3. Upon committing a module or patch and rendering it to the display, the native runtime responds with `ClientMessage::FramePresented { revision, frame_id }`.
4. Capture scripts await this confirmation in the dev log before querying accessibility nodes or capturing simulator/emulator framebuffers.
