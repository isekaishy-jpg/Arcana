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
    let rendered = symbol.return_type.as_ref().map(|ty| ty.render());
    validate_runtime_main_entry_contract(symbol.params.len(), rendered.as_deref())
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

#[cfg(test)]
mod tests {
    use super::validate_runtime_main_entry_contract;

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
}
