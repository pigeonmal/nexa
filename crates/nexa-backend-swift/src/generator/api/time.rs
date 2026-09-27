//! The core clock, on Swift.
//!
//! A wall clock and a monotonic clock are separate on purpose: `now()` can jump
//! when the system time is corrected, `monotonic()` cannot, so only the latter
//! measures a duration.
//!
//! ISO 8601 is rendered and parsed by hand rather than through a formatter or a
//! date style. A formatter carries locale and calendar settings that would make
//! the output differ between platforms and between devices, and a hand-rolled
//! civil-date conversion is a fixed, testable layout: `yyyy-MM-ddTHH:mm:ss.SSSZ`
//! in UTC. Both platforms implement the same layout and the same leap-year
//! rules, so a timestamp written on one reads on the other.

use nexa_codegen::SourceWriter;

use crate::generator::engine::imports::ImportSet;
use crate::generator::engine::features::Features;

/// Declares what the clock needs: `Date` for the wall clock, `Dispatch` for the
/// monotonic counter, and `String(format:)` for the ISO 8601 layout.
pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    if !features.facts.capabilities.uses_time {
        return;
    }
    imports.add(true, "Foundation");
    imports.add(true, "Dispatch");
}

/// Emits the clock helpers an app needs to read, format, parse, and sleep.
pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(TIME_RUNTIME);
}

const TIME_RUNTIME: &str = r#"// MARK: - Core clock

/// Days from the Unix epoch to the civil date 0000-03-01, which is where the
/// civil-date conversion counts from.
private let nexaEpochDays: Int64 = 719_468
private let nexaDaysIn400Years: Int64 = 146_097

/// The UTC instant of `milliseconds` since the Unix epoch, as
/// `yyyy-MM-ddTHH:mm:ss.SSSZ`.
func nexaIso8601(_ milliseconds: Int64) -> String {
    var days = milliseconds / 86_400_000
    var remainder = milliseconds % 86_400_000
    if remainder < 0 {
        remainder += 86_400_000
        days -= 1
    }
    let (year, month, day) = nexaCivilDate(days)
    let hour = remainder / 3_600_000
    remainder %= 3_600_000
    let minute = remainder / 60_000
    remainder %= 60_000
    let second = remainder / 1000
    let millisecond = remainder % 1000
    return String(
        format: "%04d-%02d-%02dT%02d:%02d:%02d.%03dZ",
        year, month, day, hour, minute, second, millisecond
    )
}

/// The inverse of `nexaIso8601`, or nil when the text is not that layout.
func nexaIso8601ToMillis(_ text: String) -> Int64? {
    let scalars = Array(text.utf8)
    // The shortest accepted form is `yyyy-MM-ddTHH:mm:ssZ`; the fractional part
    // is optional, and the trailing `Z` is not, because the layout is UTC.
    guard scalars.count == 20 || scalars.count == 24,
        scalars[4] == UInt8(ascii: "-"),
        scalars[7] == UInt8(ascii: "-"),
        scalars[10] == UInt8(ascii: "T"),
        scalars[13] == UInt8(ascii: ":"),
        scalars[16] == UInt8(ascii: ":"),
        scalars[scalars.count - 1] == UInt8(ascii: "Z")
    else {
        return nil
    }
    if scalars.count == 24 {
        guard scalars[19] == UInt8(ascii: ".") else { return nil }
    }
    // Digit runs in layout order. A run is read left to right, so its first
    // digit is the most significant one.
    let runs = [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)]
    var values: [Int64] = []
    values.reserveCapacity(runs.count + 1)
    for (start, end) in runs {
        guard let value = nexaDigits(scalars[start..<end]) else { return nil }
        values.append(value)
    }
    if scalars.count == 24 {
        guard let millisecond = nexaDigits(scalars[20..<23]) else { return nil }
        values.append(millisecond)
    }
    let (year, month, day) = (values[0], values[1], values[2])
    let (hour, minute, second) = (values[3], values[4], values[5])
    guard (1...12).contains(month), day >= 1 else { return nil }
    guard day <= nexaMonthLength(year: year, month: month) else { return nil }
    // A leap second is a real reading of a real clock, so 60 is accepted.
    guard hour < 24, minute < 60, second <= 60 else { return nil }
    let millisecond = values.count > 6 ? values[6] : 0
    guard millisecond < 1000 else { return nil }
    let days = nexaDaysFromCivil(year: year, month: month, day: day)
    return (((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + millisecond
}

/// The number of days in a proleptic Gregorian month. February is the only one
/// that depends on the year, and 31 February has to be rejected rather than
/// rolled forward into March.
func nexaMonthLength(year: Int64, month: Int64) -> Int64 {
    if month == 2 {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
        return leap ? 29 : 28
    }
    if month == 4 || month == 6 || month == 9 || month == 11 {
        return 30
    }
    return 31
}

/// A run of ASCII digits as a number, or nil when anything else is in it.
private func nexaDigits(_ digits: ArraySlice<UInt8>) -> Int64? {
    var value: Int64 = 0
    for digit in digits {
        guard digit >= UInt8(ascii: "0"), digit <= UInt8(ascii: "9") else { return nil }
        value = value * 10 + Int64(digit - UInt8(ascii: "0"))
    }
    return value
}

/// Days since the Unix epoch to a proleptic Gregorian date.
private func nexaCivilDate(_ daysSinceEpoch: Int64) -> (Int64, Int64, Int64) {
    let shifted = daysSinceEpoch + nexaEpochDays
    let era = (shifted >= 0 ? shifted : shifted - (nexaDaysIn400Years - 1)) / nexaDaysIn400Years
    let dayOfEra = shifted - era * nexaDaysIn400Years
    let yearOfEra = (dayOfEra - dayOfEra / 1460 + dayOfEra / 36_524 - dayOfEra / 146_096) / 365
    let year = yearOfEra + era * 400
    let dayOfYear = dayOfEra - (365 * yearOfEra + yearOfEra / 4 - yearOfEra / 100)
    let monthIndex = (5 * dayOfYear + 2) / 153
    let day = dayOfYear - (153 * monthIndex + 2) / 5 + 1
    let month = monthIndex + (monthIndex < 10 ? 3 : -9)
    return (month <= 2 ? year + 1 : year, month, day)
}

/// The inverse of `nexaCivilDate`.
private func nexaDaysFromCivil(year: Int64, month: Int64, day: Int64) -> Int64 {
    let adjustedYear = year - (month <= 2 ? 1 : 0)
    let era = (adjustedYear >= 0 ? adjustedYear : adjustedYear - 399) / 400
    let yearOfEra = adjustedYear - era * 400
    let monthIndex = month > 2 ? month - 3 : month + 9
    let dayOfYear = (153 * monthIndex + 2) / 5 + day - 1
    let dayOfEra = yearOfEra * 365 + yearOfEra / 4 - yearOfEra / 100 + dayOfYear
    return era * nexaDaysIn400Years + dayOfEra - nexaEpochDays
}
"#;
