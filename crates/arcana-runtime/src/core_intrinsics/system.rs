use super::*;
use crate::runtime_intrinsics::SystemIntrinsic as RuntimeIntrinsic;

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
        RuntimeIntrinsic::IoPrint => {
            let value = expect_single_arg(args, "print")?;
            host.print(&runtime_value_to_string(&value))?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoEprint => {
            let value = expect_single_arg(args, "eprint")?;
            host.eprint(&runtime_value_to_string(&value))?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoFlushStdout => {
            if !args.is_empty() {
                return Err("flush_stdout expects zero arguments".to_string());
            }
            host.flush_stdout()?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoFlushStderr => {
            if !args.is_empty() {
                return Err("flush_stderr expects zero arguments".to_string());
            }
            host.flush_stderr()?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoStdinReadLineTry => {
            if !args.is_empty() {
                return Err("stdin_read_line expects zero arguments".to_string());
            }
            Ok(match host.stdin_read_line() {
                Ok(line) => ok_variant(RuntimeValue::Str(line)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::PackageAssetRootTry => {
            if !args.is_empty() {
                return Err("package_asset_root expects zero arguments".to_string());
            }
            let package_id = current_package_id.unwrap_or(&plan.package_id);
            Ok(match runtime_current_package_asset_root(package_id) {
                Ok(path) => ok_variant(RuntimeValue::Str(runtime_path_string(&path))),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::StdTimeMonotonicNowMs => {
            if !args.is_empty() {
                return Err("monotonic_now_ms expects zero arguments".to_string());
            }
            Ok(std_types_core_monotonic_time_ms_record(
                host.monotonic_now_ms()?,
            ))
        }
        RuntimeIntrinsic::TimeMonotonicNowMs => {
            if !args.is_empty() {
                return Err("monotonic_now_ms expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(host.monotonic_now_ms()?))
        }
        RuntimeIntrinsic::TimeMonotonicNowNs => {
            if !args.is_empty() {
                return Err("monotonic_now_ns expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(host.monotonic_now_ns()?))
        }
    }
}
