use super::*;

mod collections;
mod concurrent;
mod ecs;
mod memory;
mod system;
mod text;

pub(super) fn execute_runtime_core_intrinsic(
    intrinsic: RuntimeIntrinsic,
    type_args: &[String],
    final_args: &mut Vec<RuntimeValue>,
    plan: &RuntimePackagePlan,
    scopes: Option<&mut Vec<RuntimeScope>>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    match intrinsic {
        RuntimeIntrinsic::System(intrinsic) => system::execute(
            intrinsic,
            type_args,
            final_args,
            plan,
            scopes,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        RuntimeIntrinsic::Concurrent(intrinsic) => concurrent::execute(
            intrinsic,
            type_args,
            final_args,
            plan,
            scopes,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        RuntimeIntrinsic::Memory(intrinsic) => memory::execute(
            intrinsic,
            type_args,
            final_args,
            plan,
            scopes,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        RuntimeIntrinsic::Text(intrinsic) => text::execute(
            intrinsic,
            type_args,
            final_args,
            plan,
            scopes,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        RuntimeIntrinsic::Collections(intrinsic) => collections::execute(
            intrinsic,
            type_args,
            final_args,
            plan,
            scopes,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        RuntimeIntrinsic::Ecs(intrinsic) => ecs::execute(
            intrinsic,
            type_args,
            final_args,
            plan,
            scopes,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
    }
}
