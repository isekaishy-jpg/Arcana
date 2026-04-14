use crate::runtime_intrinsics::RuntimeIntrinsic;

mod collections;
mod concurrent;
mod ecs;
mod memory;
mod system;
mod text;

pub(super) fn resolve_runtime_intrinsic_path(callable_path: &[String]) -> Option<RuntimeIntrinsic> {
    let parts = callable_path.iter().map(String::as_str).collect::<Vec<_>>();
    system::resolve_path(parts.as_slice())
        .or_else(|| concurrent::resolve_path(parts.as_slice()))
        .or_else(|| memory::resolve_path(parts.as_slice()))
        .or_else(|| text::resolve_path(parts.as_slice()))
        .or_else(|| collections::resolve_path(parts.as_slice()))
        .or_else(|| ecs::resolve_path(parts.as_slice()))
}

pub(super) fn resolve_runtime_intrinsic_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    system::resolve_impl(intrinsic_impl)
        .or_else(|| concurrent::resolve_impl(intrinsic_impl))
        .or_else(|| memory::resolve_impl(intrinsic_impl))
        .or_else(|| text::resolve_impl(intrinsic_impl))
        .or_else(|| collections::resolve_impl(intrinsic_impl))
        .or_else(|| ecs::resolve_impl(intrinsic_impl))
}
