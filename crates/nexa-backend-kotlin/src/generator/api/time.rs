//! The core clock, on Kotlin.
//!
//! A wall clock and a monotonic clock are separate on purpose: `now()` can jump
//! when the system time is corrected, `monotonic()` cannot, so only the latter
//! measures a duration.
//!
//! ISO 8601 is formatted and parsed by hand rather than through `java.time` or
//! `SimpleDateFormat`. `java.time` needs API 26 or library desugaring, and
//! `SimpleDateFormat` carries locale and calendar settings that would make the
//! output differ between devices. A hand-rolled civil-date conversion is a fixed
//! layout - `yyyy-MM-ddTHH:mm:ss.SSSZ` in UTC - identical to the Swift side, so
//! a timestamp written on one platform reads on the other.

use nexa_codegen::SourceWriter;

/// Emits the clock helpers an app needs to read, format, parse, and sleep.
pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(TIME_RUNTIME);
}

const TIME_RUNTIME: &str = r#"// Core clock.

/** Days from the Unix epoch to the civil date 0000-03-01, which is where the civil-date conversion counts from. */
private const val NEXA_EPOCH_DAYS = 719_468L
private const val NEXA_DAYS_IN_400_YEARS = 146_097L

/** The UTC instant of [milliseconds] since the Unix epoch, as `yyyy-MM-ddTHH:mm:ss.SSSZ`. */
internal fun nexaIso8601(milliseconds: Long): String {
    var days = Math.floorDiv(milliseconds, 86_400_000L)
    var remainder = Math.floorMod(milliseconds, 86_400_000L)
    if (remainder < 0) {
        remainder += 86_400_000L
        days -= 1
    }
    val (year, month, day) = nexaCivilDate(days)
    val hour = remainder / 3_600_000L
    remainder %= 3_600_000L
    val minute = remainder / 60_000L
    remainder %= 60_000L
    val second = remainder / 1000L
    val millisecond = remainder % 1000L
    return String.format(
        java.util.Locale.ROOT,
        "%04d-%02d-%02dT%02d:%02d:%02d.%03dZ",
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
    )
}

/** The inverse of [nexaIso8601], or null when the text is not that layout. */
internal fun nexaIso8601ToMillis(text: String): Long? {
    // The shortest accepted form is `yyyy-MM-ddTHH:mm:ssZ`; the fractional part
    // is optional, and the trailing `Z` is not, because the layout is UTC.
    if (text.length != 20 && text.length != 24) {
        return null
    }
    if (text[4] != '-' || text[7] != '-' || text[10] != 'T' ||
        text[13] != ':' || text[16] != ':' || text[text.length - 1] != 'Z'
    ) {
        return null
    }
    if (text.length == 24 && text[19] != '.') {
        return null
    }
    // Digit runs in layout order. A run is read left to right, so its first
    // digit is the most significant one.
    val runs = listOf(0..3, 5..6, 8..9, 11..12, 14..15, 17..18)
    val values = LongArray(runs.size + 1)
    for (index in runs.indices) {
        val value = nexaDigits(text, runs[index]) ?: return null
        values[index] = value
    }
    if (text.length == 24) {
        values[6] = nexaDigits(text, 20..22) ?: return null
    }
    val year = values[0]
    val month = values[1]
    val day = values[2]
    val hour = values[3]
    val minute = values[4]
    val second = values[5]
    if (month < 1 || month > 12 || day < 1) {
        return null
    }
    if (day > nexaMonthLength(year, month)) {
        return null
    }
    // A leap second is a real reading of a real clock, so 60 is accepted.
    if (hour >= 24 || minute >= 60 || second > 60) {
        return null
    }
    val millisecond = if (values.size > 6) values[6] else 0L
    if (millisecond >= 1000) {
        return null
    }
    val days = nexaDaysFromCivil(year, month, day)
    return (((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + millisecond
}

/**
 * The number of days in a proleptic Gregorian month. February is the only one that
 * depends on the year, and 31 February has to be rejected rather than rolled
 * forward into March.
 */
fun nexaMonthLength(year: Long, month: Long): Long {
    if (month == 2L) {
        val leap = year % 4L == 0L && (year % 100L != 0L || year % 400L == 0L)
        return if (leap) 29L else 28L
    }
    if (month == 4L || month == 6L || month == 9L || month == 11L) {
        return 30L
    }
    return 31L
}

/** A run of ASCII digits as a number, or null when anything else is in it. */
private fun nexaDigits(text: String, range: IntRange): Long? {
    var value = 0L
    for (index in range) {
        val digit = text[index]
        if (digit < '0' || digit > '9') {
            return null
        }
        value = value * 10 + (digit - '0').toLong()
    }
    return value
}

/** Days since the Unix epoch to a proleptic Gregorian date. */
private fun nexaCivilDate(daysSinceEpoch: Long): Triple<Long, Long, Long> {
    val shifted = daysSinceEpoch + NEXA_EPOCH_DAYS
    val era = if (shifted >= 0) shifted / NEXA_DAYS_IN_400_YEARS
    else (shifted - (NEXA_DAYS_IN_400_YEARS - 1)) / NEXA_DAYS_IN_400_YEARS
    val dayOfEra = shifted - era * NEXA_DAYS_IN_400_YEARS
    val yearOfEra = (dayOfEra - dayOfEra / 1460 + dayOfEra / 36524 - dayOfEra / 146096) / 365
    val year = yearOfEra + era * 400
    val dayOfYear = dayOfEra - (365 * yearOfEra + yearOfEra / 4 - yearOfEra / 100)
    val monthIndex = (5 * dayOfYear + 2) / 153
    val day = dayOfYear - (153 * monthIndex + 2) / 5 + 1
    val month = monthIndex + if (monthIndex < 10) 3 else -9
    return Triple(if (month <= 2) year + 1 else year, month, day)
}

/** The inverse of [nexaCivilDate]. */
private fun nexaDaysFromCivil(year: Long, month: Long, day: Long): Long {
    val adjustedYear = year - if (month <= 2) 1 else 0
    val era = if (adjustedYear >= 0) adjustedYear / 400 else (adjustedYear - 399) / 400
    val yearOfEra = adjustedYear - era * 400
    val monthIndex = if (month > 2) month - 3 else month + 9
    val dayOfYear = (153 * monthIndex + 2) / 5 + day - 1
    val dayOfEra = yearOfEra * 365 + yearOfEra / 4 - yearOfEra / 100 + dayOfYear
    return era * NEXA_DAYS_IN_400_YEARS + dayOfEra - NEXA_EPOCH_DAYS
}
"#;
