use arcana_hir::{
    HirLifetime, HirPath, HirProjection, HirTraitRef, HirType, HirTypeBindingScope, HirTypeKind,
    HirTypeSubstitutions, hir_type_matches, parse_hir_type,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutineProvenance {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutinePath {
    pub segments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutineLifetime {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutineTraitRef {
    pub path: IrRoutinePath,
    pub args: Vec<IrRoutineType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutineProjection {
    pub trait_ref: IrRoutineTraitRef,
    pub assoc: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutineType {
    pub kind: IrRoutineTypeKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrRoutineTypeKind {
    Path(IrRoutinePath),
    Apply {
        base: IrRoutinePath,
        args: Vec<IrRoutineType>,
    },
    Ref {
        lifetime: Option<IrRoutineLifetime>,
        mutable: bool,
        inner: Box<IrRoutineType>,
    },
    Tuple(Vec<IrRoutineType>),
    Projection(IrRoutineProjection),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrRoutineParam {
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub binding_id: u64,
    pub mode: Option<String>,
    pub name: String,
    pub ty: IrRoutineType,
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

impl IrRoutinePath {
    pub fn render(&self) -> String {
        self.segments.join(".")
    }

    pub fn root_name(&self) -> Option<&str> {
        self.segments.last().map(String::as_str).map(|segment| {
            segment
                .split_once('<')
                .map(|(head, _)| head)
                .unwrap_or(segment)
        })
    }

    pub fn is_well_formed(&self) -> bool {
        !self.segments.is_empty() && self.segments.iter().all(|segment| !segment.is_empty())
    }
}

impl IrRoutineLifetime {
    pub fn render(&self) -> String {
        self.name.clone()
    }

    fn is_well_formed(&self) -> bool {
        !self.name.is_empty()
    }
}

impl IrRoutineTraitRef {
    pub fn render(&self) -> String {
        if self.args.is_empty() {
            self.path.render()
        } else {
            format!(
                "{}[{}]",
                self.path.render(),
                self.args
                    .iter()
                    .map(IrRoutineType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn is_well_formed(&self) -> bool {
        self.path.is_well_formed() && self.args.iter().all(IrRoutineType::is_well_formed)
    }
}

impl IrRoutineProjection {
    pub fn render(&self) -> String {
        format!("{}.{}", self.trait_ref.render(), self.assoc)
    }

    fn is_well_formed(&self) -> bool {
        !self.assoc.is_empty() && self.trait_ref.is_well_formed()
    }
}

impl IrRoutineType {
    pub fn render(&self) -> String {
        match &self.kind {
            IrRoutineTypeKind::Path(path) => path.render(),
            IrRoutineTypeKind::Apply { base, args } => format!(
                "{}[{}]",
                base.render(),
                args.iter()
                    .map(IrRoutineType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            IrRoutineTypeKind::Ref {
                lifetime,
                mutable,
                inner,
            } => {
                let mut rendered = String::from("&");
                if let Some(lifetime) = lifetime {
                    rendered.push_str(&lifetime.render());
                    rendered.push(' ');
                }
                if *mutable {
                    rendered.push_str("mut ");
                }
                rendered.push_str(&inner.render());
                rendered
            }
            IrRoutineTypeKind::Tuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(IrRoutineType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            IrRoutineTypeKind::Projection(projection) => projection.render(),
        }
    }

    pub fn from_hir(ty: &HirType) -> Self {
        Self {
            kind: match &ty.kind {
                HirTypeKind::Path(path) => IrRoutineTypeKind::Path(IrRoutinePath {
                    segments: path.segments.clone(),
                }),
                HirTypeKind::Apply { base, args } => IrRoutineTypeKind::Apply {
                    base: IrRoutinePath {
                        segments: base.segments.clone(),
                    },
                    args: args.iter().map(Self::from_hir).collect(),
                },
                HirTypeKind::Ref {
                    lifetime,
                    mutable,
                    inner,
                } => IrRoutineTypeKind::Ref {
                    lifetime: lifetime.as_ref().map(|lifetime| IrRoutineLifetime {
                        name: lifetime.name.clone(),
                    }),
                    mutable: *mutable,
                    inner: Box::new(Self::from_hir(inner)),
                },
                HirTypeKind::Tuple(items) => {
                    IrRoutineTypeKind::Tuple(items.iter().map(Self::from_hir).collect())
                }
                HirTypeKind::Projection(projection) => {
                    IrRoutineTypeKind::Projection(IrRoutineProjection {
                        trait_ref: IrRoutineTraitRef {
                            path: IrRoutinePath {
                                segments: projection.trait_ref.path.segments.clone(),
                            },
                            args: projection
                                .trait_ref
                                .args
                                .iter()
                                .map(Self::from_hir)
                                .collect(),
                        },
                        assoc: projection.assoc.clone(),
                    })
                }
            },
        }
    }

    pub fn to_hir(&self) -> HirType {
        HirType {
            kind: self.to_hir_kind(),
            span: Default::default(),
        }
    }

    pub fn base_path(&self) -> Option<&IrRoutinePath> {
        match &self.kind {
            IrRoutineTypeKind::Path(path) | IrRoutineTypeKind::Apply { base: path, .. } => {
                Some(path)
            }
            IrRoutineTypeKind::Projection(projection) => Some(&projection.trait_ref.path),
            IrRoutineTypeKind::Ref { inner, .. } => inner.base_path(),
            IrRoutineTypeKind::Tuple(_) => None,
        }
    }

    pub fn root_name(&self) -> Option<&str> {
        self.base_path().and_then(IrRoutinePath::root_name)
    }

    pub fn is_well_formed(&self) -> bool {
        match &self.kind {
            IrRoutineTypeKind::Path(path) => path.is_well_formed(),
            IrRoutineTypeKind::Apply { base, args } => {
                base.is_well_formed() && args.iter().all(IrRoutineType::is_well_formed)
            }
            IrRoutineTypeKind::Ref {
                lifetime, inner, ..
            } => {
                lifetime
                    .as_ref()
                    .is_none_or(IrRoutineLifetime::is_well_formed)
                    && inner.is_well_formed()
            }
            IrRoutineTypeKind::Tuple(items) => items.iter().all(IrRoutineType::is_well_formed),
            IrRoutineTypeKind::Projection(projection) => projection.is_well_formed(),
        }
    }

    pub fn matches_declared(
        declared: &IrRoutineType,
        actual: &IrRoutineType,
        type_params: &[String],
    ) -> bool {
        let bindings = HirTypeBindingScope::from_names(type_params.iter().cloned());
        let mut substitutions = HirTypeSubstitutions::default();
        hir_type_matches(
            &declared.to_hir(),
            &actual.to_hir(),
            &bindings,
            &mut substitutions,
        )
    }

    fn to_hir_kind(&self) -> HirTypeKind {
        match &self.kind {
            IrRoutineTypeKind::Path(path) => HirTypeKind::Path(HirPath {
                segments: path.segments.clone(),
                span: Default::default(),
            }),
            IrRoutineTypeKind::Apply { base, args } => HirTypeKind::Apply {
                base: HirPath {
                    segments: base.segments.clone(),
                    span: Default::default(),
                },
                args: args.iter().map(IrRoutineType::to_hir).collect(),
            },
            IrRoutineTypeKind::Ref {
                lifetime,
                mutable,
                inner,
            } => HirTypeKind::Ref {
                lifetime: lifetime.as_ref().map(|lifetime| HirLifetime {
                    name: lifetime.name.clone(),
                    span: Default::default(),
                }),
                mutable: *mutable,
                inner: Box::new(inner.to_hir()),
            },
            IrRoutineTypeKind::Tuple(items) => {
                HirTypeKind::Tuple(items.iter().map(IrRoutineType::to_hir).collect())
            }
            IrRoutineTypeKind::Projection(projection) => HirTypeKind::Projection(HirProjection {
                trait_ref: HirTraitRef {
                    path: HirPath {
                        segments: projection.trait_ref.path.segments.clone(),
                        span: Default::default(),
                    },
                    args: projection
                        .trait_ref
                        .args
                        .iter()
                        .map(IrRoutineType::to_hir)
                        .collect(),
                    span: Default::default(),
                },
                assoc: projection.assoc.clone(),
                span: Default::default(),
            }),
        }
    }
}

pub fn parse_routine_type_text(text: &str) -> Result<IrRoutineType, String> {
    parse_hir_type(text).map(|ty| IrRoutineType::from_hir(&ty))
}

pub fn render_routine_signature_text(
    symbol_kind: &str,
    symbol_name: &str,
    is_async: bool,
    type_params: &[String],
    params: &[IrRoutineParam],
    return_type: Option<&IrRoutineType>,
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
                piece.push_str(&param.ty.render());
                piece
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    rendered.push(')');
    if let Some(return_type) = return_type {
        rendered.push_str(" -> ");
        rendered.push_str(&return_type.render());
    }
    rendered.push(':');
    rendered
}

#[cfg(test)]
mod tests {
    use super::{IrRoutineType, parse_routine_type_text};

    #[test]
    fn routine_types_roundtrip_hir_shape() {
        let ty = parse_routine_type_text("&mut Pair[Int, std.option.Option[Bool]]")
            .expect("type should parse");
        assert_eq!(ty.render(), "&mut Pair[Int, std.option.Option[Bool]]");
        assert_eq!(IrRoutineType::from_hir(&ty.to_hir()), ty);
    }

    #[test]
    fn routine_type_matching_honors_generic_placeholders() {
        let declared = parse_routine_type_text("std.concurrent.Channel[T]")
            .expect("declared type should parse");
        let actual = parse_routine_type_text("std.concurrent.Channel[Bool]")
            .expect("actual type should parse");
        assert!(IrRoutineType::matches_declared(
            &declared,
            &actual,
            &["T".to_string()]
        ));
    }
}
