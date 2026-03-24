#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrRoutineParam {
    pub mode: Option<String>,
    pub name: String,
    pub ty: String,
}

pub fn render_routine_signature_text(
    symbol_kind: &str,
    symbol_name: &str,
    is_async: bool,
    type_params: &[String],
    params: &[IrRoutineParam],
    return_type: Option<&str>,
) -> String {
    let mut rendered = String::new();
    if is_async {
        rendered.push_str("async ");
    }
    if symbol_kind == "system" {
        rendered.push_str("system ");
    } else {
        rendered.push_str("fn ");
    }
    rendered.push_str(symbol_name);
    if !type_params.is_empty() {
        rendered.push('[');
        rendered.push_str(&type_params.join(", "));
        rendered.push(']');
    }
    rendered.push('(');
    rendered.push_str(
        &params
            .iter()
            .map(|param| {
                let mut piece = String::new();
                if let Some(mode) = &param.mode {
                    piece.push_str(mode);
                    piece.push(' ');
                }
                piece.push_str(&param.name);
                piece.push_str(": ");
                piece.push_str(&param.ty);
                piece
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    rendered.push(')');
    if let Some(return_type) = return_type {
        rendered.push_str(" -> ");
        rendered.push_str(return_type);
    }
    rendered.push(':');
    rendered
}
