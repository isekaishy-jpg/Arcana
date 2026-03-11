use arcana_hir::{
    HirModuleSummary, HirResolvedModule, HirResolvedTarget, HirSymbol, HirSymbolKind,
    HirWorkspacePackage, HirWorkspaceSummary,
};

use super::TypeScope;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceSymbolUse {
    TypeLike,
    Trait,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SurfaceRefs {
    pub(crate) paths: Vec<Vec<String>>,
    pub(crate) lifetimes: Vec<String>,
}

pub(crate) struct ResolvedSymbolRef<'a> {
    pub(crate) package_name: &'a str,
    pub(crate) module_id: &'a str,
    pub(crate) symbol: &'a HirSymbol,
}

enum ParsedSurfaceToken {
    Text(String),
    Lifetime(String),
    Path(Vec<String>),
}

struct ParsedSurface {
    tokens: Vec<ParsedSurfaceToken>,
    refs: SurfaceRefs,
}

pub(crate) fn lookup_symbol_path<'a>(
    workspace: &'a HirWorkspaceSummary,
    module: &'a HirResolvedModule,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return module
            .bindings
            .get(&path[0])
            .and_then(|binding| lookup_target_symbol_tail(workspace, &binding.target, &[]));
    }

    let first = &path[0];
    if let Some(binding) = module.bindings.get(first) {
        return lookup_target_symbol_tail(workspace, &binding.target, &path[1..]);
    }

    if let Some(package) = workspace.package(first) {
        return lookup_package_symbol_path(package, &path[1..]);
    }

    let package_name = module.module_id.split('.').next()?;
    let package = workspace.package(package_name)?;
    lookup_package_symbol_path(package, path)
}

pub(crate) fn symbol_matches_surface_use(
    kind: HirSymbolKind,
    expected_use: SurfaceSymbolUse,
) -> bool {
    match expected_use {
        SurfaceSymbolUse::TypeLike => {
            matches!(
                kind,
                HirSymbolKind::Record | HirSymbolKind::Enum | HirSymbolKind::Trait
            )
        }
        SurfaceSymbolUse::Trait => kind == HirSymbolKind::Trait,
    }
}

pub(crate) fn surface_use_name(expected_use: SurfaceSymbolUse) -> &'static str {
    match expected_use {
        SurfaceSymbolUse::TypeLike => "type",
        SurfaceSymbolUse::Trait => "trait",
    }
}

pub(crate) fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Int"
            | "Str"
            | "Bool"
            | "RangeInt"
            | "List"
            | "Array"
            | "Map"
            | "Arena"
            | "ArenaId"
            | "FrameArena"
            | "FrameId"
            | "PoolArena"
            | "PoolId"
            | "Task"
            | "Thread"
            | "Channel"
            | "Mutex"
            | "AtomicInt"
            | "AtomicBool"
            | "Window"
            | "Image"
            | "FileStream"
            | "AudioDevice"
            | "AudioBuffer"
            | "AudioPlayback"
    )
}

pub(crate) fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('.') {
        let segment = segment.trim();
        if segment.is_empty() {
            return None;
        }
        let mut chars = segment.chars();
        let first = chars.next()?;
        if !is_ident_start(first) || !chars.all(is_ident_continue) {
            return None;
        }
        segments.push(segment.to_string());
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

pub(crate) fn canonicalize_surface_text(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    text: &str,
) -> String {
    let parsed = parse_surface_text(text);
    let mut out = String::new();
    for token in parsed.tokens {
        match token {
            ParsedSurfaceToken::Text(text) | ParsedSurfaceToken::Lifetime(text) => {
                out.push_str(&text);
            }
            ParsedSurfaceToken::Path(path) => out.push_str(&canonicalize_surface_path(
                workspace,
                resolved_module,
                scope,
                &path,
            )),
        }
    }
    out
}

pub(crate) fn canonicalize_surface_path(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    path: &[String],
) -> String {
    if path.len() == 1 && (scope.allows_type_name(&path[0]) || is_builtin_type_name(&path[0])) {
        return path[0].clone();
    }
    if let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, path) {
        return format!("{}.{}", symbol_ref.module_id, symbol_ref.symbol.name);
    }
    path.join(".")
}

pub(crate) fn collect_surface_refs(text: &str) -> SurfaceRefs {
    parse_surface_text(text).refs
}

pub(crate) fn surface_text_is_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    text: &str,
) -> bool {
    let refs = collect_surface_refs(text);
    if refs.paths.is_empty() {
        return true;
    }
    for path in refs.paths {
        if path.len() == 1 && (scope.allows_type_name(&path[0]) || is_builtin_type_name(&path[0])) {
            continue;
        }
        let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path) else {
            return false;
        };
        if symbol_ref.package_name == package.summary.package_name && !symbol_ref.symbol.exported {
            return false;
        }
    }
    true
}

fn lookup_target_symbol_tail<'a>(
    workspace: &'a HirWorkspaceSummary,
    target: &'a HirResolvedTarget,
    tail: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    match target {
        HirResolvedTarget::Symbol {
            package_name,
            module_id,
            symbol_name,
        } => {
            if !tail.is_empty() {
                return None;
            }
            let package = workspace.package(package_name)?;
            let module = package.module(module_id)?;
            let symbol = module
                .symbols
                .iter()
                .find(|symbol| symbol.name == *symbol_name)?;
            Some(ResolvedSymbolRef {
                package_name,
                module_id,
                symbol,
            })
        }
        HirResolvedTarget::Module {
            package_name,
            module_id,
        } => {
            let package = workspace.package(package_name)?;
            let module = package.module(module_id)?;
            lookup_module_symbol_path(package, module, tail)
        }
    }
}

fn lookup_package_symbol_path<'a>(
    package: &'a HirWorkspacePackage,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    let (symbol_name, module_path) = path.split_last()?;
    if symbol_name.is_empty() {
        return None;
    }
    let module = if module_path.is_empty() {
        package.module(&package.summary.package_name)
    } else {
        package.resolve_relative_module(module_path)
    }?;
    let symbol = module
        .symbols
        .iter()
        .find(|symbol| symbol.name == *symbol_name)?;
    Some(ResolvedSymbolRef {
        package_name: &package.summary.package_name,
        module_id: &module.module_id,
        symbol,
    })
}

fn lookup_module_symbol_path<'a>(
    package: &'a HirWorkspacePackage,
    module: &'a HirModuleSummary,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        let symbol = module
            .symbols
            .iter()
            .find(|symbol| symbol.name == path[0])?;
        return Some(ResolvedSymbolRef {
            package_name: &package.summary.package_name,
            module_id: &module.module_id,
            symbol,
        });
    }
    let (symbol_name, module_tail) = path.split_last()?;
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut target_relative = base_relative;
    target_relative.extend_from_slice(module_tail);
    let target_module = package.resolve_relative_module(&target_relative)?;
    let symbol = target_module
        .symbols
        .iter()
        .find(|symbol| symbol.name == *symbol_name)?;
    Some(ResolvedSymbolRef {
        package_name: &package.summary.package_name,
        module_id: &target_module.module_id,
        symbol,
    })
}

fn parse_surface_text(text: &str) -> ParsedSurface {
    let chars = text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut refs = SurfaceRefs::default();
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '\'' {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            let lifetime = chars[start..index].iter().collect::<String>();
            tokens.push(ParsedSurfaceToken::Lifetime(lifetime.clone()));
            refs.lifetimes.push(lifetime);
            continue;
        }
        if is_ident_start(ch) && is_projection_tail(&chars, index) {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            tokens.push(ParsedSurfaceToken::Text(
                chars[start..index].iter().collect::<String>(),
            ));
            continue;
        }
        if is_ident_start(ch) {
            let (end, token) = parse_surface_ident(&chars, index);
            match token {
                ParsedSurfaceToken::Path(path) => {
                    refs.paths.push(path.clone());
                    tokens.push(ParsedSurfaceToken::Path(path));
                }
                token => tokens.push(token),
            }
            index = end;
            continue;
        }
        tokens.push(ParsedSurfaceToken::Text(ch.to_string()));
        index += 1;
    }

    ParsedSurface { tokens, refs }
}

fn parse_surface_ident(chars: &[char], start: usize) -> (usize, ParsedSurfaceToken) {
    let mut end = start;
    let mut segments = Vec::new();
    let mut keyword = None::<String>;
    loop {
        let segment_start = end;
        end += 1;
        while end < chars.len() && is_ident_continue(chars[end]) {
            end += 1;
        }
        let segment = chars[segment_start..end].iter().collect::<String>();
        if is_surface_keyword(&segment) {
            keyword = Some(segment);
            segments.clear();
            break;
        }
        segments.push(segment);

        let Some(dot_idx) = next_non_ws_index(chars, end) else {
            break;
        };
        if chars[dot_idx] != '.' {
            break;
        }
        let Some(next_idx) = next_non_ws_index(chars, dot_idx + 1) else {
            break;
        };
        if !is_ident_start(chars[next_idx]) {
            break;
        }
        end = next_idx;
    }

    if let Some(keyword) = keyword {
        return (end, ParsedSurfaceToken::Text(keyword));
    }
    if !segments.is_empty() {
        return (end, ParsedSurfaceToken::Path(segments));
    }
    (
        end,
        ParsedSurfaceToken::Text(chars[start..end].iter().collect::<String>()),
    )
}

fn is_projection_tail(chars: &[char], index: usize) -> bool {
    let Some(dot_idx) = previous_non_ws_index(chars, index) else {
        return false;
    };
    if chars[dot_idx] != '.' {
        return false;
    }
    let Some(owner_idx) = previous_non_ws_index(chars, dot_idx) else {
        return false;
    };
    matches!(chars[owner_idx], ']' | ')')
}

fn previous_non_ws_index(chars: &[char], before: usize) -> Option<usize> {
    let mut index = before;
    while index > 0 {
        index -= 1;
        if !chars[index].is_whitespace() {
            return Some(index);
        }
    }
    None
}

fn next_non_ws_index(chars: &[char], start: usize) -> Option<usize> {
    let mut index = start;
    while index < chars.len() {
        if !chars[index].is_whitespace() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_surface_keyword(token: &str) -> bool {
    matches!(token, "mut" | "where")
}
