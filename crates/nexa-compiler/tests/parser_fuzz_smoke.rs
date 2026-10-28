use std::panic::{AssertUnwindSafe, catch_unwind};

use nexa_compiler::compile;
use nexa_syntax::parse_program;

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn malformed_source_fuzz_smoke_returns_errors_instead_of_panicking() {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_ \n\t{}()[],:.;\"'!@#$%^&*+-=/<>?";
    const SEED: &str = "app Fuzz { state count: Int32 = 0 body { Text(\"ready\") } }";

    let mut random = 0x4e45_5841_4649_4552_u64;
    for case in 0..512 {
        let length = (next_random(&mut random) % 129) as usize;
        let mut source = String::with_capacity(length + SEED.len());
        for _ in 0..length {
            let index = (next_random(&mut random) as usize) % ALPHABET.len();
            source.push(ALPHABET[index] as char);
        }

        let parsed = catch_unwind(AssertUnwindSafe(|| parse_program(&source)));
        assert!(
            parsed.is_ok(),
            "parser panicked for fuzz case {case}: {source:?}"
        );

        let compiled = catch_unwind(AssertUnwindSafe(|| compile(&source)));
        assert!(
            compiled.is_ok(),
            "compiler panicked for fuzz case {case}: {source:?}"
        );

        let plugin_idl = catch_unwind(AssertUnwindSafe(|| nexa_plugin_idl::parse(&source)));
        assert!(
            plugin_idl.is_ok(),
            "plugin IDL parser panicked for fuzz case {case}: {source:?}"
        );

        let plugin_manifest = catch_unwind(AssertUnwindSafe(|| {
            nexa_plugin_idl::manifest::parse(&source)
        }));
        assert!(
            plugin_manifest.is_ok(),
            "plugin manifest parser panicked for fuzz case {case}: {source:?}"
        );

        let mutation_index = (next_random(&mut random) as usize) % SEED.len();
        let replacement = ALPHABET[(next_random(&mut random) as usize) % ALPHABET.len()] as char;
        let mut mutated = SEED.to_owned();
        mutated.replace_range(mutation_index..mutation_index + 1, &replacement.to_string());
        assert!(
            catch_unwind(AssertUnwindSafe(|| compile(&mutated))).is_ok(),
            "compiler panicked for mutated valid seed in case {case}: {mutated:?}"
        );
    }
}
