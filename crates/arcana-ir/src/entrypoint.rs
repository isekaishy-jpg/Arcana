use arcana_hir::{HirSymbol, HirSymbolKind};

pub const RUNTIME_MAIN_ENTRYPOINT_NAME: &str = "main";

pub fn is_runtime_main_entry_symbol(
    package_name: &str,
    module_id: &str,
    symbol: &HirSymbol,
) -> bool {
    symbol.kind == HirSymbolKind::Fn
        && module_id == package_name
        && symbol.name == RUNTIME_MAIN_ENTRYPOINT_NAME
}

pub fn validate_runtime_main_entry_symbol(symbol: &HirSymbol) -> Result<(), String> {
    validate_runtime_main_entry_contract(symbol.params.len(), symbol.return_type.as_deref())
}

pub fn validate_runtime_main_entry_contract(
    param_count: usize,
    return_type: Option<&str>,
) -> Result<(), String> {
    if param_count != 0 {
        return Err("main must not take parameters in the current runtime lane".to_string());
    }
    if !matches!(
        return_type.map(str::trim),
        None | Some("Int") | Some("Unit")
    ) {
        return Err("main must return Int or Unit in the current runtime lane".to_string());
    }
    Ok(())
}

pub fn runtime_main_return_type_from_signature(signature_row: &str) -> Option<&str> {
    let (_, tail) = signature_row.rsplit_once("->")?;
    let tail = tail.trim();
    let tail = tail.strip_suffix(':').unwrap_or(tail).trim();
    (!tail.is_empty()).then_some(tail)
}

#[cfg(test)]
mod tests {
    use super::{runtime_main_return_type_from_signature, validate_runtime_main_entry_contract};

    #[test]
    fn runtime_main_contract_rejects_parameters() {
        let err = validate_runtime_main_entry_contract(1, Some("Int"))
            .expect_err("parameterized main should be rejected");
        assert_eq!(
            err,
            "main must not take parameters in the current runtime lane"
        );
    }

    #[test]
    fn runtime_main_contract_rejects_non_runtime_return_type() {
        let err = validate_runtime_main_entry_contract(0, Some("Bool"))
            .expect_err("non-runtime return should be rejected");
        assert_eq!(
            err,
            "main must return Int or Unit in the current runtime lane"
        );
    }

    #[test]
    fn runtime_main_return_type_parser_handles_unit_and_int_signatures() {
        assert_eq!(
            runtime_main_return_type_from_signature("fn main() -> Int:"),
            Some("Int")
        );
        assert_eq!(
            runtime_main_return_type_from_signature("fn main() -> Unit:"),
            Some("Unit")
        );
        assert_eq!(runtime_main_return_type_from_signature("fn main():"), None);
    }
}
