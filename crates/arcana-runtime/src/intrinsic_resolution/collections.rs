use crate::runtime_intrinsics::RuntimeIntrinsic;

pub(super) fn resolve_path(parts: &[&str]) -> Option<RuntimeIntrinsic> {
    match parts {
        ["std", "collections", "list", "new"] | ["std", "kernel", "collections", "list_new"] => {
            Some(RuntimeIntrinsic::ListNew)
        }
        ["std", "collections", "list", "len"] | ["std", "kernel", "collections", "list_len"] => {
            Some(RuntimeIntrinsic::ListLen)
        }
        ["std", "collections", "list", "is_empty"] => Some(RuntimeIntrinsic::ListIsEmpty),
        ["std", "collections", "list", "push"] | ["std", "kernel", "collections", "list_push"] => {
            Some(RuntimeIntrinsic::ListPush)
        }
        ["std", "collections", "list", "pop"] | ["std", "kernel", "collections", "list_pop"] => {
            Some(RuntimeIntrinsic::ListPop)
        }
        ["std", "collections", "list", "try_pop_or"]
        | ["std", "kernel", "collections", "list_try_pop_or"] => {
            Some(RuntimeIntrinsic::ListTryPopOr)
        }
        ["std", "kernel", "collections", "array_new"] => Some(RuntimeIntrinsic::ArrayNew),
        ["std", "kernel", "collections", "array_len"] => Some(RuntimeIntrinsic::ArrayLen),
        ["std", "kernel", "collections", "array_from_list"] => {
            Some(RuntimeIntrinsic::ArrayFromList)
        }
        ["std", "kernel", "collections", "array_to_list"] => Some(RuntimeIntrinsic::ArrayToList),
        ["std", "kernel", "collections", "map_new"] => Some(RuntimeIntrinsic::MapNew),
        ["std", "kernel", "collections", "map_len"] => Some(RuntimeIntrinsic::MapLen),
        ["std", "kernel", "collections", "map_has"] => Some(RuntimeIntrinsic::MapHas),
        ["std", "kernel", "collections", "map_get"] => Some(RuntimeIntrinsic::MapGet),
        ["std", "kernel", "collections", "map_set"] => Some(RuntimeIntrinsic::MapSet),
        ["std", "kernel", "collections", "map_remove"] => Some(RuntimeIntrinsic::MapRemove),
        ["std", "kernel", "collections", "map_try_get_or"] => Some(RuntimeIntrinsic::MapTryGetOr),
        ["std", "option", "is_some"] => Some(RuntimeIntrinsic::OptionIsSome),
        ["std", "option", "is_none"] => Some(RuntimeIntrinsic::OptionIsNone),
        ["std", "option", "unwrap_or"] => Some(RuntimeIntrinsic::OptionUnwrapOr),
        ["Result", "Ok"] | ["std", "result", "Result", "Ok"] => Some(RuntimeIntrinsic::ResultOk),
        ["Result", "Err"] | ["std", "result", "Result", "Err"] => Some(RuntimeIntrinsic::ResultErr),
        ["std", "result", "is_ok"] => Some(RuntimeIntrinsic::ResultIsOk),
        ["std", "result", "is_err"] => Some(RuntimeIntrinsic::ResultIsErr),
        ["std", "result", "unwrap_or"] => Some(RuntimeIntrinsic::ResultUnwrapOr),
        _ => None,
    }
}

pub(super) fn resolve_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "ListNew" => Some(RuntimeIntrinsic::ListNew),
        "ListLen" => Some(RuntimeIntrinsic::ListLen),
        "ListPush" => Some(RuntimeIntrinsic::ListPush),
        "ListPop" => Some(RuntimeIntrinsic::ListPop),
        "ListTryPopOr" => Some(RuntimeIntrinsic::ListTryPopOr),
        "ArrayNew" => Some(RuntimeIntrinsic::ArrayNew),
        "ArrayLen" => Some(RuntimeIntrinsic::ArrayLen),
        "ArrayFromList" => Some(RuntimeIntrinsic::ArrayFromList),
        "ArrayToList" => Some(RuntimeIntrinsic::ArrayToList),
        "MapNew" => Some(RuntimeIntrinsic::MapNew),
        "MapLen" => Some(RuntimeIntrinsic::MapLen),
        "MapHas" => Some(RuntimeIntrinsic::MapHas),
        "MapGet" => Some(RuntimeIntrinsic::MapGet),
        "MapSet" => Some(RuntimeIntrinsic::MapSet),
        "MapRemove" => Some(RuntimeIntrinsic::MapRemove),
        "MapTryGetOr" => Some(RuntimeIntrinsic::MapTryGetOr),
        _ => None,
    }
}
