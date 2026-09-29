//! Locale-aware number formatting on Android's Java number APIs.

use nexa_codegen::SourceWriter;

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Currency formatting.

internal fun nexaFormatCurrency(amount: Double, currencyCode: String): String {
    val locale = java.util.Locale.getDefault()
    val currency = try {
        java.util.Currency.getInstance(currencyCode.uppercase(java.util.Locale.ROOT))
    } catch (_: IllegalArgumentException) {
        return java.text.NumberFormat.getNumberInstance(locale).format(amount)
    }
    val formatter = java.text.NumberFormat.getCurrencyInstance(locale)
    formatter.currency = currency
    return formatter.format(amount)
}
"#,
    );
}

#[cfg(test)]
mod tests {
    use super::render;
    use nexa_codegen::SourceWriter;

    #[test]
    fn currency_formatter_uses_the_default_locale_and_handles_invalid_codes() {
        let mut out = SourceWriter::default();
        render(&mut out);
        let source = out.finish();
        assert!(source.contains("java.util.Locale.getDefault()"));
        assert!(source.contains("java.util.Currency.getInstance"));
        assert!(source.contains("NumberFormat.getNumberInstance(locale)"));
    }
}
