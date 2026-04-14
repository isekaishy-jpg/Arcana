use crate::runtime_intrinsics::RuntimeIntrinsic;

pub(super) fn resolve_path(parts: &[&str]) -> Option<RuntimeIntrinsic> {
    match parts {
        ["std", "kernel", "ecs", "set_singleton"] => Some(RuntimeIntrinsic::EcsSetSingleton),
        ["std", "kernel", "ecs", "has_singleton"] => Some(RuntimeIntrinsic::EcsHasSingleton),
        ["std", "kernel", "ecs", "get_singleton"] => Some(RuntimeIntrinsic::EcsGetSingleton),
        ["std", "kernel", "ecs", "spawn"] => Some(RuntimeIntrinsic::EcsSpawn),
        ["std", "kernel", "ecs", "despawn"] => Some(RuntimeIntrinsic::EcsDespawn),
        ["std", "kernel", "ecs", "set_component_at"] => Some(RuntimeIntrinsic::EcsSetComponentAt),
        ["std", "kernel", "ecs", "has_component_at"] => Some(RuntimeIntrinsic::EcsHasComponentAt),
        ["std", "kernel", "ecs", "get_component_at"] => Some(RuntimeIntrinsic::EcsGetComponentAt),
        ["std", "kernel", "ecs", "remove_component_at"] => {
            Some(RuntimeIntrinsic::EcsRemoveComponentAt)
        }
        _ => None,
    }
}

pub(super) fn resolve_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "EcsSetSingleton" => Some(RuntimeIntrinsic::EcsSetSingleton),
        "EcsHasSingleton" => Some(RuntimeIntrinsic::EcsHasSingleton),
        "EcsGetSingleton" => Some(RuntimeIntrinsic::EcsGetSingleton),
        "EcsSpawn" => Some(RuntimeIntrinsic::EcsSpawn),
        "EcsDespawn" => Some(RuntimeIntrinsic::EcsDespawn),
        "EcsSetComponentAt" => Some(RuntimeIntrinsic::EcsSetComponentAt),
        "EcsHasComponentAt" => Some(RuntimeIntrinsic::EcsHasComponentAt),
        "EcsGetComponentAt" => Some(RuntimeIntrinsic::EcsGetComponentAt),
        "EcsRemoveComponentAt" => Some(RuntimeIntrinsic::EcsRemoveComponentAt),
        _ => None,
    }
}
