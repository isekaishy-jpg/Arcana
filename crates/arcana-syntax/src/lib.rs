pub mod freeze;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectiveKind {
    Import,
    Use,
    Reexport,
}

impl DirectiveKind {
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Use => "use",
            Self::Reexport => "reexport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDirective {
    pub kind: DirectiveKind,
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,
    Record,
    Enum,
    Trait,
    Behavior,
    Const,
}

impl SymbolKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::Record => "record",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Behavior => "behavior",
            Self::Const => "const",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamMode {
    Read,
    Edit,
    Take,
}

impl ParamMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Take => "take",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    pub mode: Option<ParamMode>,
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumVariantDecl {
    pub name: String,
    pub payload: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitAssocTypeDecl {
    pub name: String,
    pub default_ty: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorAttr {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawBlockEntry {
    pub text: String,
    pub span: Span,
    pub children: Vec<RawBlockEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchArm {
    pub patterns: Vec<MatchPattern>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchPattern {
    Wildcard,
    Literal {
        text: String,
    },
    Name {
        text: String,
    },
    Variant {
        path: String,
        args: Vec<MatchPattern>,
    },
    Opaque {
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhraseArg {
    Positional(Expr),
    Named { name: String, value: Expr },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Weave,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    BitOr,
    BitXor,
    BitAnd,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Opaque {
        text: String,
        attached: Vec<RawBlockEntry>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    QualifiedPhrase {
        subject: Box<Expr>,
        args: Vec<PhraseArg>,
        qualifier: String,
        attached: Vec<RawBlockEntry>,
    },
    Await {
        expr: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    MemberAccess {
        expr: Box<Expr>,
        member: String,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        expr: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive_end: bool,
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive_end: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

impl AssignOp {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
            Self::ModAssign => "%=",
            Self::BitAndAssign => "&=",
            Self::BitOrAssign => "|=",
            Self::BitXorAssign => "^=",
            Self::ShlAssign => "<<=",
            Self::ShrAssign => "shr=",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Let {
        mutable: bool,
        name: String,
        value: Expr,
    },
    Return {
        value: Option<Expr>,
    },
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    For {
        binding: String,
        iterable: Expr,
        body: Vec<Statement>,
    },
    Defer {
        expr: Expr,
    },
    Break,
    Continue,
    Assign {
        target: String,
        op: AssignOp,
        value: Expr,
    },
    Expr {
        expr: Expr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolBody {
    None,
    Record {
        fields: Vec<FieldDecl>,
    },
    Enum {
        variants: Vec<EnumVariantDecl>,
    },
    Trait {
        assoc_types: Vec<TraitAssocTypeDecl>,
        methods: Vec<SymbolDecl>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolDecl {
    pub name: String,
    pub kind: SymbolKind,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub where_clause: Option<String>,
    pub params: Vec<ParamDecl>,
    pub return_type: Option<String>,
    pub behavior_attrs: Vec<BehaviorAttr>,
    pub body: SymbolBody,
    pub statements: Vec<Statement>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplAssocTypeBinding {
    pub name: String,
    pub value_ty: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplDecl {
    pub trait_path: Option<String>,
    pub target_type: String,
    pub assoc_types: Vec<ImplAssocTypeBinding>,
    pub methods: Vec<SymbolDecl>,
    pub body_entries: Vec<String>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedModule {
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directives: Vec<ModuleDirective>,
    pub symbols: Vec<SymbolDecl>,
    pub impls: Vec<ImplDecl>,
}

pub fn parse_module(source: &str) -> Result<ParsedModule, String> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut line_count = 0usize;
    let mut non_empty = 0usize;
    let mut source_lines = Vec::with_capacity(lines.len());

    for (idx, line) in lines.iter().enumerate() {
        line_count = idx + 1;
        let analysis = analyze_line(line, idx)?;
        let counts_as_non_empty = analysis.counts_as_non_empty();
        if counts_as_non_empty {
            non_empty += 1;
        }
        source_lines.push(SourceLine {
            text: analysis.trimmed.to_string(),
            leading_spaces: analysis.leading_spaces,
            line: idx + 1,
            counts_as_non_empty,
        });
    }

    let (entries, _) = collect_block_entries(&source_lines, 0, 0)?;
    let mut directives = Vec::new();
    let mut symbols = Vec::new();
    let mut impls = Vec::new();
    for entry in &entries {
        if let Some(directive) = parse_directive(&entry.text, entry.span)? {
            directives.push(directive);
            continue;
        }

        if let Some(impl_decl) = parse_impl_decl(entry)? {
            impls.push(impl_decl);
            continue;
        }

        if let Some(symbol) = parse_symbol_entry(entry)? {
            symbols.push(symbol);
        }
    }

    Ok(ParsedModule {
        line_count: line_count.max(1),
        non_empty_line_count: non_empty,
        directives,
        symbols,
        impls,
    })
}

struct AnalyzedLine<'a> {
    trimmed: &'a str,
    leading_spaces: usize,
}

impl AnalyzedLine<'_> {
    fn counts_as_non_empty(&self) -> bool {
        !self.trimmed.is_empty() && !self.trimmed.starts_with('#')
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceLine {
    text: String,
    leading_spaces: usize,
    line: usize,
    counts_as_non_empty: bool,
}

fn analyze_line<'a>(line: &'a str, line_index: usize) -> Result<AnalyzedLine<'a>, String> {
    let mut leading_spaces = 0usize;
    for (column, ch) in line.chars().enumerate() {
        match ch {
            ' ' => {
                leading_spaces += 1;
                continue;
            }
            '\t' => {
                return Err(format!(
                    "{}:{}: tabs are not allowed in indentation",
                    line_index + 1,
                    column + 1
                ));
            }
            _ => break,
        }
    }

    Ok(AnalyzedLine {
        trimmed: line.trim(),
        leading_spaces,
    })
}

fn collect_block_entries(
    lines: &[SourceLine],
    start_index: usize,
    indent: usize,
) -> Result<(Vec<RawBlockEntry>, usize), String> {
    let mut index = start_index;
    let mut entries = Vec::new();

    while let Some(next_index) = next_non_empty_index(lines, index) {
        let line = &lines[next_index];
        if line.leading_spaces < indent {
            return Ok((entries, next_index));
        }
        if line.leading_spaces > indent {
            return Err(format!("{}:{}: unexpected indentation", line.line, 1));
        }

        let mut entry = RawBlockEntry {
            text: line.text.clone(),
            span: Span::new(line.line, 1),
            children: Vec::new(),
        };

        index = next_index + 1;
        if let Some(child_index) = next_non_empty_index(lines, index) {
            let child = &lines[child_index];
            if child.leading_spaces > indent {
                let (children, next_child_index) =
                    collect_block_entries(lines, child_index, child.leading_spaces)?;
                entry.children = children;
                index = next_child_index;
            }
        }

        entries.push(entry);
    }

    Ok((entries, lines.len()))
}

fn next_non_empty_index(lines: &[SourceLine], mut index: usize) -> Option<usize> {
    while index < lines.len() {
        if lines[index].counts_as_non_empty {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_directive(trimmed: &str, span: Span) -> Result<Option<ModuleDirective>, String> {
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("import ") {
        (DirectiveKind::Import, rest)
    } else if let Some(rest) = trimmed.strip_prefix("use ") {
        (DirectiveKind::Use, rest)
    } else if let Some(rest) = trimmed.strip_prefix("reexport ") {
        (DirectiveKind::Reexport, rest)
    } else {
        return Ok(None);
    };

    let (path_text, alias) = match rest.split_once(" as ") {
        Some((path, alias)) => (path, Some(alias)),
        None => (rest, None),
    };
    let path = parse_path(path_text).map_err(|detail| {
        format!(
            "{}:{}: malformed {} directive: {}",
            span.line,
            span.column,
            kind.keyword(),
            detail
        )
    })?;
    let alias = alias
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .map(|alias| {
            if is_identifier(alias) {
                Ok(alias.to_string())
            } else {
                Err(format!(
                    "{}:{}: malformed {} directive: invalid alias `{}`",
                    span.line,
                    span.column,
                    kind.keyword(),
                    alias
                ))
            }
        })
        .transpose()?;

    Ok(Some(ModuleDirective {
        kind,
        path,
        alias,
        span,
    }))
}

fn parse_path(path: &str) -> Result<Vec<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("missing path".to_string());
    }

    let segments = trimmed
        .split('.')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(format!("invalid path `{trimmed}`"));
    }
    for segment in &segments {
        if !is_identifier(segment) {
            return Err(format!("invalid path segment `{segment}`"));
        }
    }
    Ok(segments)
}

fn parse_symbol_entry(entry: &RawBlockEntry) -> Result<Option<SymbolDecl>, String> {
    let Some(mut symbol) = parse_symbol_header(&entry.text, entry.span) else {
        return Ok(None);
    };
    symbol.surface_text = collect_symbol_surface(&entry.text, &symbol.kind, &entry.children);
    symbol.body = parse_symbol_body(&symbol.kind, &entry.children)?;
    symbol.statements = parse_symbol_statements(&symbol.kind, &entry.children)?;
    Ok(Some(symbol))
}

fn parse_symbol_header(trimmed: &str, span: Span) -> Option<SymbolDecl> {
    let exported = trimmed.starts_with("export ");
    let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    if let Some(symbol) = parse_behavior_symbol(rest, exported, span) {
        return Some(symbol);
    }
    let (is_async, rest) = if let Some(rest) = rest.strip_prefix("async ") {
        (true, rest)
    } else {
        (false, rest)
    };
    for (keyword, kind) in [
        ("fn", SymbolKind::Fn),
        ("record", SymbolKind::Record),
        ("enum", SymbolKind::Enum),
        ("trait", SymbolKind::Trait),
        ("behavior", SymbolKind::Behavior),
        ("const", SymbolKind::Const),
    ] {
        let Some(rest) = rest.strip_prefix(keyword) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(' ') else {
            continue;
        };
        let signature = parse_symbol_signature(kind.clone(), rest)?;
        return Some(SymbolDecl {
            name: signature.name,
            kind,
            exported,
            is_async,
            type_params: signature.type_params,
            where_clause: signature.where_clause,
            params: signature.params,
            return_type: signature.return_type,
            behavior_attrs: Vec::new(),
            body: SymbolBody::None,
            statements: Vec::new(),
            surface_text: trimmed.to_string(),
            span,
        });
    }
    None
}

fn parse_behavior_symbol(rest: &str, exported: bool, span: Span) -> Option<SymbolDecl> {
    let open_idx = rest.find('[')?;
    if !rest[..open_idx].trim().eq("behavior") {
        return None;
    }
    let close_idx = find_matching_delim(rest, open_idx, '[', ']')?;
    let attrs = parse_behavior_attrs(&rest[open_idx + 1..close_idx]).ok()?;
    let after_attrs = rest[close_idx + 1..].trim();
    let fn_rest = after_attrs.strip_prefix("fn ")?;
    let signature = parse_symbol_signature(SymbolKind::Fn, fn_rest)?;
    Some(SymbolDecl {
        name: signature.name,
        kind: SymbolKind::Behavior,
        exported,
        is_async: false,
        type_params: signature.type_params,
        where_clause: signature.where_clause,
        params: signature.params,
        return_type: signature.return_type,
        behavior_attrs: attrs,
        body: SymbolBody::None,
        statements: Vec::new(),
        surface_text: format!(
            "behavior[{}] fn {}",
            &rest[open_idx + 1..close_idx],
            fn_rest
        ),
        span,
    })
}

fn collect_symbol_surface(trimmed: &str, kind: &SymbolKind, entries: &[RawBlockEntry]) -> String {
    let mut surface_lines = vec![
        trimmed
            .strip_prefix("export ")
            .unwrap_or(trimmed)
            .to_string(),
    ];

    if matches!(
        kind,
        SymbolKind::Fn | SymbolKind::Behavior | SymbolKind::Const
    ) {
        return surface_lines.join("\n");
    }

    surface_lines.extend(entries.iter().map(|entry| entry.text.clone()));
    surface_lines.join("\n")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedSymbolSignature {
    name: String,
    type_params: Vec<String>,
    where_clause: Option<String>,
    params: Vec<ParamDecl>,
    return_type: Option<String>,
}

fn parse_symbol_signature(kind: SymbolKind, rest: &str) -> Option<ParsedSymbolSignature> {
    let rest = rest.trim();
    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let name = parse_symbol_name(header)?;
    let after_name = &header[name.len()..];
    let (type_params, where_clause, params, return_type) = match kind {
        SymbolKind::Fn => parse_function_signature_tail(after_name)?,
        SymbolKind::Record | SymbolKind::Enum | SymbolKind::Trait | SymbolKind::Behavior => {
            parse_named_type_tail(after_name)?
        }
        SymbolKind::Const => parse_const_signature_tail(after_name),
    };

    Some(ParsedSymbolSignature {
        name,
        type_params,
        where_clause,
        params,
        return_type,
    })
}

fn parse_function_signature_tail(
    tail: &str,
) -> Option<(Vec<String>, Option<String>, Vec<ParamDecl>, Option<String>)> {
    let tail = tail.trim();
    let (type_params, where_clause, remainder) = parse_type_params_and_where(tail)?;
    let remainder = remainder.trim();
    let open_idx = remainder.find('(')?;
    let close_idx = find_matching_delim(remainder, open_idx, '(', ')')?;
    let params = parse_param_list(&remainder[open_idx + 1..close_idx]).ok()?;
    let after_params = remainder[close_idx + 1..].trim();
    let return_type = after_params
        .strip_prefix("->")
        .map(|ty| ty.trim().to_string())
        .filter(|ty| !ty.is_empty());
    Some((type_params, where_clause, params, return_type))
}

fn parse_named_type_tail(
    tail: &str,
) -> Option<(Vec<String>, Option<String>, Vec<ParamDecl>, Option<String>)> {
    let (type_params, where_clause, remainder) = parse_type_params_and_where(tail.trim())?;
    if !remainder.trim().is_empty() {
        return None;
    }
    Some((type_params, where_clause, Vec::new(), None))
}

fn parse_const_signature_tail(
    tail: &str,
) -> (Vec<String>, Option<String>, Vec<ParamDecl>, Option<String>) {
    let return_type = tail
        .trim()
        .strip_prefix(':')
        .map(|ty| ty.trim().to_string())
        .filter(|ty| !ty.is_empty());
    (Vec::new(), None, Vec::new(), return_type)
}

fn parse_type_params_and_where(tail: &str) -> Option<(Vec<String>, Option<String>, &str)> {
    let tail = tail.trim();
    let Some('[') = tail.chars().next() else {
        return Some((Vec::new(), None, tail));
    };
    let close_idx = find_matching_delim(tail, 0, '[', ']')?;
    let inside = &tail[1..close_idx];
    let mut type_params = Vec::new();
    let mut where_clause = None;
    for part in split_top_level(inside, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(clause) = part.strip_prefix("where ") {
            where_clause = Some(clause.trim().to_string());
        } else {
            type_params.push(part.to_string());
        }
    }
    Some((type_params, where_clause, &tail[close_idx + 1..]))
}

fn parse_param_list(source: &str) -> Result<Vec<ParamDecl>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(Vec::new());
    }

    let mut params = Vec::new();
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (mode, rest) = if let Some(rest) = part.strip_prefix("read ") {
            (Some(ParamMode::Read), rest)
        } else if let Some(rest) = part.strip_prefix("edit ") {
            (Some(ParamMode::Edit), rest)
        } else if let Some(rest) = part.strip_prefix("take ") {
            (Some(ParamMode::Take), rest)
        } else {
            (None, part)
        };

        let (name, ty) = rest
            .split_once(':')
            .ok_or_else(|| format!("malformed parameter `{part}`"))?;
        let name = name.trim();
        let ty = ty.trim();
        if !is_identifier(name) || ty.is_empty() {
            return Err(format!("malformed parameter `{part}`"));
        }
        params.push(ParamDecl {
            mode,
            name: name.to_string(),
            ty: ty.to_string(),
        });
    }

    Ok(params)
}

fn parse_behavior_attrs(source: &str) -> Result<Vec<BehaviorAttr>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(Vec::new());
    }

    let mut attrs = Vec::new();
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| format!("malformed behavior attribute `{part}`"))?;
        let name = name.trim();
        let value = value.trim();
        if !is_identifier(name) || value.is_empty() {
            return Err(format!("malformed behavior attribute `{part}`"));
        }
        attrs.push(BehaviorAttr {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
    Ok(attrs)
}

fn parse_impl_decl(entry: &RawBlockEntry) -> Result<Option<ImplDecl>, String> {
    let Some(rest) = entry.text.strip_prefix("impl ") else {
        return Ok(None);
    };
    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let (trait_path, target_type) = match header.rsplit_once(" for ") {
        Some((trait_path, target_type)) => (
            Some(trait_path.trim().to_string()),
            target_type.trim().to_string(),
        ),
        None => (None, header.to_string()),
    };
    if target_type.is_empty() {
        return Err(format!(
            "{}:{}: malformed impl declaration",
            entry.span.line, entry.span.column
        ));
    }
    let body_entries = entry
        .children
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    let mut assoc_types = Vec::new();
    let mut methods = Vec::new();
    for child in &entry.children {
        if let Some(assoc_type) = parse_impl_assoc_type_binding(&child.text, child.span) {
            assoc_types.push(assoc_type);
            continue;
        }
        if let Some(method) = parse_symbol_entry(child)? {
            methods.push(method);
        }
    }
    let mut surface_lines = vec![entry.text.clone()];
    surface_lines.extend(body_entries.iter().cloned());
    Ok(Some(ImplDecl {
        trait_path,
        target_type,
        assoc_types,
        methods,
        body_entries,
        surface_text: surface_lines.join("\n"),
        span: entry.span,
    }))
}

fn parse_symbol_body(kind: &SymbolKind, entries: &[RawBlockEntry]) -> Result<SymbolBody, String> {
    match kind {
        SymbolKind::Fn | SymbolKind::Const | SymbolKind::Behavior => Ok(SymbolBody::None),
        SymbolKind::Record => Ok(SymbolBody::Record {
            fields: entries
                .iter()
                .filter_map(|entry| parse_field_decl(&entry.text, entry.span))
                .collect(),
        }),
        SymbolKind::Enum => Ok(SymbolBody::Enum {
            variants: entries
                .iter()
                .filter_map(|entry| parse_enum_variant_decl(&entry.text, entry.span))
                .collect(),
        }),
        SymbolKind::Trait => {
            let mut assoc_types = Vec::new();
            let mut methods = Vec::new();
            for entry in entries {
                if let Some(assoc_type) = parse_trait_assoc_type_decl(&entry.text, entry.span) {
                    assoc_types.push(assoc_type);
                    continue;
                }
                if let Some(method) = parse_symbol_entry(entry)? {
                    methods.push(method);
                }
            }
            Ok(SymbolBody::Trait {
                assoc_types,
                methods,
            })
        }
    }
}

fn parse_symbol_statements(
    kind: &SymbolKind,
    entries: &[RawBlockEntry],
) -> Result<Vec<Statement>, String> {
    match kind {
        SymbolKind::Fn | SymbolKind::Behavior => parse_statement_block(entries, 0),
        SymbolKind::Trait | SymbolKind::Record | SymbolKind::Enum | SymbolKind::Const => {
            Ok(Vec::new())
        }
    }
}

fn parse_statement_block(
    entries: &[RawBlockEntry],
    loop_depth: usize,
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut index = 0usize;
    while index < entries.len() {
        let entry = &entries[index];
        if entry.text == "else:" {
            return Err(format!(
                "{}:{}: `else` without a preceding `if`",
                entry.span.line, entry.span.column
            ));
        }
        if entry.text.starts_with("else ") {
            return Err(format!(
                "{}:{}: malformed `else` clause",
                entry.span.line, entry.span.column
            ));
        }

        let mut statement = parse_statement(entry, loop_depth)?;
        if let StatementKind::If { else_branch, .. } = &mut statement.kind {
            if let Some(next) = entries.get(index + 1) {
                if next.text == "else:" {
                    *else_branch = Some(parse_statement_block(&next.children, loop_depth)?);
                    index += 1;
                } else if next.text.starts_with("else ") {
                    return Err(format!(
                        "{}:{}: malformed `else` clause",
                        next.span.line, next.span.column
                    ));
                }
            }
        }

        statements.push(statement);
        index += 1;
    }

    Ok(statements)
}

fn parse_statement(entry: &RawBlockEntry, loop_depth: usize) -> Result<Statement, String> {
    if let Some(rest) = entry.text.strip_prefix("if ") {
        let condition = parse_expression(
            &parse_block_header(rest, "if", entry.span)?,
            &[],
            entry.span,
        )?;
        return Ok(Statement {
            kind: StatementKind::If {
                condition,
                then_branch: parse_statement_block(&entry.children, loop_depth)?,
                else_branch: None,
            },
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("while ") {
        let condition = parse_expression(
            &parse_block_header(rest, "while", entry.span)?,
            &[],
            entry.span,
        )?;
        return Ok(Statement {
            kind: StatementKind::While {
                condition,
                body: parse_statement_block(&entry.children, loop_depth + 1)?,
            },
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("for ") {
        let header = parse_block_header(rest, "for", entry.span)?;
        let (binding, iterable) = header.split_once(" in ").ok_or_else(|| {
            format!(
                "{}:{}: malformed `for` statement",
                entry.span.line, entry.span.column
            )
        })?;
        let binding = binding.trim();
        let iterable = iterable.trim();
        if !is_identifier(binding) || iterable.is_empty() {
            return Err(format!(
                "{}:{}: malformed `for` statement",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::For {
                binding: binding.to_string(),
                iterable: parse_expression(iterable, &[], entry.span)?,
                body: parse_statement_block(&entry.children, loop_depth + 1)?,
            },
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("let ") {
        let (mutable, rest) = if let Some(rest) = rest.strip_prefix("mut ") {
            (true, rest)
        } else {
            (false, rest)
        };
        let (name, value) = rest.split_once('=').ok_or_else(|| {
            format!(
                "{}:{}: malformed `let` statement",
                entry.span.line, entry.span.column
            )
        })?;
        let name = name.trim();
        let value = value.trim();
        if !is_identifier(name) || value.is_empty() {
            return Err(format!(
                "{}:{}: malformed `let` statement",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Let {
                mutable,
                name: name.to_string(),
                value: parse_expression(value, &entry.children, entry.span)?,
            },
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("return") {
        let value = match rest.trim() {
            "" if entry.children.is_empty() => None,
            "" => {
                return Err(format!(
                    "{}:{}: malformed `return` statement",
                    entry.span.line, entry.span.column
                ));
            }
            value => Some(parse_expression(value, &entry.children, entry.span)?),
        };
        return Ok(Statement {
            kind: StatementKind::Return { value },
            span: entry.span,
        });
    }

    if let Some(rest) = entry.text.strip_prefix("defer ") {
        return Ok(Statement {
            kind: StatementKind::Defer {
                expr: parse_expression(rest, &entry.children, entry.span)?,
            },
            span: entry.span,
        });
    }

    if entry.text == "break" {
        if loop_depth == 0 {
            return Err(format!(
                "{}:{}: `break` is only valid inside loops",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Break,
            span: entry.span,
        });
    }

    if entry.text == "continue" {
        if loop_depth == 0 {
            return Err(format!(
                "{}:{}: `continue` is only valid inside loops",
                entry.span.line, entry.span.column
            ));
        }
        return Ok(Statement {
            kind: StatementKind::Continue,
            span: entry.span,
        });
    }

    if let Some((target, op, value)) = parse_assignment_statement(&entry.text) {
        return Ok(Statement {
            kind: StatementKind::Assign {
                target,
                op,
                value: parse_expression(&value, &entry.children, entry.span)?,
            },
            span: entry.span,
        });
    }

    Ok(Statement {
        kind: StatementKind::Expr {
            expr: parse_expression(&entry.text, &entry.children, entry.span)?,
        },
        span: entry.span,
    })
}

fn parse_expression(text: &str, attached: &[RawBlockEntry], span: Span) -> Result<Expr, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{}:{}: malformed expression",
            span.line, span.column
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("match ") {
        return parse_match_expression(rest, attached, span);
    }

    let expr = parse_expression_core(trimmed)?;
    if attached.is_empty() {
        return Ok(expr);
    }

    match expr {
        Expr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            ..
        } => Ok(Expr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached: attached.to_vec(),
        }),
        _ => Ok(Expr::Opaque {
            text: trimmed.to_string(),
            attached: attached.to_vec(),
        }),
    }
}

fn parse_expression_core(text: &str) -> Result<Expr, String> {
    let trimmed = text.trim();
    if let Some(inner) = strip_group_parens(trimmed) {
        return parse_expression_core(inner);
    }
    parse_range_expression(trimmed)
}

#[derive(Clone, Copy)]
struct BinaryOpSpec {
    token: &'static str,
    op: BinaryOp,
    keyword: bool,
}

impl BinaryOpSpec {
    const fn keyword(token: &'static str, op: BinaryOp) -> Self {
        Self {
            token,
            op,
            keyword: true,
        }
    }

    const fn symbol(token: &'static str, op: BinaryOp) -> Self {
        Self {
            token,
            op,
            keyword: false,
        }
    }
}

fn parse_range_expression(text: &str) -> Result<Expr, String> {
    if let Some(expr) = parse_range(text)? {
        return Ok(expr);
    }
    parse_logical_or_expression(text)
}

fn parse_logical_or_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_logical_and_expression,
        &[BinaryOpSpec::keyword("or", BinaryOp::Or)],
    )
}

fn parse_logical_and_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_equality_expression,
        &[BinaryOpSpec::keyword("and", BinaryOp::And)],
    )
}

fn parse_equality_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_comparison_expression,
        &[
            BinaryOpSpec::symbol("==", BinaryOp::EqEq),
            BinaryOpSpec::symbol("!=", BinaryOp::NotEq),
        ],
    )
}

fn parse_comparison_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_bit_or_expression,
        &[
            BinaryOpSpec::symbol("<=", BinaryOp::LtEq),
            BinaryOpSpec::symbol(">=", BinaryOp::GtEq),
            BinaryOpSpec::symbol("<", BinaryOp::Lt),
            BinaryOpSpec::symbol(">", BinaryOp::Gt),
        ],
    )
}

fn parse_bit_or_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_bit_xor_expression,
        &[BinaryOpSpec::symbol("|", BinaryOp::BitOr)],
    )
}

fn parse_bit_xor_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_bit_and_expression,
        &[BinaryOpSpec::symbol("^", BinaryOp::BitXor)],
    )
}

fn parse_bit_and_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_shift_expression,
        &[BinaryOpSpec::symbol("&", BinaryOp::BitAnd)],
    )
}

fn parse_shift_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_additive_expression,
        &[
            BinaryOpSpec::symbol("<<", BinaryOp::Shl),
            BinaryOpSpec::keyword("shr", BinaryOp::Shr),
        ],
    )
}

fn parse_additive_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_multiplicative_expression,
        &[
            BinaryOpSpec::symbol("+", BinaryOp::Add),
            BinaryOpSpec::symbol("-", BinaryOp::Sub),
        ],
    )
}

fn parse_multiplicative_expression(text: &str) -> Result<Expr, String> {
    parse_binary_layer(
        text,
        parse_unary_expression,
        &[
            BinaryOpSpec::symbol("*", BinaryOp::Mul),
            BinaryOpSpec::symbol("/", BinaryOp::Div),
            BinaryOpSpec::symbol("%", BinaryOp::Mod),
        ],
    )
}

fn parse_binary_layer(
    text: &str,
    lower: fn(&str) -> Result<Expr, String>,
    ops: &[BinaryOpSpec],
) -> Result<Expr, String> {
    if let Some((index, op, token_len)) = find_top_level_binary_op(text, ops) {
        let left = text[..index].trim();
        let right = text[index + token_len..].trim();
        if !left.is_empty() && !right.is_empty() {
            return Ok(Expr::Binary {
                left: Box::new(parse_binary_layer(left, lower, ops)?),
                op,
                right: Box::new(lower(right)?),
            });
        }
    }
    lower(text)
}

fn parse_unary_expression(text: &str) -> Result<Expr, String> {
    if let Some(inner) = strip_group_parens(text) {
        return parse_expression_core(inner);
    }
    if let Some(rest) = strip_keyword_prefix(text, "weave") {
        return Ok(Expr::Unary {
            op: UnaryOp::Weave,
            expr: Box::new(parse_unary_expression(rest)?),
        });
    }
    if let Some(rest) = strip_keyword_prefix(text, "split") {
        return Ok(Expr::Unary {
            op: UnaryOp::Split,
            expr: Box::new(parse_unary_expression(rest)?),
        });
    }
    if let Some(rest) = strip_keyword_prefix(text, "not") {
        return Ok(Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(parse_unary_expression(rest)?),
        });
    }
    if let Some(rest) = text.strip_prefix('-') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(parse_unary_expression(rest)?),
            });
        }
    }
    if let Some(rest) = text.strip_prefix('~') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(parse_unary_expression(rest)?),
            });
        }
    }
    parse_postfix_expression(text)
}

fn parse_postfix_expression(text: &str) -> Result<Expr, String> {
    if let Some(expr) = parse_qualified_phrase(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_await_expression(text)? {
        return Ok(expr);
    }
    if let Some(expr) = parse_access_expression(text)? {
        return Ok(expr);
    }
    Ok(Expr::Opaque {
        text: text.trim().to_string(),
        attached: Vec::new(),
    })
}

fn parse_qualified_phrase(text: &str) -> Result<Option<Expr>, String> {
    let positions = find_top_level_token_positions(text, "::");
    if positions.len() != 2 {
        return Ok(None);
    }

    let subject_text = text[..positions[0]].trim();
    let args_text = text[positions[0] + 2..positions[1]].trim();
    let qualifier = text[positions[1] + 2..].trim();
    if subject_text.is_empty() || qualifier.is_empty() {
        return Ok(None);
    }

    let args = match parse_phrase_args(args_text)? {
        Some(args) => args,
        None => return Ok(None),
    };

    Ok(Some(Expr::QualifiedPhrase {
        subject: Box::new(parse_expression_core(subject_text)?),
        args,
        qualifier: qualifier.to_string(),
        attached: Vec::new(),
    }))
}

fn parse_phrase_args(text: &str) -> Result<Option<Vec<PhraseArg>>, String> {
    if text.is_empty() {
        return Ok(Some(Vec::new()));
    }

    let mut args = Vec::new();
    for part in split_top_level(text, ',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        args.push(parse_phrase_arg(trimmed)?);
    }
    Ok(Some(args))
}

fn parse_phrase_arg(text: &str) -> Result<PhraseArg, String> {
    if let Some(index) = find_top_level_named_eq(text) {
        let name = text[..index].trim();
        let value = text[index + 1..].trim();
        if is_identifier(name) && !value.is_empty() {
            return Ok(PhraseArg::Named {
                name: name.to_string(),
                value: parse_expression_core(value)?,
            });
        }
    }

    Ok(PhraseArg::Positional(parse_expression_core(text)?))
}

fn parse_await_expression(text: &str) -> Result<Option<Expr>, String> {
    let Some(index) = find_top_level_token(text, ">>") else {
        return Ok(None);
    };
    let left = text[..index].trim();
    let right = text[index + 2..].trim();
    if left.is_empty() || right != "await" {
        return Ok(None);
    }
    Ok(Some(Expr::Await {
        expr: Box::new(parse_expression_core(left)?),
    }))
}

fn parse_access_expression(text: &str) -> Result<Option<Expr>, String> {
    if let Some((base, inside)) = split_trailing_bracket_suffix(text) {
        let base = base.trim();
        if base.is_empty() {
            return Ok(None);
        }
        if let Some((start, end, inclusive_end)) = parse_range_parts(inside) {
            return Ok(Some(Expr::Slice {
                expr: Box::new(parse_expression_core(base)?),
                start: parse_optional_range_bound(start)?,
                end: parse_optional_range_bound(end)?,
                inclusive_end,
            }));
        }
        if should_parse_index_brackets(inside) {
            return Ok(Some(Expr::Index {
                expr: Box::new(parse_expression_core(base)?),
                index: Box::new(parse_expression_core(inside.trim())?),
            }));
        }
    }

    if let Some((base, member)) = split_member_access(text) {
        return Ok(Some(Expr::MemberAccess {
            expr: Box::new(parse_expression_core(base.trim())?),
            member: member.trim().to_string(),
        }));
    }

    Ok(None)
}

fn parse_range(text: &str) -> Result<Option<Expr>, String> {
    let Some((start, end, inclusive_end)) = parse_range_parts(text) else {
        return Ok(None);
    };
    if start.trim().is_empty() && end.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(Expr::Range {
        start: parse_optional_range_bound(start)?,
        end: parse_optional_range_bound(end)?,
        inclusive_end,
    }))
}

fn parse_optional_range_bound(text: &str) -> Result<Option<Box<Expr>>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(Box::new(parse_logical_or_expression(trimmed)?)))
}

fn parse_range_parts(text: &str) -> Option<(&str, &str, bool)> {
    if let Some(index) = find_top_level_token(text, "..=") {
        return Some((&text[..index], &text[index + 3..], true));
    }
    find_top_level_token(text, "..").map(|index| (&text[..index], &text[index + 2..], false))
}

fn split_trailing_bracket_suffix(text: &str) -> Option<(&str, &str)> {
    if !text.ends_with(']') {
        return None;
    }

    let mut candidate = None;
    for (index, ch) in text.char_indices() {
        if ch != '[' {
            continue;
        }
        let Some(close) = find_matching_delim(text, index, '[', ']') else {
            continue;
        };
        if close == text.len() - 1 {
            candidate = Some(index);
        }
    }

    let open = candidate?;
    Some((&text[..open], &text[open + 1..text.len() - 1]))
}

fn should_parse_index_brackets(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if parse_range_parts(trimmed).is_some() {
        return true;
    }
    if matches!(trimmed, "true" | "false")
        || trimmed.starts_with('"')
        || trimmed.parse::<i64>().is_ok()
    {
        return true;
    }
    if trimmed.starts_with('(')
        || trimmed.starts_with('[')
        || trimmed.starts_with('-')
        || trimmed.starts_with('~')
    {
        return true;
    }
    if trimmed.starts_with("not ")
        || trimmed.starts_with("weave ")
        || trimmed.starts_with("split ")
        || trimmed.contains("::")
        || trimmed.contains(">>")
    {
        return true;
    }
    if let Some(first) = trimmed.chars().next() {
        return first.is_ascii_lowercase() || first == '_';
    }
    false
}

fn split_member_access(text: &str) -> Option<(&str, &str)> {
    let positions = find_top_level_dot_positions(text);
    let index = *positions.last()?;
    let base = text[..index].trim();
    let member = text[index + 1..].trim();
    if base.is_empty() || member.is_empty() {
        return None;
    }
    Some((base, member))
}

fn find_top_level_binary_op(text: &str, ops: &[BinaryOpSpec]) -> Option<(usize, BinaryOp, usize)> {
    let mut candidate = None;
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        for spec in ops {
            if operator_matches_at(text, idx, *spec) {
                candidate = Some((idx, spec.op, spec.token.len()));
                break;
            }
        }
    }

    candidate
}

fn operator_matches_at(text: &str, index: usize, spec: BinaryOpSpec) -> bool {
    if !text[index..].starts_with(spec.token) {
        return false;
    }
    if spec.keyword {
        return has_word_boundary_before(text, index)
            && has_word_boundary_after(text, index + spec.token.len());
    }

    match spec.token {
        "<" => {
            !matches!(text[index + 1..].chars().next(), Some('=' | '<'))
                && !matches!(text[..index].chars().next_back(), Some('<'))
        }
        ">" => {
            !matches!(text[index + 1..].chars().next(), Some('=' | '>'))
                && !matches!(text[..index].chars().next_back(), Some('>'))
        }
        "|" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "&" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "+" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "-" => !matches!(text[index + 1..].chars().next(), Some('=' | '>')),
        "*" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "/" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "%" => !matches!(text[index + 1..].chars().next(), Some('=')),
        "^" => !matches!(text[index + 1..].chars().next(), Some('=')),
        _ => true,
    }
}

fn strip_keyword_prefix<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    if rest.is_empty() || !has_word_boundary_after(text, keyword.len()) {
        return None;
    }
    Some(rest.trim_start())
}

fn strip_group_parens(text: &str) -> Option<&str> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return None;
    }
    let close = find_matching_delim(text, 0, '(', ')')?;
    if close != text.len() - 1 {
        return None;
    }
    let inner = text[1..close].trim();
    if inner.is_empty() || contains_top_level_char(inner, ',') {
        return None;
    }
    Some(inner)
}

fn contains_top_level_char(text: &str, needle: char) -> bool {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for ch in text.chars() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 && ch == needle {
            return true;
        }
    }

    false
}

fn find_top_level_named_eq(text: &str) -> Option<usize> {
    let (index, op, len) = find_top_level_assignment_op(text)?;
    if op == AssignOp::Assign && len == 1 {
        return Some(index);
    }
    None
}

fn has_word_boundary_before(text: &str, index: usize) -> bool {
    !matches!(text[..index].chars().next_back(), Some(ch) if is_identifier_continue(ch))
}

fn has_word_boundary_after(text: &str, index: usize) -> bool {
    !matches!(text[index..].chars().next(), Some(ch) if is_identifier_continue(ch))
}

fn find_top_level_token_positions(text: &str, token: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        if text[idx..].starts_with(token) {
            positions.push(idx);
        }
    }

    positions
}

fn find_top_level_dot_positions(text: &str) -> Vec<usize> {
    find_top_level_token_positions(text, ".")
        .into_iter()
        .filter(|index| {
            !matches!(text[..*index].chars().next_back(), Some('.'))
                && !matches!(text[*index + 1..].chars().next(), Some('.'))
        })
        .collect()
}

fn parse_match_expression(
    rest: &str,
    attached: &[RawBlockEntry],
    span: Span,
) -> Result<Expr, String> {
    let Some(subject) = rest.strip_suffix(':') else {
        return Err(format!(
            "{}:{}: malformed `match` expression",
            span.line, span.column
        ));
    };
    let subject = subject.trim();
    if subject.is_empty() || attached.is_empty() {
        return Err(format!(
            "{}:{}: malformed `match` expression",
            span.line, span.column
        ));
    }

    let mut arms = Vec::new();
    for entry in attached {
        arms.push(parse_match_arm(entry)?);
    }

    Ok(Expr::Match {
        subject: Box::new(parse_expression_core(subject)?),
        arms,
    })
}

fn parse_match_arm(entry: &RawBlockEntry) -> Result<MatchArm, String> {
    let Some(index) = find_top_level_token(&entry.text, "=>") else {
        return Err(format!(
            "{}:{}: malformed `match` arm",
            entry.span.line, entry.span.column
        ));
    };
    let patterns_text = entry.text[..index].trim();
    let value_text = entry.text[index + 2..].trim();
    if patterns_text.is_empty() || value_text.is_empty() {
        return Err(format!(
            "{}:{}: malformed `match` arm",
            entry.span.line, entry.span.column
        ));
    }

    let patterns = split_top_level(patterns_text, '|')
        .into_iter()
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(parse_match_pattern)
        .collect::<Result<Vec<_>, _>>()?;
    if patterns.is_empty() {
        return Err(format!(
            "{}:{}: malformed `match` arm",
            entry.span.line, entry.span.column
        ));
    }

    Ok(MatchArm {
        patterns,
        value: parse_expression(value_text, &entry.children, entry.span)?,
        span: entry.span,
    })
}

fn parse_match_pattern(text: &str) -> Result<MatchPattern, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("malformed `match` pattern".to_string());
    }
    if trimmed == "_" {
        return Ok(MatchPattern::Wildcard);
    }
    if is_match_literal(trimmed) {
        return Ok(MatchPattern::Literal {
            text: trimmed.to_string(),
        });
    }
    if let Some(variant) = parse_variant_pattern(trimmed)? {
        return Ok(variant);
    }
    if is_path_like(trimmed) {
        return Ok(MatchPattern::Name {
            text: trimmed.to_string(),
        });
    }
    Ok(MatchPattern::Opaque {
        text: trimmed.to_string(),
    })
}

fn parse_variant_pattern(text: &str) -> Result<Option<MatchPattern>, String> {
    let Some(open_idx) = text.find('(') else {
        return Ok(None);
    };
    let Some(close_idx) = find_matching_delim(text, open_idx, '(', ')') else {
        return Ok(None);
    };
    if close_idx != text.len() - 1 {
        return Ok(None);
    }
    let path = text[..open_idx].trim();
    if !is_path_like(path) {
        return Ok(None);
    }
    let inside = text[open_idx + 1..close_idx].trim();
    let args = if inside.is_empty() {
        Vec::new()
    } else {
        split_top_level(inside, ',')
            .into_iter()
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .map(parse_match_pattern)
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(Some(MatchPattern::Variant {
        path: path.to_string(),
        args,
    }))
}

fn is_match_literal(text: &str) -> bool {
    matches!(text, "true" | "false") || text.starts_with('"') || text.parse::<i64>().is_ok()
}

fn parse_block_header(rest: &str, keyword: &str, span: Span) -> Result<String, String> {
    let Some(header) = rest.strip_suffix(':') else {
        return Err(format!(
            "{}:{}: malformed `{keyword}` statement",
            span.line, span.column
        ));
    };
    let header = header.trim();
    if header.is_empty() {
        return Err(format!(
            "{}:{}: malformed `{keyword}` statement",
            span.line, span.column
        ));
    }
    Ok(header.to_string())
}

fn parse_assignment_statement(text: &str) -> Option<(String, AssignOp, String)> {
    let (index, op, op_len) = find_top_level_assignment_op(text)?;
    let target = text[..index].trim();
    let value = text[index + op_len..].trim();
    if target.is_empty() || value.is_empty() {
        return None;
    }
    Some((target.to_string(), op, value.to_string()))
}

fn find_top_level_assignment_op(text: &str) -> Option<(usize, AssignOp, usize)> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren != 0 || depth_bracket != 0 || depth_brace != 0 {
            continue;
        }

        for (token, op) in [
            ("<<=", AssignOp::ShlAssign),
            ("shr=", AssignOp::ShrAssign),
            ("+=", AssignOp::AddAssign),
            ("-=", AssignOp::SubAssign),
            ("*=", AssignOp::MulAssign),
            ("/=", AssignOp::DivAssign),
            ("%=", AssignOp::ModAssign),
            ("&=", AssignOp::BitAndAssign),
            ("|=", AssignOp::BitOrAssign),
            ("^=", AssignOp::BitXorAssign),
            ("=", AssignOp::Assign),
        ] {
            if !text[idx..].starts_with(token) {
                continue;
            }
            if token == "=" {
                let prev = text[..idx].chars().next_back();
                let next = text[idx + 1..].chars().next();
                if matches!(prev, Some('<' | '>' | '!' | '=')) || matches!(next, Some('=' | '>')) {
                    continue;
                }
            }
            return Some((idx, op, token.len()));
        }
    }

    None
}

fn find_top_level_token(text: &str, token: &str) -> Option<usize> {
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if depth_paren == 0
            && depth_bracket == 0
            && depth_brace == 0
            && text[idx..].starts_with(token)
        {
            return Some(idx);
        }
    }

    None
}

fn parse_field_decl(trimmed: &str, span: Span) -> Option<FieldDecl> {
    let (name, ty) = trimmed.split_once(':')?;
    let name = name.trim();
    let ty = ty.trim();
    if !is_identifier(name) || ty.is_empty() {
        return None;
    }
    Some(FieldDecl {
        name: name.to_string(),
        ty: ty.to_string(),
        span,
    })
}

fn parse_enum_variant_decl(trimmed: &str, span: Span) -> Option<EnumVariantDecl> {
    let name = parse_symbol_name(trimmed)?;
    let tail = trimmed[name.len()..].trim();
    let payload = if tail.is_empty() {
        None
    } else if tail.starts_with('(') && tail.ends_with(')') {
        Some(tail[1..tail.len() - 1].trim().to_string())
    } else {
        None
    };
    Some(EnumVariantDecl {
        name,
        payload,
        span,
    })
}

fn parse_trait_assoc_type_decl(trimmed: &str, span: Span) -> Option<TraitAssocTypeDecl> {
    let rest = trimmed.strip_prefix("type ")?;
    let (name, default_ty) = match rest.split_once('=') {
        Some((name, value)) => (name.trim(), Some(value.trim().to_string())),
        None => (rest.trim(), None),
    };
    if !is_identifier(name) {
        return None;
    }
    Some(TraitAssocTypeDecl {
        name: name.to_string(),
        default_ty: default_ty.filter(|value| !value.is_empty()),
        span,
    })
}

fn parse_impl_assoc_type_binding(trimmed: &str, span: Span) -> Option<ImplAssocTypeBinding> {
    let rest = trimmed.strip_prefix("type ")?;
    let (name, value_ty) = match rest.split_once('=') {
        Some((name, value)) => (name.trim(), Some(value.trim().to_string())),
        None => (rest.trim(), None),
    };
    if !is_identifier(name) {
        return None;
    }
    Some(ImplAssocTypeBinding {
        name: name.to_string(),
        value_ty: value_ty.filter(|value| !value.is_empty()),
        span,
    })
}

fn split_top_level(source: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut start = 0usize;

    for (idx, ch) in source.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if ch == separator && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
            parts.push(&source[start..idx]);
            start = idx + ch.len_utf8();
        }
    }

    parts.push(&source[start..]);
    parts
}

fn find_matching_delim(source: &str, open_idx: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in source.char_indices().skip_while(|(idx, _)| *idx < open_idx) {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn parse_symbol_name(rest: &str) -> Option<String> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !is_identifier_start(first) {
        return None;
    }

    let mut name = String::new();
    name.push(first);
    for ch in chars {
        if !is_identifier_continue(ch) {
            break;
        }
        name.push(ch);
    }
    Some(name)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_identifier_start(first) {
        return false;
    }
    chars.all(is_identifier_continue)
}

fn is_path_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.split('.').all(is_identifier)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::freeze::{FROZEN_AST_NODE_KINDS, FROZEN_TOKEN_KINDS};
    use super::{
        BinaryOp, DirectiveKind, Expr, MatchPattern, ParamMode, PhraseArg, StatementKind,
        SymbolBody, SymbolKind, UnaryOp, parse_module,
    };

    #[test]
    fn frozen_lists_are_unique() {
        let mut tokens = FROZEN_TOKEN_KINDS.to_vec();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), FROZEN_TOKEN_KINDS.len());

        let mut nodes = FROZEN_AST_NODE_KINDS.to_vec();
        nodes.sort_unstable();
        nodes.dedup();
        assert_eq!(nodes.len(), FROZEN_AST_NODE_KINDS.len());
    }

    #[test]
    fn parse_module_rejects_tabs() {
        let err = parse_module("fn main()\n\treturn 0\n").expect_err("expected tab rejection");
        assert!(err.contains("tabs are not allowed"));
    }

    #[test]
    fn parse_module_collects_directives_and_symbols() {
        let parsed = parse_module(
            "import std.io\nuse std.result.Result\nreexport types\nexport record Counter:\n    value: Int\nexport enum Result[T]:\n    Ok(Int)\n    Err(Str)\nexport trait CounterOps[T]:\n    type Output\n    fn tick(edit self: T) -> Int:\n        return 0\nfn main() -> Int:\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.directives.len(), 3);
        assert_eq!(parsed.directives[0].kind, DirectiveKind::Import);
        assert_eq!(parsed.directives[0].path, ["std", "io"]);
        assert_eq!(parsed.directives[1].kind, DirectiveKind::Use);
        assert_eq!(parsed.directives[1].path, ["std", "result", "Result"]);
        assert_eq!(parsed.symbols.len(), 4);
        assert_eq!(parsed.symbols[0].name, "Counter");
        assert_eq!(parsed.symbols[0].kind.as_str(), "record");
        assert!(parsed.symbols[0].exported);
        assert_eq!(
            parsed.symbols[0].surface_text,
            "record Counter:\nvalue: Int"
        );
        assert_eq!(parsed.symbols[0].type_params, Vec::<String>::new());
        match &parsed.symbols[0].body {
            SymbolBody::Record { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "value");
                assert_eq!(fields[0].ty, "Int");
            }
            other => panic!("expected record body, got {other:?}"),
        }
        assert_eq!(parsed.symbols[1].name, "Result");
        match &parsed.symbols[1].body {
            SymbolBody::Enum { variants } => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Ok");
                assert_eq!(variants[0].payload, Some("Int".to_string()));
                assert_eq!(variants[1].payload, Some("Str".to_string()));
            }
            other => panic!("expected enum body, got {other:?}"),
        }
        assert_eq!(parsed.symbols[2].name, "CounterOps");
        match &parsed.symbols[2].body {
            SymbolBody::Trait {
                assoc_types,
                methods,
            } => {
                assert_eq!(assoc_types.len(), 1);
                assert_eq!(assoc_types[0].name, "Output");
                assert_eq!(methods.len(), 1);
                assert_eq!(methods[0].name, "tick");
            }
            other => panic!("expected trait body, got {other:?}"),
        }
        assert_eq!(parsed.symbols[3].name, "main");
        assert_eq!(parsed.symbols[3].kind.as_str(), "fn");
        assert!(!parsed.symbols[3].exported);
        assert_eq!(parsed.symbols[3].surface_text, "fn main() -> Int:");
        assert_eq!(parsed.symbols[3].return_type, Some("Int".to_string()));
        assert!(parsed.symbols[3].statements.is_empty());
    }

    #[test]
    fn parse_module_collects_async_functions_and_impls() {
        let parsed = parse_module(
            "async fn worker[T, where std.iter.Iterator[T]](read it: T, count: Int) -> Int:\n    return count\nbehavior[phase=update, affinity=worker] fn tick():\n    return 0\nimpl std.iter.Iterator[T] for RangeIter:\n    type Item = Int\n    fn next(edit self: RangeIter) -> (Bool, Int):\n        return (false, 0)\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.symbols.len(), 2);
        let worker = &parsed.symbols[0];
        assert!(worker.is_async);
        assert_eq!(worker.type_params, vec!["T".to_string()]);
        assert_eq!(
            worker.where_clause,
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(worker.params.len(), 2);
        assert_eq!(worker.params[0].mode, Some(ParamMode::Read));
        assert_eq!(worker.params[0].name, "it");
        assert_eq!(worker.params[0].ty, "T");
        assert_eq!(worker.params[1].mode, None);
        assert_eq!(worker.return_type, Some("Int".to_string()));
        let tick = &parsed.symbols[1];
        assert_eq!(tick.kind, SymbolKind::Behavior);
        assert_eq!(tick.name, "tick");
        assert_eq!(tick.behavior_attrs.len(), 2);
        assert_eq!(tick.behavior_attrs[0].name, "phase");
        assert_eq!(tick.behavior_attrs[0].value, "update");
        assert_eq!(tick.behavior_attrs[1].value, "worker");

        assert_eq!(parsed.impls.len(), 1);
        let impl_decl = &parsed.impls[0];
        assert_eq!(
            impl_decl.trait_path,
            Some("std.iter.Iterator[T]".to_string())
        );
        assert_eq!(impl_decl.target_type, "RangeIter");
        assert_eq!(impl_decl.body_entries.len(), 2);
        assert!(impl_decl.body_entries[0].starts_with("type Item"));
        assert_eq!(impl_decl.assoc_types.len(), 1);
        assert_eq!(impl_decl.assoc_types[0].name, "Item");
        assert_eq!(impl_decl.assoc_types[0].value_ty, Some("Int".to_string()));
        assert_eq!(impl_decl.methods.len(), 1);
        assert_eq!(impl_decl.methods[0].name, "next");
    }

    #[test]
    fn parse_module_collects_structured_statements() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let mut frames = 0\n    while frames < 10:\n        if frames % 2 == 0:\n            frames += 1\n        else:\n            continue\n    return match frames:\n        10 => 1\n        _ => 0\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        assert_eq!(statements.len(), 3);
        match &statements[0].kind {
            StatementKind::Let {
                mutable,
                name,
                value,
            } => {
                assert!(*mutable);
                assert_eq!(name, "frames");
                assert!(matches!(
                    value,
                    Expr::Opaque { text, attached } if text == "0" && attached.is_empty()
                ));
            }
            other => panic!("expected let statement, got {other:?}"),
        }
        match &statements[1].kind {
            StatementKind::While { condition, body } => {
                match condition {
                    Expr::Binary { left, op, right } => {
                        assert_eq!(*op, BinaryOp::Lt);
                        assert!(matches!(
                            left.as_ref(),
                            Expr::Opaque { text, attached }
                                if text == "frames" && attached.is_empty()
                        ));
                        assert!(matches!(
                            right.as_ref(),
                            Expr::Opaque { text, attached } if text == "10" && attached.is_empty()
                        ));
                    }
                    other => panic!("expected binary while condition, got {other:?}"),
                }
                assert_eq!(body.len(), 1);
                match &body[0].kind {
                    StatementKind::If {
                        condition,
                        then_branch,
                        else_branch,
                    } => {
                        match condition {
                            Expr::Binary { left, op, right } => {
                                assert_eq!(*op, BinaryOp::EqEq);
                                match left.as_ref() {
                                    Expr::Binary { left, op, right } => {
                                        assert_eq!(*op, BinaryOp::Mod);
                                        assert!(matches!(
                                            left.as_ref(),
                                            Expr::Opaque { text, attached }
                                                if text == "frames" && attached.is_empty()
                                        ));
                                        assert!(matches!(
                                            right.as_ref(),
                                            Expr::Opaque { text, attached }
                                                if text == "2" && attached.is_empty()
                                        ));
                                    }
                                    other => panic!(
                                        "expected modulo expression in if condition, got {other:?}"
                                    ),
                                }
                                assert!(matches!(
                                    right.as_ref(),
                                    Expr::Opaque { text, attached }
                                        if text == "0" && attached.is_empty()
                                ));
                            }
                            other => panic!("expected equality if condition, got {other:?}"),
                        }
                        assert_eq!(then_branch.len(), 1);
                        assert!(matches!(then_branch[0].kind, StatementKind::Assign { .. }));
                        let else_branch = else_branch.as_ref().expect("else branch should exist");
                        assert_eq!(else_branch.len(), 1);
                        assert!(matches!(else_branch[0].kind, StatementKind::Continue));
                    }
                    other => panic!("expected nested if statement, got {other:?}"),
                }
            }
            other => panic!("expected while statement, got {other:?}"),
        }
        match &statements[2].kind {
            StatementKind::Return { value } => {
                match value.as_ref().expect("return should carry a value") {
                    Expr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            Expr::Opaque { text, attached }
                                if text == "frames" && attached.is_empty()
                        ));
                        assert_eq!(arms.len(), 2);
                        assert_eq!(
                            arms[0].patterns,
                            vec![MatchPattern::Literal {
                                text: "10".to_string()
                            }]
                        );
                        assert!(matches!(
                            arms[0].value,
                            Expr::Opaque { ref text, ref attached }
                                if text == "1" && attached.is_empty()
                        ));
                        assert_eq!(arms[1].patterns, vec![MatchPattern::Wildcard]);
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_match_expressions() {
        let parsed = parse_module(
            "fn score(t: Token) -> Int:\n    return match t:\n        Token.Plus | Token.Minus => 1\n        Token.IntLit(v) => v\nfn main() -> Int:\n    let out = score :: Token.Minus :: call\n    let v = match out:\n        0 => 0\n        _ => 1\n    return v\n",
        )
        .expect("parse should pass");

        match &parsed.symbols[0].statements[0].kind {
            StatementKind::Return { value } => match value.as_ref().expect("match return expected")
            {
                Expr::Match { subject, arms } => {
                    assert!(matches!(
                        subject.as_ref(),
                        Expr::Opaque { text, attached } if text == "t" && attached.is_empty()
                    ));
                    assert_eq!(
                        arms[0].patterns,
                        vec![
                            MatchPattern::Name {
                                text: "Token.Plus".to_string()
                            },
                            MatchPattern::Name {
                                text: "Token.Minus".to_string()
                            }
                        ]
                    );
                    assert_eq!(
                        arms[1].patterns,
                        vec![MatchPattern::Variant {
                            path: "Token.IntLit".to_string(),
                            args: vec![MatchPattern::Name {
                                text: "v".to_string()
                            }]
                        }]
                    );
                }
                other => panic!("expected match expression, got {other:?}"),
            },
            other => panic!("expected return statement, got {other:?}"),
        }

        match &parsed.symbols[1].statements[1].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "v");
                match value {
                    Expr::Match { subject, arms } => {
                        assert!(matches!(
                            subject.as_ref(),
                            Expr::Opaque { text, attached } if text == "out" && attached.is_empty()
                        ));
                        assert_eq!(arms.len(), 2);
                        assert_eq!(
                            arms[0].patterns,
                            vec![MatchPattern::Literal {
                                text: "0".to_string()
                            }]
                        );
                        assert_eq!(arms[1].patterns, vec![MatchPattern::Wildcard]);
                    }
                    other => panic!("expected match expression, got {other:?}"),
                }
            }
            other => panic!("expected let statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_rejects_break_outside_loop() {
        let err = parse_module("fn main() -> Int:\n    break\n").expect_err("break should fail");
        assert!(err.contains("`break` is only valid inside loops"), "{err}");
    }

    #[test]
    fn parse_module_rejects_stray_else() {
        let err = parse_module("fn main() -> Int:\n    else:\n        return 0\n")
            .expect_err("else should fail");
        assert!(err.contains("`else` without a preceding `if`"), "{err}");
    }

    #[test]
    fn parse_module_collects_structured_phrases_and_operators() {
        let parsed = parse_module(
            "fn main() -> Int:\n    defer io.print[Str] :: \"bye\" :: call\n    let task = weave worker :: 41 :: call\n    let ready = task >> await\n    let ok = not false and ((1 + 2) << 3) >= 8\n    let cfg = winspell.loop.FrameConfig :: clear = 0 :: call\n    let printed = io.print[Int] :: ready, ok :: call\n    return printed\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        assert_eq!(statements.len(), 7);

        match &statements[0].kind {
            StatementKind::Defer { expr } => match expr {
                Expr::QualifiedPhrase {
                    subject,
                    args,
                    qualifier,
                    attached,
                } => {
                    assert_eq!(qualifier, "call");
                    assert!(attached.is_empty());
                    assert!(matches!(
                        subject.as_ref(),
                        Expr::MemberAccess { member, .. } if member == "print[Str]"
                    ));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(
                        &args[0],
                        PhraseArg::Positional(Expr::Opaque { text, attached })
                            if text == "\"bye\"" && attached.is_empty()
                    ));
                }
                other => panic!("expected defer phrase expression, got {other:?}"),
            },
            other => panic!("expected defer statement, got {other:?}"),
        }

        match &statements[1].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "task");
                match value {
                    Expr::Unary { op, expr } => {
                        assert_eq!(*op, UnaryOp::Weave);
                        assert!(matches!(
                            expr.as_ref(),
                            Expr::QualifiedPhrase { qualifier, .. } if qualifier == "call"
                        ));
                    }
                    other => panic!("expected weave unary expression, got {other:?}"),
                }
            }
            other => panic!("expected let task statement, got {other:?}"),
        }

        match &statements[2].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "ready");
                match value {
                    Expr::Await { expr } => {
                        assert!(matches!(
                            expr.as_ref(),
                            Expr::Opaque { text, attached } if text == "task" && attached.is_empty()
                        ));
                    }
                    other => panic!("expected await expression, got {other:?}"),
                }
            }
            other => panic!("expected let ready statement, got {other:?}"),
        }

        match &statements[3].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "ok");
                match value {
                    Expr::Binary { left, op, right } => {
                        assert_eq!(*op, BinaryOp::And);
                        assert!(matches!(
                            left.as_ref(),
                            Expr::Unary {
                                op: UnaryOp::Not,
                                ..
                            }
                        ));
                        match right.as_ref() {
                            Expr::Binary { left, op, right } => {
                                assert_eq!(*op, BinaryOp::GtEq);
                                match left.as_ref() {
                                    Expr::Binary { left, op, right } => {
                                        assert_eq!(*op, BinaryOp::Shl);
                                        match left.as_ref() {
                                            Expr::Binary { op, .. } => {
                                                assert_eq!(*op, BinaryOp::Add);
                                            }
                                            other => panic!(
                                                "expected additive lhs in shift expression, got {other:?}"
                                            ),
                                        }
                                        assert!(matches!(
                                            right.as_ref(),
                                            Expr::Opaque { text, attached }
                                                if text == "3" && attached.is_empty()
                                        ));
                                    }
                                    other => panic!(
                                        "expected shift expression in comparison lhs, got {other:?}"
                                    ),
                                }
                                assert!(matches!(
                                    right.as_ref(),
                                    Expr::Opaque { text, attached }
                                        if text == "8" && attached.is_empty()
                                ));
                            }
                            other => panic!(
                                "expected comparison expression on rhs of logical and, got {other:?}"
                            ),
                        }
                    }
                    other => panic!("expected structured boolean expression, got {other:?}"),
                }
            }
            other => panic!("expected let ok statement, got {other:?}"),
        }

        match &statements[4].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "cfg");
                match value {
                    Expr::QualifiedPhrase {
                        subject,
                        args,
                        qualifier,
                        attached,
                    } => {
                        assert_eq!(qualifier, "call");
                        assert!(attached.is_empty());
                        assert!(matches!(
                            subject.as_ref(),
                            Expr::MemberAccess { member, .. } if member == "FrameConfig"
                        ));
                        assert_eq!(args.len(), 1);
                        assert!(matches!(
                            &args[0],
                            PhraseArg::Named { name, value: Expr::Opaque { text, attached } }
                                if name == "clear" && text == "0" && attached.is_empty()
                        ));
                    }
                    other => panic!("expected named-arg phrase, got {other:?}"),
                }
            }
            other => panic!("expected let cfg statement, got {other:?}"),
        }

        match &statements[5].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "printed");
                match value {
                    Expr::QualifiedPhrase {
                        subject,
                        args,
                        qualifier,
                        attached,
                    } => {
                        assert_eq!(qualifier, "call");
                        assert!(attached.is_empty());
                        assert!(matches!(
                            subject.as_ref(),
                            Expr::MemberAccess { member, .. } if member == "print[Int]"
                        ));
                        assert_eq!(args.len(), 2);
                    }
                    other => panic!("expected print phrase, got {other:?}"),
                }
            }
            other => panic!("expected let printed statement, got {other:?}"),
        }

        match &statements[6].kind {
            StatementKind::Return { value } => {
                assert!(matches!(
                    value.as_ref().expect("return should have value"),
                    Expr::Opaque { text, attached } if text == "printed" && attached.is_empty()
                ));
            }
            other => panic!("expected return statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_collects_access_and_range_expressions() {
        let parsed = parse_module(
            "fn main() -> Int:\n    let tuple_head = pair.0\n    let color = spec.color\n    let xs = [1, 2, 3, 4]\n    let first = xs[0]\n    let tail = xs[1..]\n    let mid = xs[1..=2]\n    let whole = xs[..]\n    let r1 = 0..3\n    let r2 = ..=3\n    return first\n",
        )
        .expect("parse should pass");

        let statements = &parsed.symbols[0].statements;
        match &statements[0].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "tuple_head");
                assert!(matches!(
                    value,
                    Expr::MemberAccess { member, .. } if member == "0"
                ));
            }
            other => panic!("expected tuple_head let, got {other:?}"),
        }
        match &statements[1].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "color");
                assert!(matches!(
                    value,
                    Expr::MemberAccess { member, .. } if member == "color"
                ));
            }
            other => panic!("expected color let, got {other:?}"),
        }
        match &statements[3].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "first");
                match value {
                    Expr::Index { expr, index } => {
                        assert!(matches!(
                            expr.as_ref(),
                            Expr::Opaque { text, attached } if text == "xs" && attached.is_empty()
                        ));
                        assert!(matches!(
                            index.as_ref(),
                            Expr::Opaque { text, attached } if text == "0" && attached.is_empty()
                        ));
                    }
                    other => panic!("expected index expression, got {other:?}"),
                }
            }
            other => panic!("expected first let, got {other:?}"),
        }
        match &statements[4].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "tail");
                match value {
                    Expr::Slice {
                        start,
                        end,
                        inclusive_end,
                        ..
                    } => {
                        assert!(!inclusive_end);
                        assert!(matches!(
                            start.as_deref(),
                            Some(Expr::Opaque { text, attached })
                                if text == "1" && attached.is_empty()
                        ));
                        assert!(end.is_none());
                    }
                    other => panic!("expected tail slice, got {other:?}"),
                }
            }
            other => panic!("expected tail let, got {other:?}"),
        }
        match &statements[5].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "mid");
                match value {
                    Expr::Slice {
                        start,
                        end,
                        inclusive_end,
                        ..
                    } => {
                        assert!(*inclusive_end);
                        assert!(matches!(
                            start.as_deref(),
                            Some(Expr::Opaque { text, attached })
                                if text == "1" && attached.is_empty()
                        ));
                        assert!(matches!(
                            end.as_deref(),
                            Some(Expr::Opaque { text, attached })
                                if text == "2" && attached.is_empty()
                        ));
                    }
                    other => panic!("expected mid slice, got {other:?}"),
                }
            }
            other => panic!("expected mid let, got {other:?}"),
        }
        match &statements[6].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "whole");
                assert!(matches!(
                    value,
                    Expr::Slice {
                        start: None,
                        end: None,
                        inclusive_end: false,
                        ..
                    }
                ));
            }
            other => panic!("expected whole let, got {other:?}"),
        }
        match &statements[7].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "r1");
                assert!(matches!(
                    value,
                    Expr::Range {
                        start: Some(_),
                        end: Some(_),
                        inclusive_end: false
                    }
                ));
            }
            other => panic!("expected r1 let, got {other:?}"),
        }
        match &statements[8].kind {
            StatementKind::Let { name, value, .. } => {
                assert_eq!(name, "r2");
                assert!(matches!(
                    value,
                    Expr::Range {
                        start: None,
                        end: Some(_),
                        inclusive_end: true
                    }
                ));
            }
            other => panic!("expected r2 let, got {other:?}"),
        }
    }

    #[test]
    fn parse_module_rejects_match_without_arms() {
        let err = parse_module("fn main() -> Int:\n    return match value:\n")
            .expect_err("match should fail");
        assert!(err.contains("malformed `match` expression"), "{err}");
    }
}
