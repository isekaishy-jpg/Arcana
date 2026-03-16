use std::collections::BTreeSet;

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
                collect_pair_types(&param.ty, &mut pair_types);
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

    pub fn render_rust_type_defs(&self) -> String {
        let mut out = String::from(concat!(
            "#[repr(C)]\n",
            "#[derive(Clone, Copy, Default)]\n",
            "pub struct ArcanaBytesView {\n",
            "    pub ptr: *const u8,\n",
            "    pub len: usize,\n",
            "}\n\n",
            "#[repr(C)]\n",
            "#[derive(Clone, Copy, Default)]\n",
            "pub struct ArcanaStrView {\n",
            "    pub ptr: *const u8,\n",
            "    pub len: usize,\n",
            "}\n\n",
            "#[repr(C)]\n",
            "#[derive(Clone, Copy, Default)]\n",
            "pub struct ArcanaOwnedBytes {\n",
            "    pub ptr: *mut u8,\n",
            "    pub len: usize,\n",
            "}\n\n",
            "#[repr(C)]\n",
            "#[derive(Clone, Copy, Default)]\n",
            "pub struct ArcanaOwnedStr {\n",
            "    pub ptr: *mut u8,\n",
            "    pub len: usize,\n",
            "}\n\n",
        ));
        for ty in &self.pair_types {
            out.push_str(&render_rust_pair_struct(ty, NativeAbiRole::Param));
            out.push_str(&render_rust_pair_struct(ty, NativeAbiRole::Return));
        }
        out
    }

    pub fn render_c_type_defs(&self) -> String {
        let mut out = String::from(concat!(
            "typedef struct ArcanaBytesView {\n",
            "    const uint8_t* ptr;\n",
            "    size_t len;\n",
            "} ArcanaBytesView;\n\n",
            "typedef struct ArcanaStrView {\n",
            "    const uint8_t* ptr;\n",
            "    size_t len;\n",
            "} ArcanaStrView;\n\n",
            "typedef struct ArcanaOwnedBytes {\n",
            "    uint8_t* ptr;\n",
            "    size_t len;\n",
            "} ArcanaOwnedBytes;\n\n",
            "typedef struct ArcanaOwnedStr {\n",
            "    uint8_t* ptr;\n",
            "    size_t len;\n",
            "} ArcanaOwnedStr;\n\n",
        ));
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
