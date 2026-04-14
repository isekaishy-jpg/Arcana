use super::*;
use crate::runtime_intrinsics::EcsIntrinsic as RuntimeIntrinsic;

#[allow(unused_variables)]
pub(super) fn execute(
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
    let args = final_args.clone();
    match intrinsic {
        RuntimeIntrinsic::EcsSetSingleton => {
            let key = require_runtime_type_key(type_args, "ecs_set_singleton")?;
            let value = expect_single_arg(args, "ecs_set_singleton")?;
            ecs_slot_mut(state, &key).insert(0, value);
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::EcsHasSingleton => {
            let key = require_runtime_type_key(type_args, "ecs_has_singleton")?;
            if !args.is_empty() {
                return Err("ecs_has_singleton expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Bool(ecs_slot(state, &key, 0).is_some()))
        }
        RuntimeIntrinsic::EcsGetSingleton => {
            let key = require_runtime_type_key(type_args, "ecs_get_singleton")?;
            if !args.is_empty() {
                return Err("ecs_get_singleton expects zero arguments".to_string());
            }
            ecs_slot(state, &key, 0)
                .cloned()
                .ok_or_else(|| format!("missing singleton component for `{}`", key.join(", ")))
        }
        RuntimeIntrinsic::EcsSpawn => {
            if !args.is_empty() {
                return Err("ecs_spawn expects zero arguments".to_string());
            }
            let entity = if state.next_entity_id <= 0 {
                1
            } else {
                state.next_entity_id
            };
            state.next_entity_id = entity + 1;
            state.live_entities.insert(entity);
            Ok(RuntimeValue::Int(entity))
        }
        RuntimeIntrinsic::EcsDespawn => {
            let entity = expect_entity_id(expect_single_arg(args, "ecs_despawn")?, "ecs_despawn")?;
            if entity == 0 {
                return Err("ecs_despawn cannot target singleton entity 0".to_string());
            }
            if !state.live_entities.remove(&entity) {
                return Err(format!("ecs_despawn unknown entity `{entity}`"));
            }
            for slots in state.component_slots.values_mut() {
                slots.remove(&entity);
            }
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::EcsSetComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_set_component_at")?;
            if args.len() != 2 {
                return Err("ecs_set_component_at expects two arguments".to_string());
            }
            let entity = expect_entity_id(args[0].clone(), "ecs_set_component_at")?;
            if !ecs_entity_exists(state, entity) {
                return Err(format!("ecs_set_component_at unknown entity `{entity}`"));
            }
            ecs_slot_mut(state, &key).insert(entity, args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::EcsHasComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_has_component_at")?;
            let entity = expect_entity_id(
                expect_single_arg(args, "ecs_has_component_at")?,
                "ecs_has_component_at",
            )?;
            if !ecs_entity_exists(state, entity) {
                return Ok(RuntimeValue::Bool(false));
            }
            Ok(RuntimeValue::Bool(ecs_slot(state, &key, entity).is_some()))
        }
        RuntimeIntrinsic::EcsGetComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_get_component_at")?;
            let entity = expect_entity_id(
                expect_single_arg(args, "ecs_get_component_at")?,
                "ecs_get_component_at",
            )?;
            if !ecs_entity_exists(state, entity) {
                return Err(format!("ecs_get_component_at unknown entity `{entity}`"));
            }
            ecs_slot(state, &key, entity).cloned().ok_or_else(|| {
                format!(
                    "missing component `{}` at entity `{entity}`",
                    key.join(", ")
                )
            })
        }
        RuntimeIntrinsic::EcsRemoveComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_remove_component_at")?;
            let entity = expect_entity_id(
                expect_single_arg(args, "ecs_remove_component_at")?,
                "ecs_remove_component_at",
            )?;
            if !ecs_entity_exists(state, entity) {
                return Err(format!("ecs_remove_component_at unknown entity `{entity}`"));
            }
            ecs_slot_mut(state, &key).remove(&entity);
            Ok(RuntimeValue::Unit)
        }
    }
}
