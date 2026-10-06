# Nexa Operational Learnings

Hard-won knowledge that is **not** derivable from reading the code. Anything the compiler can be
asked directly should be asked directly rather than recorded here — see the last section.

## 1. A clock is not an identity

`SystemTime::now().as_nanos()` is not unique. Measured on one machine, 320,000 clock reads produced
40,838 distinct values, and 35,377 were handed out more than once. Reproducing the old temp-directory
naming scheme under concurrency:

```text
4,500 directory creations -> 1,470 distinct paths
1,026 paths shared by two or more owners (worst case: 8)
```

A collision is silent because `create_dir_all` on an existing directory is a no-op, not an error.
Two tests share a tree, whichever finishes first deletes the other's files, and the failure surfaces
as `NotFound` on a write — only under load. Ownership must come from the OS: `create_dir` is atomic
and fails when a name is taken. `nexa_testkit::TempDir` and `VacantDir` do this and are the only
supported way to name a temporary directory in a test.

## 2. Type erasure is not a style preference

`AnyView` around a section header, sticky header, or row destroys SwiftUI view identity across
layout passes, forcing a heap allocation per scroll event. The generated runtime solves this with
generic parameters and defaulted unused slots — `NexaFastList<RowContent, HeaderContent>` and
`NexaFastSectionedList<RowContent, HeaderContent>`, specialized as `NexaFastList<_, EmptyView>`
when a slot goes unused. Introducing `AnyView` "temporarily" reintroduces the cost silently.

## 3. Compose stability reports are not measurements

`mutableStateOf(0.0)` allocates `MutableState<java.lang.Double>` and boxes on every mutation, and
`mutableStateOf<Any>` is worse. Use the primitive-specialized holders. But do not report these as
*slower* without a profile: they are wrong by construction, which is a different claim from slow.
Reading `LazyListState.layoutInfo` inside `snapshotFlow` without `.distinctUntilChanged()` emits on
every sub-pixel offset — that one is measurable.

## 4. The `.nxid` lexer discards comments and spans

`nexa-plugin-idl` skips `//` to end of line while lexing, and no model node carries a line or
column. Two consequences that shape plugin work:

- Doc comments in a `.nxid` never reach `PluginIdl`, so a generator cannot render the "Description"
  column the plugin READMEs carry. Generating those tables requires comment retention in the lexer
  first.
- Errors are bare `String`s with private `Location`, so a diagnostic cannot point at the declaration
  it rejected.

`Field::default` being `None` *is* the requiredness signal: the compiler treats a defaultless
property as required. A README claiming a default for such a property is wrong, not merely
incomplete.

## 5. Postfix `?` is four operators

In `.nx`, `?` is optional chaining (`obj?.field`), optional indexing (`arr?[index]`), coalescing
(`expr ?? default`), and `Result` propagation (`expr?`). In `nexa-syntax` the disambiguation is
positional: a following `.` or `[` means optional access, otherwise it parses as `ast::Expr::Try`.
`nexa-compiler` then checks that the inner expression has type `Result<ok, err>` and lowers to
`nexa_ir::Expr::Try` returning `ok`. Getting this backwards silently accepts invalid programs.

## 6. Dead code elimination changes what tests prove

`nexa-compiler`'s pass in `optimize.rs` drops top-level `fn` declarations not reachable from UI
event handlers, state initializers, or other reachable functions. A function that "works" in a unit
test but is never rooted in app-reachable code will not appear in generated output — so a generated-
source assertion can fail for a reason that has nothing to do with the function's body.

## 7. Verify with the tool, not with recall

Most "facts" about this compiler can be asked directly, and asking is cheap:

```bash
nexa check                       # accepts or rejects a `.nx` snippet, naming the exact error
cargo test -p nexa-syntax        # catalog probes must parse
grep -n 'Signature:' docs/syntax-audit.md   # the generated grammar reference
```

Every skill in `.agents/skills/` used to restate component arguments, CLI flags, and platform
minimums by hand. All of it drifted — the README claimed 14 of 16 plugin iOS floors incorrectly, a
skill advertised `nexa build` and `nexa generate` commands that do not exist, and `.onPress` was
documented where the grammar accepts only `.onTap`. The fix is structural, not editorial: those
tables are now generated from their sources of truth and gated by tests, and the skills point at the
generated references instead of copying them. When you find a fact worth recording, first check
whether a machine can be the source of it.
