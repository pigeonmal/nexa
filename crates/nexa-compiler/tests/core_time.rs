//! The core clock: `Time.*`.
//!
//! The clock is a core API rather than a plugin, so it lowers to its own typed
//! expression instead of a plugin call. These tests pin the fixed return types,
//! the two clocks' separate meanings, the argument checking, and the capability
//! that tells a backend to emit the clock support at all.

use std::fs;

use nexa_ir::{Expr, NumericType, TimeMethod, Type};
use nexa_testkit::TestProject;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};

/// Compiles a whole app source, reporting the compiler diagnostic on failure.
fn compile(name: &str, source: &str) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new(name);
    let entry = project.join("App.nx");
    fs::write(&entry, source).expect("app source should be written");
    compile_file_with_warnings_for_target(&entry, Target::Swift)
        .map(|compiled| compiled.module)
        .map_err(|error| error.to_string())
}

/// Compiles an app declaring `states`, with `body` as the body block's content.
fn compile_with(name: &str, states: &str, body: &str) -> Result<nexa_ir::Module, String> {
    compile(
        name,
        &format!("app ClockApp {{\n{states}    body {{\n{body}\n    }}\n}}\n"),
    )
}

/// Every clock call in the module, with the type it returns.
fn clock_calls(module: &nexa_ir::Module) -> Vec<(TimeMethod, Type, bool)> {
    let mut calls = Vec::new();
    let mut visit = |expr: &Expr| {
        if let Expr::TimeCall {
            method,
            return_type,
            is_async,
            ..
        } = expr
        {
            calls.push((*method, return_type.clone(), *is_async));
        }
    };
    let mut noop = |_: &nexa_ir::Node| {};
    for node in &module.body {
        nexa_ir::walk::walk_ir(std::slice::from_ref(node), &mut noop, &mut visit);
    }
    for actions in [
        &module.on_appear,
        &module.on_disappear,
        &module.on_active,
        &module.on_inactive,
        &module.on_background,
    ]
    .into_iter()
    .flatten()
    {
        nexa_ir::walk::walk_actions(actions, &mut visit);
    }
    calls
}

#[test]
fn the_two_clocks_are_separate_readings_of_one_api() {
    let module = compile_with(
        "nexa-core-clock-reads",
        "    state stamp: String = \"\"\n",
        "        Column(spacing: 16) {\n\
         \x20           Text(stamp)\n\
         \x20           Button(\"Wait\") { stamp = Time.iso8601(timestamp: Time.monotonic()) }\n\
         \x20       }\n\
         \x20       OnAppear async {\n\
         \x20           try {\n\
         \x20               await Time.sleep(milliseconds: 10)\n\
         \x20           } catch {\n\
         \x20           }\n\
         \x20           stamp = Time.iso8601(timestamp: Time.now())\n\
         \x20       }\n",
    )
    .expect("clock reads should compile");

    let calls = clock_calls(&module);
    assert!(
        calls
            .iter()
            .any(|(method, _, _)| *method == TimeMethod::Now),
        "the wall clock should be readable: {calls:?}"
    );
    assert!(
        calls
            .iter()
            .any(|(method, _, _)| *method == TimeMethod::Monotonic),
        "the monotonic counter should be readable beside the wall clock: {calls:?}"
    );
    // A sleep suspends, and cancelling the surrounding task can end it early,
    // so it is the one clock call that throws.
    assert!(
        calls
            .iter()
            .any(|(method, ty, is_async)| *method == TimeMethod::Sleep
                && matches!(ty, Type::Void)
                && *is_async),
        "a sleep should suspend and return nothing: {calls:?}"
    );
    assert_eq!(
        calls
            .iter()
            .find(|(method, _, _)| *method == TimeMethod::Iso8601)
            .map(|(_, ty, _)| ty.clone()),
        Some(Type::String),
        "formatting an instant yields a string"
    );
}

#[test]
fn elapsed_measures_from_a_monotonic_reading() {
    let module = compile_with(
        "nexa-core-clock-elapsed",
        "    state elapsed: Int64 = 0\n",
        r#"        Column {
            Button("Measure") {
                elapsed = Time.elapsed(since: Time.monotonic())
            }
        }
"#,
    )
    .expect("elapsed time should compile");

    let calls = clock_calls(&module);
    assert!(
        calls
            .iter()
            .any(|(method, ty, is_async)| *method == TimeMethod::Elapsed
                && *ty == Type::Numeric(NumericType::Int64)
                && !is_async),
        "elapsed should return an Int64 synchronously: {calls:?}"
    );
}

#[test]
fn local_calendar_day_key_lowers_to_a_typed_time_call() {
    let module = compile_with(
        "nexa-local-calendar-date-formatting",
        "",
        r#"        Column {
            Text(Time.startOfDay(timestamp: Time.now()))
        }
"#,
    )
    .expect("local calendar date helpers should compile");

    let calls = clock_calls(&module);
    assert!(calls.iter().any(|(method, ty, is_async)| {
        *method == TimeMethod::StartOfDay && *ty == Type::Numeric(NumericType::Int64) && !is_async
    }));
}

#[test]
fn calendar_day_addition_is_typed_and_preserves_platform_calendar_semantics() {
    let module = compile_with(
        "nexa-calendar-day-addition",
        "    state dueAt: Int64 = Time.now()\n    state shifted: Int64 = 0\n",
        "        Column {\n\
         \x20           Button(\"Next day\") { shifted = Time.addCalendarDays(timestamp: dueAt, days: 1) }\n\
         \x20       }\n",
    )
    .expect("calendar-day addition should compile");

    let calls = clock_calls(&module);
    assert!(calls.iter().any(|(method, ty, is_async)| {
        *method == TimeMethod::AddCalendarDays
            && *ty == Type::Numeric(NumericType::Int64)
            && !is_async
    }));

    let invalid = compile_with(
        "nexa-calendar-day-addition-type-error",
        "    state dueAt: Int64 = 0\n",
        "        Column { Text(Time.addCalendarDays(timestamp: dueAt, days: 1.5)) }\n",
    )
    .expect_err("calendar-day count must be an Int32");
    assert!(invalid.contains("Int32"));
}

#[test]
fn localized_date_and_time_formatters_are_typed_string_calls() {
    let module = compile_with(
        "nexa-localized-time-formatters",
        "    state timestamp: Int64 = Time.now()\n",
        r#"        Column {
            Text(Time.localizedDate(timestamp: timestamp))
            Text(Time.localizedTime(timestamp: timestamp))
            Text(Time.localizedDateTime(timestamp: timestamp))
        }
"#,
    )
    .expect("native-locale formatters should compile");

    let calls = clock_calls(&module);
    for method in [
        TimeMethod::LocalizedDate,
        TimeMethod::LocalizedTime,
        TimeMethod::LocalizedDateTime,
    ] {
        assert!(
            calls.iter().any(|(actual, ty, is_async)| *actual == method
                && *ty == Type::String
                && !is_async),
            "{method:?} should lower to a synchronous String call: {calls:?}"
        );
    }

    let invalid = compile_with(
        "nexa-localized-time-formatters-invalid-type",
        "    state timestamp: String = \"today\"\n",
        "        Column { Text(Time.localizedDate(timestamp: timestamp)) }\n",
    )
    .expect_err("localized formatters require epoch milliseconds");
    assert!(invalid.contains("Int64"));
}

#[test]
fn explicit_date_patterns_are_typed_calls_with_positional_arguments() {
    let module = compile_with(
        "nexa-explicit-date-format",
        "    state timestamp: Int64 = Time.now()\n",
        r#"        Column {
            Text(Time.format(timestamp, "d MMM, HH:mm"))
        }
"#,
    )
    .expect("explicit date patterns should compile using simple positional arguments");

    assert!(clock_calls(&module).iter().any(|(method, ty, is_async)| {
        *method == TimeMethod::Format && *ty == Type::String && !is_async
    }));
}

#[test]
fn log_methods_lower_to_typed_native_calls() {
    let module = compile_with(
        "nexa-core-log",
        "",
        r#"        Column {
            Button("Log") {
                Log.info(message: "ready")
                Log.warning(message: "slow")
                Log.error(message: "failed")
            }
        }
"#,
    )
    .expect("log methods should compile");

    let mut methods = Vec::new();
    let mut visit = |expr: &Expr| {
        if let Expr::LogCall { method, .. } = expr {
            methods.push(*method);
        }
    };
    for node in &module.body {
        nexa_ir::walk::walk_ir(std::slice::from_ref(node), &mut |_| {}, &mut visit);
    }
    for actions in [&module.on_appear, &module.on_disappear]
        .into_iter()
        .flatten()
    {
        nexa_ir::walk::walk_actions(actions, &mut visit);
    }
    assert_eq!(
        methods,
        [
            nexa_ir::LogMethod::Info,
            nexa_ir::LogMethod::Warning,
            nexa_ir::LogMethod::Error,
        ]
    );
}

#[test]
fn the_clock_capability_reaches_the_backends_only_when_the_app_reads_it() {
    let with_clock = compile_with(
        "nexa-core-clock-capability-on",
        "    state stamp: String = \"\"\n",
        "        Column {\n\
         \x20           Button(\"Now\") { stamp = Time.iso8601(timestamp: Time.now()) }\n\
         \x20       }\n",
    )
    .expect("an app that reads the clock should compile");
    assert!(
        nexa_ir::capabilities::analyze(&with_clock).uses_time,
        "reading the clock has to ask the backends for clock support"
    );

    let without_clock = compile_with(
        "nexa-core-clock-capability-off",
        "",
        "        Column {\n\
         \x20           Text(\"no clock here\")\n\
         \x20       }\n",
    )
    .expect("an app that never reads the clock should compile");
    assert!(
        !nexa_ir::capabilities::analyze(&without_clock).uses_time,
        "an app that never reads the clock must not carry clock support"
    );
}

#[test]
fn parsing_an_iso8601_string_reports_failure_as_an_optional() {
    let module = compile_with(
        "nexa-core-clock-parse",
        "    state stamp: Int64 = 0\n",
        "        Column {\n\
         \x20           Button(\"Parse\") {\n\
         \x20               stamp = Time.iso8601ToMillis(text: \"2026-09-27T09:41:02.123Z\") ?? 0\n\
         \x20           }\n\
         \x20       }\n",
    )
    .expect("parsing an ISO 8601 string should compile");

    assert_eq!(
        clock_calls(&module)
            .iter()
            .find(|(method, _, _)| *method == TimeMethod::Iso8601ToMillis)
            .map(|(_, ty, _)| ty.clone()),
        Some(Type::Optional(Box::new(Type::Numeric(NumericType::Int64)))),
        "malformed input is null, so parsing has to be observable as a failure"
    );
}

#[test]
fn clock_diagnostics_name_the_offending_call() {
    let int_state = "    state stamp: Int64 = 0\n";
    let string_state = "    state stamp: String = \"\"\n";
    let cases = [
        (
            "nexa-core-clock-arity",
            string_state,
            "        Column { Button(\"Extra\") { stamp = Time.iso8601(timestamp: Time.now(), format: \"long\") } }\n",
            "unknown option `format` for `Time.iso8601`",
        ),
        (
            "nexa-core-clock-argument-type",
            string_state,
            "        Column { Button(\"Wrong type\") { stamp = Time.iso8601(timestamp: \"not a number\") } }\n",
            "expected Int64, found String",
        ),
        (
            "nexa-core-clock-unknown-method",
            int_state,
            "        Column { Button(\"Unknown\") { stamp = Time.milliseconds() } }\n",
            "unknown native API",
        ),
        (
            "nexa-core-clock-sleep-not-awaited",
            int_state,
            "        OnAppear async { try { Time.sleep(milliseconds: 10) } catch { } }\n",
            "must be awaited",
        ),
        (
            "nexa-core-clock-awaiting-a-reading",
            int_state,
            "        OnAppear async { stamp = await Time.now() }\n",
            "native call `Time.now` is not async and cannot be awaited",
        ),
        (
            "nexa-core-clock-missing-argument",
            string_state,
            "        Column { Button(\"Missing\") { stamp = Time.iso8601() } }\n",
            "requires `timestamp`",
        ),
    ];
    for (name, states, body, expected) in cases {
        let error =
            compile_with(name, states, body).expect_err(&format!("{name} should be rejected"));
        assert!(
            error.contains(expected),
            "{name} should explain the mistake, got: {error}"
        );
    }
}
