use std::collections::BTreeSet;

use arcana_cabi::render_c_value_type_defs;

use crate::native_abi::{NativeAbiType, NativeExport};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeAbiRole {
    Param,
    Return,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeLayoutCatalog {
    pair_types: Vec<NativeAbiType>,
}

impl NativeLayoutCatalog {
    pub fn from_exports(exports: &[NativeExport]) -> Self {
        let mut pair_types = BTreeSet::new();
        for export in exports {
            for param in &export.params {
                collect_pair_types(&param.input_type, &mut pair_types);
            }
            collect_pair_types(&export.return_type, &mut pair_types);
        }
        Self {
            pair_types: pair_types.into_iter().collect(),
        }
    }

    pub fn rust_type_ref(&self, ty: &NativeAbiType, role: NativeAbiRole) -> String {
        let _ = self;
        match (ty, role) {
            (NativeAbiType::Int, _) => "i64".to_string(),
            (NativeAbiType::Bool, _) => "u8".to_string(),
            (NativeAbiType::Str, NativeAbiRole::Param) => "ArcanaStrView".to_string(),
            (NativeAbiType::Str, NativeAbiRole::Return) => "ArcanaOwnedStr".to_string(),
            (NativeAbiType::Bytes, NativeAbiRole::Param) => "ArcanaBytesView".to_string(),
            (NativeAbiType::Bytes, NativeAbiRole::Return) => "ArcanaOwnedBytes".to_string(),
            (NativeAbiType::Opaque(_), _) => "u64".to_string(),
            (NativeAbiType::Unit, _) => "()".to_string(),
            (NativeAbiType::Pair(_, _), _) => pair_struct_name(ty, role),
        }
    }

    pub fn rust_out_type_ref(&self, ty: &NativeAbiType) -> Option<String> {
        match ty {
            NativeAbiType::Unit => None,
            _ => Some(self.rust_type_ref(ty, NativeAbiRole::Return)),
        }
    }

    pub fn c_type_ref(&self, ty: &NativeAbiType, role: NativeAbiRole) -> String {
        let _ = self;
        match (ty, role) {
            (NativeAbiType::Int, _) => "int64_t".to_string(),
            (NativeAbiType::Bool, _) => "uint8_t".to_string(),
            (NativeAbiType::Str, NativeAbiRole::Param) => "ArcanaStrView".to_string(),
            (NativeAbiType::Str, NativeAbiRole::Return) => "ArcanaOwnedStr".to_string(),
            (NativeAbiType::Bytes, NativeAbiRole::Param) => "ArcanaBytesView".to_string(),
            (NativeAbiType::Bytes, NativeAbiRole::Return) => "ArcanaOwnedBytes".to_string(),
            (NativeAbiType::Opaque(_), _) => "uint64_t".to_string(),
            (NativeAbiType::Unit, _) => "void".to_string(),
            (NativeAbiType::Pair(_, _), _) => pair_struct_name(ty, role),
        }
    }

    pub fn c_out_type_ref(&self, ty: &NativeAbiType) -> Option<String> {
        match ty {
            NativeAbiType::Unit => None,
            _ => Some(self.c_type_ref(ty, NativeAbiRole::Return)),
        }
    }

    pub fn render_rust_pair_type_defs(&self) -> String {
        let mut out = String::new();
        for ty in &self.pair_types {
            out.push_str(&render_rust_pair_struct(ty, NativeAbiRole::Param));
            out.push_str(&render_rust_pair_struct(ty, NativeAbiRole::Return));
        }
        out
    }

    pub fn render_c_type_defs(&self) -> String {
        let mut out = render_c_value_type_defs();
        out.push_str(&self.render_c_pair_type_defs());
        out
    }

    pub fn render_c_pair_type_defs(&self) -> String {
        let mut out = String::new();
        for ty in &self.pair_types {
            out.push_str(&render_c_pair_struct(ty, NativeAbiRole::Param));
            out.push_str(&render_c_pair_struct(ty, NativeAbiRole::Return));
        }
        out
    }
}

fn collect_pair_types(ty: &NativeAbiType, out: &mut BTreeSet<NativeAbiType>) {
    if let NativeAbiType::Pair(left, right) = ty {
        collect_pair_types(left, out);
        collect_pair_types(right, out);
        out.insert(ty.clone());
    }
}

fn render_rust_pair_struct(ty: &NativeAbiType, role: NativeAbiRole) -> String {
    let name = pair_struct_name(ty, role);
    let (left_ty, right_ty) = pair_fields(ty);
    format!(
        concat!(
            "#[repr(C)]\n",
            "#[derive(Clone, Copy, Default)]\n",
            "pub struct {} {{\n",
            "    pub left: {},\n",
            "    pub right: {},\n",
            "}}\n\n"
        ),
        name,
        rust_field_type(left_ty, role),
        rust_field_type(right_ty, role),
    )
}

fn render_c_pair_struct(ty: &NativeAbiType, role: NativeAbiRole) -> String {
    let name = pair_struct_name(ty, role);
    let (left_ty, right_ty) = pair_fields(ty);
    format!(
        concat!(
            "typedef struct {} {{\n",
            "    {} left;\n",
            "    {} right;\n",
            "}} {};\n\n"
        ),
        name,
        c_field_type(left_ty, role),
        c_field_type(right_ty, role),
        name,
    )
}

fn rust_field_type(ty: &NativeAbiType, role: NativeAbiRole) -> String {
    match (ty, role) {
        (NativeAbiType::Int, _) => "i64".to_string(),
        (NativeAbiType::Bool, _) => "u8".to_string(),
        (NativeAbiType::Str, NativeAbiRole::Param) => "ArcanaStrView".to_string(),
        (NativeAbiType::Str, NativeAbiRole::Return) => "ArcanaOwnedStr".to_string(),
        (NativeAbiType::Bytes, NativeAbiRole::Param) => "ArcanaBytesView".to_string(),
        (NativeAbiType::Bytes, NativeAbiRole::Return) => "ArcanaOwnedBytes".to_string(),
        (NativeAbiType::Opaque(_), _) => "u64".to_string(),
        (NativeAbiType::Unit, _) => "()".to_string(),
        (NativeAbiType::Pair(_, _), _) => pair_struct_name(ty, role),
    }
}

fn c_field_type(ty: &NativeAbiType, role: NativeAbiRole) -> String {
    match (ty, role) {
        (NativeAbiType::Int, _) => "int64_t".to_string(),
        (NativeAbiType::Bool, _) => "uint8_t".to_string(),
        (NativeAbiType::Str, NativeAbiRole::Param) => "ArcanaStrView".to_string(),
        (NativeAbiType::Str, NativeAbiRole::Return) => "ArcanaOwnedStr".to_string(),
        (NativeAbiType::Bytes, NativeAbiRole::Param) => "ArcanaBytesView".to_string(),
        (NativeAbiType::Bytes, NativeAbiRole::Return) => "ArcanaOwnedBytes".to_string(),
        (NativeAbiType::Opaque(_), _) => "uint64_t".to_string(),
        (NativeAbiType::Unit, _) => "void".to_string(),
        (NativeAbiType::Pair(_, _), _) => pair_struct_name(ty, role),
    }
}

fn pair_struct_name(ty: &NativeAbiType, role: NativeAbiRole) -> String {
    let prefix = match role {
        NativeAbiRole::Param => "ArcanaPairView",
        NativeAbiRole::Return => "ArcanaPairOwned",
    };
    format!("{prefix}__{}", mangle_type_name(ty))
}

fn mangle_type_name(ty: &NativeAbiType) -> String {
    match ty {
        NativeAbiType::Int => "Int".to_string(),
        NativeAbiType::Bool => "Bool".to_string(),
        NativeAbiType::Str => "Str".to_string(),
        NativeAbiType::Bytes => "Bytes".to_string(),
        NativeAbiType::Opaque(name) => format!(
            "Opaque__{}",
            name.chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect::<String>()
        ),
        NativeAbiType::Unit => "Unit".to_string(),
        NativeAbiType::Pair(left, right) => {
            format!(
                "Pair__{}__{}",
                mangle_type_name(left),
                mangle_type_name(right)
            )
        }
    }
}

fn pair_fields(ty: &NativeAbiType) -> (&NativeAbiType, &NativeAbiType) {
    match ty {
        NativeAbiType::Pair(left, right) => (left, right),
        _ => panic!("pair_fields requires a pair type"),
    }
}
