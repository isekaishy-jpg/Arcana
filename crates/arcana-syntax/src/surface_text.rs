use crate::type_surface::SurfaceRefs;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceTextToken {
    Text(String),
    Lifetime(String),
    Path(Vec<String>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedSurfaceText {
    pub tokens: Vec<SurfaceTextToken>,
    pub refs: SurfaceRefs,
}

pub fn parse_surface_text(text: &str) -> ParsedSurfaceText {
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
            tokens.push(SurfaceTextToken::Lifetime(lifetime.clone()));
            refs.lifetimes.push(lifetime);
            continue;
        }
        if is_ident_start(ch) && is_projection_tail(&chars, index) {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            tokens.push(SurfaceTextToken::Text(
                chars[start..index].iter().collect::<String>(),
            ));
            continue;
        }
        if is_ident_start(ch) {
            let (end, token) = parse_surface_ident(&chars, index);
            match token {
                SurfaceTextToken::Path(path) => {
                    refs.paths.push(path.clone());
                    tokens.push(SurfaceTextToken::Path(path));
                }
                token => tokens.push(token),
            }
            index = end;
            continue;
        }
        tokens.push(SurfaceTextToken::Text(ch.to_string()));
        index += 1;
    }

    ParsedSurfaceText { tokens, refs }
}

fn parse_surface_ident(chars: &[char], start: usize) -> (usize, SurfaceTextToken) {
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
        return (end, SurfaceTextToken::Text(keyword));
    }
    if !segments.is_empty() {
        return (end, SurfaceTextToken::Path(segments));
    }
    (
        end,
        SurfaceTextToken::Text(chars[start..end].iter().collect::<String>()),
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

fn next_non_ws_index(chars: &[char], mut index: usize) -> Option<usize> {
    while index < chars.len() {
        if !chars[index].is_whitespace() {
            return Some(index);
        }
        index += 1;
    }
    None
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

fn is_surface_keyword(segment: &str) -> bool {
    matches!(segment, "mut" | "read" | "take" | "edit" | "where")
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
