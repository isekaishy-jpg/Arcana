pub use arcana_runtime::{
    NativeProcessHost, RuntimeAbiCallOutcome, RuntimeAbiValue, RuntimeHost, RuntimePackagePlan,
    execute_entrypoint_routine, execute_exported_abi_routine, parse_runtime_package_image,
    plan_from_artifact, render_runtime_package_image,
};

pub fn current_process_runtime_host() -> Result<Box<dyn RuntimeHost>, String> {
    #[cfg(windows)]
    {
        return Ok(Box::new(NativeProcessHost::current()?));
    }

    #[cfg(not(windows))]
    {
        arcana_runtime::current_process_runtime_host()
    }
}
