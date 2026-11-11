**Scope:** TextInput · iOS / Android · AOT / DevRuntime · simulator verification

The shared input primitives now own native editing, validation, sizing, and focus. Application code keeps the existing TextInput API.

```nx
TextInput(value: notes, placeholder: "Delivery instructions", multiline: true,
    minLines: 3, maxLines: 5, maxLength: 240)
```

## Findings and changes

| Finding | Correction |
|---|---|
| iOS model validation could leave rejected characters visible in the native buffer | Validate native UITextField / UITextView delegate edits before accepting them |
| SwiftUI FocusState did not address the UIKit editor | Bind each native editor directly to its scoped Boolean state |
| Placeholder and edited text could use different heights | Use stable single-line font metrics and matching Compose text styles |
| Length truncation could split Unicode text or remove the existing suffix | Limit grapheme clusters and preserve unchanged text around an insertion |
| Multiline maximum only controlled visible height | Reject edits increasing explicit newline count beyond maxLines |
| Android IME animation restarted focus relocation each frame | Observe keyboard visibility only while the input is focused |

## Defined behavior

- `minLines` controls minimum visible height.
- `maxLines` limits explicit newline-separated lines and maximum visible height. Soft wrapping remains editable and scrolls within the field.
- Rejected edits emit no change event. Deletion remains possible after rejection.
- Programmatic values are retained, including existing data above a newly lowered limit.
- Native IME composition remains intact until commit. Secure fields disable correction and capitalization.
- AOT and DevRuntime use the same native input and edit-policy implementations on each platform.

## Repeatable regression checks

```sh
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test -p nexa-cli --test native_text_input_edit
python3 tests/native_text_input_check.py --android-device emulator-5554 --require-device
python3 tests/native_text_input_control_check.py --ios-device <booted-simulator-udid> --require-device
```

The shared corpus covers newline normalization, line rejection, grapheme limits, middle insertions, pasted text, and editing existing over-limit values. Native iOS delegate checks additionally verify editing-buffer agreement, callbacks, submission, and measured height. These commands create no screenshots.

Simulator app interaction checks also exercised Focus email, clear focus, single-line height, multiline growth, rejection, and deletion. Human approval of TextInputCases remains pending; simulator checks do not establish physical-device keyboard or performance acceptance.
