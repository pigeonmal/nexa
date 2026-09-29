//! Locale-aware number formatting on Swift Foundation.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.facts.capabilities.uses_number_formatting,
        "Foundation",
    );
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// MARK: - Currency formatting

func nexaFormatCurrency(_ amount: Double, _ currencyCode: String) -> String {
    let formatter = NumberFormatter()
    formatter.locale = .current
    let code = currencyCode.uppercased()
    if Locale.commonISOCurrencyCodes.contains(code) {
        formatter.currencyCode = code
        formatter.numberStyle = .currency
    } else {
        formatter.numberStyle = .decimal
    }
    return formatter.string(from: NSNumber(value: amount)) ?? String(amount)
}
"#,
    );
}

#[cfg(test)]
mod tests {
    use super::render;
    use nexa_codegen::SourceWriter;

    #[test]
    fn currency_formatter_uses_current_locale_and_validates_the_currency_code() {
        let mut out = SourceWriter::default();
        render(&mut out);
        let source = out.finish();
        assert!(source.contains("formatter.locale = .current"));
        assert!(source.contains("Locale.commonISOCurrencyCodes.contains(code)"));
        assert!(source.contains("formatter.numberStyle = .currency"));
        assert!(source.contains("formatter.numberStyle = .decimal"));
    }
}
