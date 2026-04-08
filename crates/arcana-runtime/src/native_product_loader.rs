use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString, c_char, c_void};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_cabi::{
    ARCANA_CABI_CONTRACT_VERSION_V1, ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL,
    ArcanaCabiBindingCallback, ArcanaCabiBindingCallbackEntryV1, ArcanaCabiBindingImport,
    ArcanaCabiBindingImportEntryV1, ArcanaCabiBindingImportFn, ArcanaCabiBindingOpsV1,
    ArcanaCabiBindingRegisterCallbackFn, ArcanaCabiBindingSignature,
    ArcanaCabiBindingSignatureKind, ArcanaCabiBindingUnregisterCallbackFn,
    ArcanaCabiBindingValueV1, ArcanaCabiChildOpsV1, ArcanaCabiChildRunEntrypointFn,
    ArcanaCabiCreateInstanceFn, ArcanaCabiDestroyInstanceFn, ArcanaCabiExportParam,
    ArcanaCabiInstanceOpsV1, ArcanaCabiLastErrorAllocFn, ArcanaCabiOwnedBytesFreeFn,
    ArcanaCabiOwnedStrFreeFn, ArcanaCabiParamSourceMode, ArcanaCabiPassMode,
    ArcanaCabiPluginDescribeInstanceFn, ArcanaCabiPluginOpsV1, ArcanaCabiPluginUseInstanceFn,
    ArcanaCabiProductApiV1, ArcanaCabiProductRole, ArcanaCabiType, binding_write_back_slots,
    compare_binding_signatures, release_binding_output_value, validate_binding_callbacks,
    validate_binding_imports, validate_binding_write_backs,
};
use serde::Deserialize;

const DISTRIBUTION_BUNDLE_FORMAT: &str = "arcana-distribution-bundle-v2";
const DISTRIBUTION_BUNDLE_FORMAT_V1: &str = "arcana-distribution-bundle-v1";
const DISTRIBUTION_MANIFEST_FILE: &str = "arcana.bundle.toml";
const NATIVE_PRODUCT_TEMP_PROBES_ENV: &str = "ARCANA_NATIVE_PRODUCT_TEMP_PROBES";
const EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC: &[u8] = b"ARCANA_DIST_MANIFEST_V2\0";
const EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC_V1: &[u8] = b"ARCANA_DIST_MANIFEST_V1\0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeNativeProductInfo {
    pub package_id: String,
    pub package_name: String,
    pub product_name: String,
    pub role: ArcanaCabiProductRole,
    pub contract_id: String,
    pub contract_version: u32,
    pub file: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeChildBindingInfo {
    pub consumer_member: String,
    pub dependency_alias: String,
    pub package_id: String,
    pub package_name: String,
    pub product_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeNativePluginHandle(u64);

pub struct RuntimeNativeProductCatalog {
    bundle_dir: PathBuf,
    root_member: Option<String>,
    products: Vec<RuntimeNativeProductInfo>,
    child_bindings: Vec<RuntimeChildBindingInfo>,
    runtime_child_binding: Option<RuntimeChildBindingInfo>,
    package_assets: BTreeMap<String, String>,
    next_plugin_handle: u64,
    #[cfg(windows)]
    loaded_children: BTreeMap<(String, String), LoadedNativeLibrary>,
    #[cfg(windows)]
    active_child_bindings: BTreeMap<(String, String), ActiveChildBinding>,
    #[cfg(windows)]
    active_bindings: BTreeMap<(String, String), ActiveBindingProduct>,
    #[cfg(windows)]
    open_plugins: BTreeMap<RuntimeNativePluginHandle, OpenPluginInstance>,
}

impl RuntimeNativeProductCatalog {
    fn empty(bundle_dir: PathBuf) -> Self {
        Self {
            bundle_dir,
            root_member: None,
            products: Vec::new(),
            child_bindings: Vec::new(),
            runtime_child_binding: None,
            package_assets: BTreeMap::new(),
            next_plugin_handle: 1,
            #[cfg(windows)]
            loaded_children: BTreeMap::new(),
            #[cfg(windows)]
            active_child_bindings: BTreeMap::new(),
            #[cfg(windows)]
            active_bindings: BTreeMap::new(),
            #[cfg(windows)]
            open_plugins: BTreeMap::new(),
        }
    }

    pub fn bundle_dir(&self) -> &Path {
        &self.bundle_dir
    }

    pub fn root_member(&self) -> Option<&str> {
        self.root_member.as_deref()
    }

    pub fn products(&self) -> &[RuntimeNativeProductInfo] {
        &self.products
    }

    pub fn child_bindings(&self) -> &[RuntimeChildBindingInfo] {
        &self.child_bindings
    }

    pub fn runtime_child_binding(&self) -> Option<&RuntimeChildBindingInfo> {
        self.runtime_child_binding.as_ref()
    }

    pub fn package_asset_root(&self, package_id: &str) -> Option<&str> {
        self.package_assets.get(package_id).map(String::as_str)
    }

    pub fn package_asset_roots(&self) -> &BTreeMap<String, String> {
        &self.package_assets
    }

    pub fn plugin_products(&self) -> Vec<&RuntimeNativeProductInfo> {
        self.products
            .iter()
            .filter(|product| product.role == ArcanaCabiProductRole::Plugin)
            .collect()
    }

    pub fn plugin_products_for_contract(
        &self,
        contract_id: &str,
    ) -> Vec<&RuntimeNativeProductInfo> {
        self.products
            .iter()
            .filter(|product| {
                product.role == ArcanaCabiProductRole::Plugin && product.contract_id == contract_id
            })
            .collect()
    }

    #[cfg(windows)]
    pub fn active_child_binding_count(&self) -> usize {
        self.active_child_bindings.len()
    }

    #[cfg(not(windows))]
    pub fn active_child_binding_count(&self) -> usize {
        0
    }

    #[cfg(windows)]
    pub fn open_plugin_count(&self) -> usize {
        self.open_plugins.len()
    }

    #[cfg(not(windows))]
    pub fn open_plugin_count(&self) -> usize {
        0
    }

    pub fn run_child_entrypoint(
        &mut self,
        package_image_text: &str,
        main_routine_key: &str,
    ) -> Result<Option<i32>, String> {
        #[cfg(windows)]
        {
            if self.active_child_bindings.is_empty() {
                return Ok(None);
            }
            if let Some(binding) = &self.runtime_child_binding {
                let binding_key = (
                    binding.consumer_member.clone(),
                    binding.dependency_alias.clone(),
                );
                let binding = self.active_child_bindings.get(&binding_key).ok_or_else(|| {
                    format!(
                        "bundle root `{}` selected runtime child binding `{}:{}` but it is not active",
                        self.root_member.as_deref().unwrap_or("<unknown>"),
                        binding.consumer_member,
                        binding.dependency_alias
                    )
                })?;
                return run_active_child_binding_entrypoint(
                    binding,
                    package_image_text,
                    main_routine_key,
                )
                .map(Some);
            }
            let candidate_bindings = self
                .active_child_bindings
                .iter()
                .filter(|((consumer, _alias), _binding)| {
                    self.root_member
                        .as_deref()
                        .map(|root| consumer == root)
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();
            if candidate_bindings.is_empty() {
                native_product_probe(
                    "root_child_runtime_provider_missing",
                    format!(
                        "root_member={} active_bindings={}",
                        self.root_member.as_deref().unwrap_or("<unknown>"),
                        self.active_child_bindings.len()
                    ),
                );
                return Ok(None);
            }
            if candidate_bindings.len() != 1 {
                let bindings = candidate_bindings
                    .iter()
                    .map(|((consumer, alias), binding)| {
                        format!(
                            "{}:{}=>{}:{}",
                            consumer,
                            alias,
                            binding.product.package_id,
                            binding.product.product_name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                native_product_probe(
                    "ambiguous_child_runtime_provider",
                    format!(
                        "root_member={} bindings={bindings}",
                        self.root_member.as_deref().unwrap_or("<unknown>")
                    ),
                );
                return Err(format!(
                    "bundle root `{}` has multiple active child bindings and runtime provider selection is ambiguous: {bindings}",
                    self.root_member.as_deref().unwrap_or("<unknown>")
                ));
            }
            let ((_binding_key_consumer, _binding_key_alias), binding) = candidate_bindings[0];
            run_active_child_binding_entrypoint(binding, package_image_text, main_routine_key)
                .map(Some)
        }

        #[cfg(not(windows))]
        {
            let _ = (package_image_text, main_routine_key);
            Ok(None)
        }
    }

    pub fn activate_children(&mut self) -> Result<(), String> {
        #[cfg(windows)]
        {
            for binding in self.child_bindings.clone() {
                let binding_key = (
                    binding.consumer_member.clone(),
                    binding.dependency_alias.clone(),
                );
                if self.active_child_bindings.contains_key(&binding_key) {
                    native_product_probe(
                        "duplicate_child_binding_activation",
                        format!(
                            "consumer={} alias={} package={} product={}",
                            binding.consumer_member,
                            binding.dependency_alias,
                            binding.package_id,
                            binding.product_name
                        ),
                    );
                    continue;
                }
                let product = self
                    .find_product_by_id(&binding.package_id, &binding.product_name)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "child binding `{}` -> `{}` references missing native product `{}:{}`",
                            binding.consumer_member,
                            binding.dependency_alias,
                            binding.package_id,
                            binding.product_name
                        )
                    })?;
                if product.role != ArcanaCabiProductRole::Child {
                    return Err(format!(
                        "child binding `{}` -> `{}` references `{}`:`{}` with role `{}` instead of `child`",
                        binding.consumer_member,
                        binding.dependency_alias,
                        binding.package_id,
                        binding.product_name,
                        product.role.as_str()
                    ));
                }
                let product_key = (product.package_id.clone(), product.product_name.clone());
                if !self.loaded_children.contains_key(&product_key) {
                    let library = LoadedNativeLibrary::load(&self.bundle_dir, &product)?;
                    self.loaded_children.insert(product_key.clone(), library);
                }
                let library = self.loaded_children.get(&product_key).ok_or_else(|| {
                    format!(
                        "loaded child library entry missing for `{}:{}`",
                        product.package_id, product.product_name
                    )
                })?;
                let instance = library.create_instance()?;
                native_product_probe(
                    "activate_child_binding",
                    format!(
                        "consumer={} alias={} package={} product={} file={}",
                        binding.consumer_member,
                        binding.dependency_alias,
                        product.package_id,
                        product.product_name,
                        product.file
                    ),
                );
                self.active_child_bindings.insert(
                    binding_key,
                    ActiveChildBinding {
                        product,
                        instance: ActiveNativeInstance {
                            instance,
                            destroy_instance: library.destroy_instance,
                        },
                        run_entrypoint: library.child_run_entrypoint.ok_or_else(|| {
                            format!(
                                "native child product `{}:{}` is missing `run_entrypoint` ops",
                                library.package_name, library.product_name
                            )
                        })?,
                        last_error_alloc: library.last_error_alloc.ok_or_else(|| {
                            format!(
                                "native child product `{}:{}` is missing `last_error_alloc` ops",
                                library.package_name, library.product_name
                            )
                        })?,
                        owned_bytes_free: library.owned_bytes_free.ok_or_else(|| {
                            format!(
                                "native child product `{}:{}` is missing `owned_bytes_free` ops",
                                library.package_name, library.product_name
                            )
                        })?,
                    },
                );
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            if self
                .products
                .iter()
                .any(|product| product.role == ArcanaCabiProductRole::Child)
            {
                return Err("native child products currently require a Windows host".to_string());
            }
            Ok(())
        }
    }

    pub fn open_plugin(
        &mut self,
        package_name: &str,
        product_name: &str,
    ) -> Result<RuntimeNativePluginHandle, String> {
        #[cfg(windows)]
        {
            let matches = self
                .products
                .iter()
                .filter(|product| {
                    product.package_name == package_name && product.product_name == product_name
                })
                .collect::<Vec<_>>();
            let package_id = match matches.as_slice() {
                [] => {
                    return Err(format!(
                        "bundle does not declare native product `{package_name}:{product_name}`"
                    ));
                }
                [product] => product.package_id.clone(),
                _ => {
                    let candidates = matches
                        .iter()
                        .map(|product| format!("{}:{}", product.package_id, product.product_name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!(
                        "bundle declares multiple native products named `{package_name}:{product_name}`; select by package id instead: {candidates}"
                    ));
                }
            };
            self.open_plugin_by_package_id(&package_id, product_name)
        }

        #[cfg(not(windows))]
        {
            let _ = (package_name, product_name);
            Err("native plugin products currently require a Windows host".to_string())
        }
    }

    pub fn open_plugin_by_package_id(
        &mut self,
        package_id: &str,
        product_name: &str,
    ) -> Result<RuntimeNativePluginHandle, String> {
        #[cfg(windows)]
        {
            let product = self
                .find_product_by_id(package_id, product_name)
                .cloned()
                .ok_or_else(|| {
                    format!("bundle does not declare native product `{package_id}:{product_name}`")
                })?;
            if product.role != ArcanaCabiProductRole::Plugin {
                return Err(format!(
                    "native product `{package_id}:{product_name}` uses role `{}` instead of `plugin`",
                    product.role.as_str()
                ));
            }
            let library = LoadedNativeLibrary::load(&self.bundle_dir, &product)?;
            let instance = library.create_instance()?;
            let destroy_instance = library.destroy_instance;
            let describe_instance = library.plugin_describe_instance.ok_or_else(|| {
                format!(
                    "native plugin product `{package_id}:{product_name}` is missing `describe_instance` ops"
                )
            })?;
            let use_instance = library.plugin_use_instance.ok_or_else(|| {
                format!(
                    "native plugin product `{package_id}:{product_name}` is missing `use_instance` ops"
                )
            })?;
            let owned_bytes_free = library.owned_bytes_free.ok_or_else(|| {
                format!(
                    "native plugin product `{package_id}:{product_name}` is missing `owned_bytes_free` ops"
                )
            })?;
            let handle = RuntimeNativePluginHandle(self.next_plugin_handle);
            self.next_plugin_handle += 1;
            native_product_probe(
                "open_plugin",
                format!(
                    "handle={} package={} product={} contract={}",
                    handle.0, product.package_id, product.product_name, product.contract_id
                ),
            );
            self.open_plugins.insert(
                handle,
                OpenPluginInstance {
                    product,
                    active: ActiveNativeInstance {
                        instance,
                        destroy_instance,
                    },
                    describe_instance,
                    use_instance,
                    owned_bytes_free,
                    _library: library,
                },
            );
            Ok(handle)
        }

        #[cfg(not(windows))]
        {
            let _ = (package_id, product_name);
            Err("native plugin products currently require a Windows host".to_string())
        }
    }

    pub fn release_plugin(&mut self, handle: RuntimeNativePluginHandle) -> Result<(), String> {
        #[cfg(windows)]
        {
            let plugin = self
                .open_plugins
                .remove(&handle)
                .ok_or_else(|| format!("invalid RuntimeNativePluginHandle `{}`", handle.0))?;
            drop(plugin);
            native_product_probe("release_plugin", format!("handle={}", handle.0));
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = handle;
            Err("native plugin products currently require a Windows host".to_string())
        }
    }

    pub fn describe_open_plugin(
        &self,
        handle: RuntimeNativePluginHandle,
    ) -> Result<String, String> {
        #[cfg(windows)]
        {
            let plugin = self
                .open_plugins
                .get(&handle)
                .ok_or_else(|| format!("invalid RuntimeNativePluginHandle `{}`", handle.0))?;
            let description = read_plugin_description(plugin)?.ok_or_else(|| {
                format!(
                    "plugin `{}:{}` returned no description",
                    plugin.product.package_name, plugin.product.product_name
                )
            })?;
            native_product_probe(
                "describe_plugin",
                format!(
                    "handle={} package={} product={} description={}",
                    handle.0, plugin.product.package_name, plugin.product.product_name, description
                ),
            );
            Ok(description)
        }

        #[cfg(not(windows))]
        {
            let _ = handle;
            Err("native plugin products currently require a Windows host".to_string())
        }
    }

    pub fn use_open_plugin(
        &self,
        handle: RuntimeNativePluginHandle,
        request: &[u8],
    ) -> Result<Vec<u8>, String> {
        #[cfg(windows)]
        {
            let plugin = self
                .open_plugins
                .get(&handle)
                .ok_or_else(|| format!("invalid RuntimeNativePluginHandle `{}`", handle.0))?;
            let response = read_plugin_use_response(plugin, request)?;
            native_product_probe(
                "use_plugin",
                format!(
                    "handle={} package={} product={} request_len={} response_len={}",
                    handle.0,
                    plugin.product.package_name,
                    plugin.product.product_name,
                    request.len(),
                    response.len()
                ),
            );
            Ok(response)
        }

        #[cfg(not(windows))]
        {
            let _ = (handle, request);
            Err("native plugin products currently require a Windows host".to_string())
        }
    }

    pub(crate) fn invoke_binding_import(
        &mut self,
        package_id: &str,
        import_name: &str,
        callback_specs: &[RuntimeBindingCallbackRegistrationSpec],
        expected_imports: &[ArcanaCabiBindingSignature],
        expected_callbacks: &[ArcanaCabiBindingSignature],
        args: &[ArcanaCabiBindingValueV1],
    ) -> Result<RuntimeBindingImportOutcome, String> {
        #[cfg(windows)]
        {
            let binding = self.ensure_active_binding(
                package_id,
                callback_specs,
                expected_imports,
                expected_callbacks,
            )?;
            let import = binding.imports.get(import_name).ok_or_else(|| {
                format!(
                    "binding package `{}` has no native import `{}`",
                    binding.product.package_name, import_name
                )
            })?;
            if import.metadata.params.len() != args.len() {
                return Err(format!(
                    "binding import `{}:{}` expected {} arguments, got {}",
                    binding.product.package_name,
                    import_name,
                    import.metadata.params.len(),
                    args.len()
                ));
            }
            let mut out_write_backs = binding_write_back_slots(&import.metadata.params);
            let mut out_result = ArcanaCabiBindingValueV1::default();
            let ok = unsafe {
                (import.call)(
                    binding.active.instance,
                    args.as_ptr(),
                    args.len(),
                    out_write_backs.as_mut_ptr(),
                    &mut out_result,
                )
            };
            if ok == 0 {
                let err =
                    read_library_last_error(binding.last_error_alloc, binding.owned_bytes_free)
                        .unwrap_or_else(|| {
                            format!(
                                "binding import `{}:{}` failed without an error message",
                                binding.product.package_name, import_name
                            )
                        });
                native_product_probe(
                    "binding_import_error",
                    format!(
                        "package={} product={} import={} error={}",
                        binding.product.package_name,
                        binding.product.product_name,
                        import_name,
                        err
                    ),
                );
                return Err(err);
            }
            if let Err(err) =
                validate_binding_write_backs(&import.metadata.params, &out_write_backs)
            {
                let _ = release_binding_output_value(
                    out_result,
                    binding.owned_bytes_free,
                    binding.owned_str_free,
                );
                for value in out_write_backs {
                    let _ = release_binding_output_value(
                        value,
                        binding.owned_bytes_free,
                        binding.owned_str_free,
                    );
                }
                return Err(format!(
                    "binding import `{}:{}` returned invalid write-backs: {err}",
                    binding.product.package_name, import_name
                ));
            }
            native_product_probe(
                "binding_import_call",
                format!(
                    "package={} product={} import={} arg_count={}",
                    binding.product.package_name,
                    binding.product.product_name,
                    import_name,
                    args.len()
                ),
            );
            Ok(RuntimeBindingImportOutcome {
                result: out_result,
                write_backs: out_write_backs,
                owned_bytes_free: binding.owned_bytes_free,
                owned_str_free: binding.owned_str_free,
            })
        }

        #[cfg(not(windows))]
        {
            let _ = (
                package_id,
                import_name,
                callback_specs,
                expected_imports,
                expected_callbacks,
                args,
            );
            Err("native binding products currently require a Windows host".to_string())
        }
    }

    fn find_product_by_id(
        &self,
        package_id: &str,
        product_name: &str,
    ) -> Option<&RuntimeNativeProductInfo> {
        self.products.iter().find(|product| {
            product.package_id == package_id && product.product_name == product_name
        })
    }

    #[cfg(windows)]
    fn ensure_active_binding(
        &mut self,
        package_id: &str,
        callback_specs: &[RuntimeBindingCallbackRegistrationSpec],
        expected_imports: &[ArcanaCabiBindingSignature],
        expected_callbacks: &[ArcanaCabiBindingSignature],
    ) -> Result<&mut ActiveBindingProduct, String> {
        let matches = self
            .products
            .iter()
            .filter(|product| {
                product.package_id == package_id && product.role == ArcanaCabiProductRole::Binding
            })
            .cloned()
            .collect::<Vec<_>>();
        let product = match matches.as_slice() {
            [] => {
                return Err(format!(
                    "bundle does not declare a binding product for package `{package_id}`"
                ));
            }
            [product] => product.clone(),
            _ => {
                let products = matches
                    .iter()
                    .map(|product| product.product_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!(
                    "bundle declares multiple binding products for package `{package_id}`: {products}"
                ));
            }
        };
        let binding_key = (product.package_id.clone(), product.product_name.clone());
        if !self.active_bindings.contains_key(&binding_key) {
            let library = LoadedNativeLibrary::load(&self.bundle_dir, &product)?;
            compare_binding_signatures(
                ArcanaCabiBindingSignatureKind::Import,
                expected_imports,
                &library
                    .binding_imports
                    .iter()
                    .map(ArcanaCabiBindingImport::signature)
                    .collect::<Vec<_>>(),
            )?;
            compare_binding_signatures(
                ArcanaCabiBindingSignatureKind::Callback,
                expected_callbacks,
                &library
                    .binding_callbacks
                    .iter()
                    .map(ArcanaCabiBindingCallback::signature)
                    .collect::<Vec<_>>(),
            )?;
            let instance = library.create_instance()?;
            let register_callback = library.binding_register_callback.ok_or_else(|| {
                format!(
                    "native binding product `{}:{}` is missing `register_callback` ops",
                    product.package_name, product.product_name
                )
            })?;
            let unregister_callback = library.binding_unregister_callback.ok_or_else(|| {
                format!(
                    "native binding product `{}:{}` is missing `unregister_callback` ops",
                    product.package_name, product.product_name
                )
            })?;
            let last_error_alloc = library.last_error_alloc.ok_or_else(|| {
                format!(
                    "native binding product `{}:{}` is missing `last_error_alloc` ops",
                    product.package_name, product.product_name
                )
            })?;
            let owned_bytes_free = library.owned_bytes_free.ok_or_else(|| {
                format!(
                    "native binding product `{}:{}` is missing `owned_bytes_free` ops",
                    product.package_name, product.product_name
                )
            })?;
            let owned_str_free = library.owned_str_free.ok_or_else(|| {
                format!(
                    "native binding product `{}:{}` is missing `owned_str_free` ops",
                    product.package_name, product.product_name
                )
            })?;

            let imports = library
                .binding_imports
                .iter()
                .map(|metadata| {
                    let symbol_name =
                        CString::new(metadata.symbol_name.as_str()).map_err(|_| {
                            format!(
                                "binding import symbol `{}` contains an interior NUL byte",
                                metadata.symbol_name
                            )
                        })?;
                    let proc = unsafe {
                        windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                            library.module,
                            symbol_name.as_ptr().cast(),
                        )
                    };
                    let Some(proc) = proc else {
                        return Err(format!(
                            "binding import `{}` is missing symbol `{}` in `{}:{}`",
                            metadata.name,
                            metadata.symbol_name,
                            product.package_name,
                            product.product_name
                        ));
                    };
                    let call = unsafe {
                        std::mem::transmute::<
                            unsafe extern "system" fn() -> isize,
                            ArcanaCabiBindingImportFn,
                        >(proc)
                    };
                    Ok((
                        metadata.name.clone(),
                        ActiveBindingImport {
                            metadata: metadata.clone(),
                            call,
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;

            let mut callback_registrations = Vec::new();
            for spec in callback_specs {
                let callback_name = CString::new(spec.name).map_err(|_| {
                    format!(
                        "binding callback name `{}` contains an interior NUL byte",
                        spec.name
                    )
                })?;
                if !library
                    .binding_callbacks
                    .iter()
                    .any(|callback| callback.name == spec.name)
                {
                    return Err(format!(
                        "native binding product `{}:{}` does not declare callback `{}`",
                        product.package_name, product.product_name, spec.name
                    ));
                }
                let mut handle = 0u64;
                let ok = unsafe {
                    register_callback(
                        instance,
                        callback_name.as_ptr(),
                        spec.callback,
                        spec.owned_bytes_free,
                        spec.owned_str_free,
                        spec.user_data,
                        &mut handle,
                    )
                };
                if ok == 0 {
                    return Err(read_library_last_error(last_error_alloc, owned_bytes_free)
                        .unwrap_or_else(|| {
                            format!(
                                "binding callback registration failed for `{}:{}` callback `{}`",
                                product.package_name, product.product_name, spec.name
                            )
                        }));
                }
                callback_registrations.push(ActiveBindingCallbackRegistration {
                    handle,
                    user_data: spec.user_data,
                    cleanup_user_data: spec.cleanup_user_data,
                });
            }

            native_product_probe(
                "activate_binding",
                format!(
                    "package={} product={} callbacks={} imports={}",
                    product.package_name,
                    product.product_name,
                    callback_registrations.len(),
                    imports.len()
                ),
            );

            self.active_bindings.insert(
                binding_key.clone(),
                ActiveBindingProduct {
                    product,
                    active: ActiveNativeInstance {
                        instance,
                        destroy_instance: library.destroy_instance,
                    },
                    imports,
                    callback_registrations,
                    unregister_callback,
                    last_error_alloc,
                    owned_bytes_free,
                    owned_str_free,
                    _library: library,
                },
            );
        }
        self.active_bindings.get_mut(&binding_key).ok_or_else(|| {
            format!("active binding cache entry is missing for package `{package_id}`")
        })
    }
}

impl Drop for RuntimeNativeProductCatalog {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            self.open_plugins.clear();
            self.active_bindings.clear();
            self.active_child_bindings.clear();
            self.loaded_children.clear();
        }
    }
}

pub fn load_current_bundle_native_products() -> Result<RuntimeNativeProductCatalog, String> {
    let exe_path = std::env::current_exe().map_err(|e| {
        format!("failed to resolve current executable for native product loading: {e}")
    })?;
    let bundle_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    if let Some(text) = read_embedded_distribution_manifest(&exe_path)? {
        return load_bundle_native_products_from_text(
            bundle_dir,
            &exe_path.display().to_string(),
            &text,
        );
    }
    load_bundle_native_products(bundle_dir)
}

pub fn load_bundle_native_products_from_manifest_path(
    manifest_path: &Path,
) -> Result<RuntimeNativeProductCatalog, String> {
    let bundle_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let text = fs::read_to_string(manifest_path).map_err(|e| {
        format!(
            "failed to read distribution bundle manifest `{}`: {e}",
            manifest_path.display()
        )
    })?;
    load_bundle_native_products_from_text(bundle_dir, &manifest_path.display().to_string(), &text)
}

pub fn activate_current_bundle_native_products() -> Result<RuntimeNativeProductCatalog, String> {
    let mut catalog = load_current_bundle_native_products()?;
    catalog.activate_children()?;
    Ok(catalog)
}

pub fn load_bundle_native_products(
    bundle_dir: &Path,
) -> Result<RuntimeNativeProductCatalog, String> {
    let manifest_path = bundle_dir.join(DISTRIBUTION_MANIFEST_FILE);
    if manifest_path.is_file() {
        let text = fs::read_to_string(&manifest_path).map_err(|e| {
            format!(
                "failed to read distribution bundle manifest `{}`: {e}",
                manifest_path.display()
            )
        })?;
        return load_bundle_native_products_from_text(
            bundle_dir,
            &manifest_path.display().to_string(),
            &text,
        );
    }
    let mut embedded = Vec::new();
    for entry in fs::read_dir(bundle_dir).map_err(|e| {
        format!(
            "failed to read distribution bundle directory `{}`: {e}",
            bundle_dir.display()
        )
    })? {
        let entry = entry.map_err(|e| format!("failed to read bundle entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(text) = read_embedded_distribution_manifest(&path)? {
            embedded.push((path, text));
        }
    }
    match embedded.as_slice() {
        [(path, text)] => {
            load_bundle_native_products_from_text(bundle_dir, &path.display().to_string(), text)
        }
        [] => {
            native_product_probe(
                "bundle_manifest_missing",
                format!("bundle_dir={}", bundle_dir.display()),
            );
            Ok(RuntimeNativeProductCatalog::empty(bundle_dir.to_path_buf()))
        }
        _many => Err(format!(
            "bundle directory `{}` has multiple embedded distribution manifests; set `{}` explicitly",
            bundle_dir.display(),
            crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV
        )),
    }
}

fn load_bundle_native_products_from_text(
    bundle_dir: &Path,
    manifest_label: &str,
    text: &str,
) -> Result<RuntimeNativeProductCatalog, String> {
    let manifest = toml::from_str::<DistributionBundleManifest>(text).map_err(|e| {
        format!("failed to parse distribution bundle manifest `{manifest_label}`: {e}",)
    })?;
    let is_v1 = manifest.format == DISTRIBUTION_BUNDLE_FORMAT_V1;
    if manifest.format != DISTRIBUTION_BUNDLE_FORMAT && !is_v1 {
        return Err(format!(
            "unsupported distribution bundle format `{}` in `{manifest_label}`",
            manifest.format
        ));
    }

    let support_files = manifest
        .support_files
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let package_assets = manifest
        .package_assets
        .into_iter()
        .map(|asset| {
            let package_id = asset.package_id.ok_or_else(|| {
                format!(
                    "distribution bundle manifest `{manifest_label}` is missing `package_id` for package asset root `{}`",
                    asset.asset_root
                )
            })?;
            if asset.package_name.is_empty() {
                return Err(format!(
                    "distribution bundle manifest `{manifest_label}` is missing `package_name` for package asset root `{}`",
                    asset.asset_root
                ));
            }
            let has_files = support_files
                .iter()
                .any(|path| path == &asset.asset_root || path.starts_with(&format!("{}/", asset.asset_root)));
            if !has_files {
                return Err(format!(
                    "distribution bundle manifest `{manifest_label}` declares package asset root `{}` for `{}`, but no listed support file lives under that root",
                    asset.asset_root, package_id
                ));
            }
            Ok((package_id, asset.asset_root))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut products = manifest
        .native_products
        .into_iter()
        .map(|product| {
            let role = ArcanaCabiProductRole::parse(&product.role)?;
            if !support_files.contains(&product.file) {
                native_product_probe(
                    "bundle_manifest_product_missing_support_file",
                    format!(
                        "package={} product={} file={}",
                        product.package_id.as_deref().unwrap_or(&product.package_name),
                        product.product_name,
                        product.file
                    ),
                );
                return Err(format!(
                    "distribution bundle manifest `{}` declares native product `{}:{}` at `{}`, but that file is not listed in `support_files`",
                    manifest_label,
                    product.package_name,
                    product.product_name,
                    product.file
                ));
            }
            let package_name = product.package_name;
            let package_id = match product.package_id {
                Some(package_id) if !package_id.is_empty() => package_id,
                _ if is_v1 => package_name.clone(),
                _ => {
                    return Err(format!(
                        "distribution bundle manifest `{manifest_label}` is missing `package_id` for native product `{}:{}`",
                        package_name, product.product_name
                    ));
                }
            };
            Ok(RuntimeNativeProductInfo {
                package_id,
                package_name,
                product_name: product.product_name,
                role,
                contract_id: product.contract_id,
                contract_version: product
                    .contract_version
                    .unwrap_or(ARCANA_CABI_CONTRACT_VERSION_V1),
                file: product.file,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    products.sort_by(|left, right| {
        left.package_name
            .cmp(&right.package_name)
            .then_with(|| left.package_id.cmp(&right.package_id))
            .then_with(|| left.product_name.cmp(&right.product_name))
    });

    let child_bindings = manifest
        .child_bindings
        .into_iter()
        .map(|binding| {
            let package_name = binding.package_name;
            let package_id = match binding.package_id {
                Some(package_id) if !package_id.is_empty() => package_id,
                _ if is_v1 => package_name.clone(),
                _ => {
                    return Err(format!(
                        "distribution bundle manifest `{manifest_label}` is missing `package_id` for child binding `{}` -> `{}`",
                        binding.consumer_member, binding.dependency_alias
                    ));
                }
            };
            Ok(RuntimeChildBindingInfo {
                consumer_member: binding.consumer_member,
                dependency_alias: binding.dependency_alias,
                package_id,
                package_name,
                product_name: binding.product_name,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    for binding in &child_bindings {
        let product = products
            .iter()
            .find(|product| {
                product.package_id == binding.package_id
                    && product.product_name == binding.product_name
            })
            .ok_or_else(|| {
                format!(
                    "distribution bundle manifest `{}` references missing child product `{}:{}` for binding `{}` -> `{}`",
                    manifest_label,
                    binding.package_id,
                    binding.product_name,
                    binding.consumer_member,
                    binding.dependency_alias
                )
            })?;
        if product.package_name != binding.package_name {
            return Err(format!(
                "distribution bundle manifest `{}` binds package id `{}` as `{}`, but the matching native product declares package name `{}`",
                manifest_label, binding.package_id, binding.package_name, product.package_name
            ));
        }
        if product.role != ArcanaCabiProductRole::Child {
            return Err(format!(
                "distribution bundle manifest `{}` references `{}`:`{}` as a child binding, but its role is `{}`",
                manifest_label,
                binding.package_id,
                binding.product_name,
                product.role.as_str()
            ));
        }
    }
    let runtime_child_binding =
        manifest
            .runtime_child_binding
            .map(|binding| {
                let package_name = binding.package_name;
                let package_id = match binding.package_id {
                    Some(package_id) if !package_id.is_empty() => package_id,
                    _ if is_v1 => package_name.clone(),
                    _ => {
                        return Err(format!(
                            "distribution bundle manifest `{manifest_label}` is missing `package_id` for runtime child binding `{}` -> `{}`",
                            binding.consumer_member, binding.dependency_alias
                        ));
                    }
                };
                Ok(RuntimeChildBindingInfo {
                    consumer_member: binding.consumer_member,
                    dependency_alias: binding.dependency_alias,
                    package_id,
                    package_name,
                    product_name: binding.product_name,
                })
            })
            .transpose()?;
    if let Some(binding) = &runtime_child_binding {
        if manifest.member.as_deref() != Some(binding.consumer_member.as_str()) {
            return Err(format!(
                "distribution bundle manifest `{}` sets runtime child binding `{}:{}` for consumer `{}`, but bundle root member is `{}`",
                manifest_label,
                binding.package_id,
                binding.product_name,
                binding.consumer_member,
                manifest.member.as_deref().unwrap_or("<unknown>")
            ));
        }
        if !child_bindings.iter().any(|candidate| candidate == binding) {
            return Err(format!(
                "distribution bundle manifest `{}` sets runtime child binding `{}:{}` for `{}` -> `{}`, but that binding is not listed in `[[child_bindings]]`",
                manifest_label,
                binding.package_id,
                binding.product_name,
                binding.consumer_member,
                binding.dependency_alias
            ));
        }
        native_product_probe(
            "runtime_child_binding_loaded",
            format!(
                "consumer={} alias={} package={} product={}",
                binding.consumer_member,
                binding.dependency_alias,
                binding.package_id,
                binding.product_name
            ),
        );
    }

    native_product_probe(
        "bundle_manifest_loaded",
        format!(
            "bundle_dir={} root_member={} products={} child_bindings={} runtime_child_binding={}",
            bundle_dir.display(),
            manifest.member.as_deref().unwrap_or("<unknown>"),
            products.len(),
            child_bindings.len(),
            runtime_child_binding
                .as_ref()
                .map(|binding| format!(
                    "{}:{}=>{}:{}",
                    binding.consumer_member,
                    binding.dependency_alias,
                    binding.package_id,
                    binding.product_name
                ))
                .unwrap_or_else(|| "<none>".to_string())
        ),
    );

    Ok(RuntimeNativeProductCatalog {
        bundle_dir: bundle_dir.to_path_buf(),
        root_member: manifest.member,
        products,
        child_bindings,
        runtime_child_binding,
        package_assets,
        next_plugin_handle: 1,
        #[cfg(windows)]
        loaded_children: BTreeMap::new(),
        #[cfg(windows)]
        active_child_bindings: BTreeMap::new(),
        #[cfg(windows)]
        active_bindings: BTreeMap::new(),
        #[cfg(windows)]
        open_plugins: BTreeMap::new(),
    })
}

fn read_embedded_distribution_manifest(
    root_artifact_path: &Path,
) -> Result<Option<String>, String> {
    let bytes = fs::read(root_artifact_path).map_err(|e| {
        format!(
            "failed to read embedded distribution manifest from `{}`: {e}",
            root_artifact_path.display()
        )
    })?;
    let trailer_len = EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC.len() + std::mem::size_of::<u64>();
    if bytes.len() < trailer_len {
        return Ok(None);
    }
    let magic_start = if bytes.len() >= EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC.len()
        && &bytes[bytes.len() - EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC.len()..]
            == EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC
    {
        bytes.len() - EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC.len()
    } else if bytes.len() >= EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC_V1.len()
        && &bytes[bytes.len() - EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC_V1.len()..]
            == EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC_V1
    {
        bytes.len() - EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC_V1.len()
    } else {
        return Ok(None);
    };
    let len_start = magic_start - std::mem::size_of::<u64>();
    let payload_len = u64::from_le_bytes(
        bytes[len_start..magic_start]
            .try_into()
            .expect("embedded manifest trailer should contain a u64 length"),
    ) as usize;
    if len_start < payload_len {
        return Err(format!(
            "embedded distribution manifest in `{}` is truncated",
            root_artifact_path.display()
        ));
    }
    let payload_start = len_start - payload_len;
    let payload = std::str::from_utf8(&bytes[payload_start..len_start]).map_err(|e| {
        format!(
            "embedded distribution manifest in `{}` is not valid utf8: {e}",
            root_artifact_path.display()
        )
    })?;
    Ok(Some(payload.to_string()))
}

#[derive(Debug, Deserialize)]
struct DistributionBundleManifest {
    format: String,
    #[serde(default)]
    member: Option<String>,
    #[serde(default)]
    support_files: Vec<String>,
    #[serde(default)]
    package_assets: Vec<DistributionBundlePackageAsset>,
    #[serde(default)]
    native_products: Vec<DistributionBundleNativeProduct>,
    #[serde(default)]
    runtime_child_binding: Option<DistributionBundleChildBinding>,
    #[serde(default)]
    child_bindings: Vec<DistributionBundleChildBinding>,
}

#[derive(Debug, Deserialize)]
struct DistributionBundlePackageAsset {
    #[serde(default)]
    package_id: Option<String>,
    package_name: String,
    asset_root: String,
}

#[derive(Debug, Deserialize)]
struct DistributionBundleNativeProduct {
    #[serde(default)]
    package_id: Option<String>,
    package_name: String,
    product_name: String,
    role: String,
    contract_id: String,
    #[serde(default)]
    contract_version: Option<u32>,
    file: String,
}

#[derive(Debug, Deserialize)]
struct DistributionBundleChildBinding {
    consumer_member: String,
    dependency_alias: String,
    #[serde(default)]
    package_id: Option<String>,
    package_name: String,
    product_name: String,
}

#[cfg(windows)]
struct ActiveNativeInstance {
    instance: *mut c_void,
    destroy_instance: ArcanaCabiDestroyInstanceFn,
}

#[cfg(windows)]
struct ActiveChildBinding {
    product: RuntimeNativeProductInfo,
    instance: ActiveNativeInstance,
    run_entrypoint: ArcanaCabiChildRunEntrypointFn,
    last_error_alloc: ArcanaCabiLastErrorAllocFn,
    owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
}

#[cfg(windows)]
struct ActiveBindingProduct {
    product: RuntimeNativeProductInfo,
    active: ActiveNativeInstance,
    imports: BTreeMap<String, ActiveBindingImport>,
    callback_registrations: Vec<ActiveBindingCallbackRegistration>,
    unregister_callback: ArcanaCabiBindingUnregisterCallbackFn,
    last_error_alloc: ArcanaCabiLastErrorAllocFn,
    owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    owned_str_free: ArcanaCabiOwnedStrFreeFn,
    _library: LoadedNativeLibrary,
}

#[cfg(windows)]
#[derive(Clone, Copy)]
pub(crate) struct RuntimeBindingCallbackRegistrationSpec {
    pub name: &'static str,
    pub callback: arcana_cabi::ArcanaCabiBindingCallbackFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub owned_str_free: ArcanaCabiOwnedStrFreeFn,
    pub user_data: *mut c_void,
    pub cleanup_user_data: unsafe fn(*mut c_void),
}

#[cfg(windows)]
pub(crate) struct RuntimeBindingImportOutcome {
    pub result: ArcanaCabiBindingValueV1,
    pub write_backs: Vec<ArcanaCabiBindingValueV1>,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub owned_str_free: ArcanaCabiOwnedStrFreeFn,
}

#[cfg(windows)]
#[derive(Clone)]
struct ActiveBindingImport {
    metadata: ArcanaCabiBindingImport,
    call: ArcanaCabiBindingImportFn,
}

#[cfg(windows)]
struct ActiveBindingCallbackRegistration {
    handle: u64,
    user_data: *mut c_void,
    cleanup_user_data: unsafe fn(*mut c_void),
}

#[cfg(windows)]
impl Drop for ActiveBindingProduct {
    fn drop(&mut self) {
        for registration in self.callback_registrations.drain(..) {
            unsafe {
                (self.unregister_callback)(self.active.instance, registration.handle);
                (registration.cleanup_user_data)(registration.user_data);
            }
        }
    }
}

#[cfg(windows)]
impl Drop for ActiveNativeInstance {
    fn drop(&mut self) {
        if !self.instance.is_null() {
            unsafe {
                (self.destroy_instance)(self.instance);
            }
        }
    }
}

#[cfg(windows)]
struct OpenPluginInstance {
    product: RuntimeNativeProductInfo,
    active: ActiveNativeInstance,
    _library: LoadedNativeLibrary,
    describe_instance: ArcanaCabiPluginDescribeInstanceFn,
    use_instance: ArcanaCabiPluginUseInstanceFn,
    owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
}

#[cfg(windows)]
struct LoadedNativeLibrary {
    module: windows_sys::Win32::Foundation::HMODULE,
    package_name: String,
    product_name: String,
    destroy_instance: ArcanaCabiDestroyInstanceFn,
    create_instance: ArcanaCabiCreateInstanceFn,
    child_run_entrypoint: Option<ArcanaCabiChildRunEntrypointFn>,
    plugin_describe_instance: Option<ArcanaCabiPluginDescribeInstanceFn>,
    plugin_use_instance: Option<ArcanaCabiPluginUseInstanceFn>,
    binding_imports: Vec<ArcanaCabiBindingImport>,
    binding_callbacks: Vec<ArcanaCabiBindingCallback>,
    binding_register_callback: Option<ArcanaCabiBindingRegisterCallbackFn>,
    binding_unregister_callback: Option<ArcanaCabiBindingUnregisterCallbackFn>,
    last_error_alloc: Option<ArcanaCabiLastErrorAllocFn>,
    owned_bytes_free: Option<ArcanaCabiOwnedBytesFreeFn>,
    owned_str_free: Option<ArcanaCabiOwnedStrFreeFn>,
}

#[cfg(windows)]
impl LoadedNativeLibrary {
    fn load(bundle_dir: &Path, product: &RuntimeNativeProductInfo) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;

        let dll_path = bundle_dir.join(&product.file);
        if !dll_path.is_file() {
            native_product_probe(
                "native_product_file_missing",
                format!(
                    "package={} product={} path={}",
                    product.package_name,
                    product.product_name,
                    dll_path.display()
                ),
            );
            return Err(format!(
                "bundle-native product `{}:{}` is missing staged file `{}`",
                product.package_name,
                product.product_name,
                dll_path.display()
            ));
        }

        let wide = dll_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let module =
            unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(wide.as_ptr()) };
        if module.is_null() {
            return Err(format!(
                "failed to load native product library `{}`",
                dll_path.display()
            ));
        }

        let get_api_symbol = format!("{ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL}\0");
        let proc = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                module,
                get_api_symbol.as_ptr(),
            )
        };
        let proc = match proc {
            Some(proc) => proc,
            None => {
                unsafe {
                    windows_sys::Win32::Foundation::FreeLibrary(module);
                }
                return Err(format!(
                    "native product `{}` does not export `{ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL}`",
                    dll_path.display()
                ));
            }
        };

        let get_api: unsafe extern "system" fn() -> *const ArcanaCabiProductApiV1 =
            unsafe { std::mem::transmute(proc) };
        let api_ptr = unsafe { get_api() };
        let api = unsafe { api_ptr.as_ref() }.ok_or_else(|| {
            unsafe {
                windows_sys::Win32::Foundation::FreeLibrary(module);
            }
            format!(
                "native product `{}` returned a null product descriptor",
                dll_path.display()
            )
        })?;
        if api.descriptor_size < std::mem::size_of::<ArcanaCabiProductApiV1>() {
            unsafe {
                windows_sys::Win32::Foundation::FreeLibrary(module);
            }
            return Err(format!(
                "native product `{}` reported descriptor_size={} smaller than expected {}",
                dll_path.display(),
                api.descriptor_size,
                std::mem::size_of::<ArcanaCabiProductApiV1>()
            ));
        }

        let package_name = unsafe { read_cabi_utf8_field(api.package_name, "package_name") }?;
        let product_name = unsafe { read_cabi_utf8_field(api.product_name, "product_name") }?;
        let role_text = unsafe { read_cabi_utf8_field(api.role, "role") }?;
        let role = ArcanaCabiProductRole::parse(&role_text)?;
        let contract_id = unsafe { read_cabi_utf8_field(api.contract_id, "contract_id") }?;
        if package_name != product.package_name
            || product_name != product.product_name
            || role != product.role
            || contract_id != product.contract_id
        {
            native_product_probe(
                "descriptor_manifest_mismatch",
                format!(
                    "path={} manifest={}:{}:{}:{} descriptor={}:{}:{}:{}",
                    dll_path.display(),
                    product.package_name,
                    product.product_name,
                    product.role.as_str(),
                    product.contract_id,
                    package_name,
                    product_name,
                    role.as_str(),
                    contract_id
                ),
            );
            unsafe {
                windows_sys::Win32::Foundation::FreeLibrary(module);
            }
            return Err(format!(
                "native product descriptor mismatch for `{}`; bundle manifest says `{}:{}` role `{}` contract `{}`, descriptor says `{}:{}` role `{}` contract `{}`",
                dll_path.display(),
                product.package_name,
                product.product_name,
                product.role.as_str(),
                product.contract_id,
                package_name,
                product_name,
                role.as_str(),
                contract_id
            ));
        }
        if api.contract_version != product.contract_version {
            unsafe {
                windows_sys::Win32::Foundation::FreeLibrary(module);
            }
            return Err(format!(
                "native product `{}` reports contract version `{}` but bundle manifest expected `{}`",
                dll_path.display(),
                api.contract_version,
                product.contract_version
            ));
        }
        if api.role_ops.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::FreeLibrary(module);
            }
            return Err(format!(
                "native product `{}` has null role_ops for role `{}`",
                dll_path.display(),
                role.as_str()
            ));
        }

        let (
            create_instance,
            destroy_instance,
            child_run_entrypoint,
            plugin_describe_instance,
            plugin_use_instance,
            binding_imports,
            binding_callbacks,
            binding_register_callback,
            binding_unregister_callback,
            last_error_alloc,
            owned_bytes_free,
            owned_str_free,
        ) = match role {
            ArcanaCabiProductRole::Child => {
                let child_ops = unsafe { &*(api.role_ops as *const ArcanaCabiChildOpsV1) };
                if child_ops.base.ops_size < std::mem::size_of::<ArcanaCabiInstanceOpsV1>() {
                    unsafe {
                        windows_sys::Win32::Foundation::FreeLibrary(module);
                    }
                    return Err(format!(
                        "native child product `{}` reported instance ops size {} smaller than expected {}",
                        dll_path.display(),
                        child_ops.base.ops_size,
                        std::mem::size_of::<ArcanaCabiInstanceOpsV1>()
                    ));
                }
                (
                    child_ops.base.create_instance,
                    child_ops.base.destroy_instance,
                    Some(child_ops.run_entrypoint),
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    None,
                    None,
                    Some(child_ops.last_error_alloc),
                    Some(child_ops.owned_bytes_free),
                    None,
                )
            }
            ArcanaCabiProductRole::Plugin => {
                let plugin_ops = unsafe { &*(api.role_ops as *const ArcanaCabiPluginOpsV1) };
                if plugin_ops.base.ops_size < std::mem::size_of::<ArcanaCabiInstanceOpsV1>() {
                    unsafe {
                        windows_sys::Win32::Foundation::FreeLibrary(module);
                    }
                    return Err(format!(
                        "native plugin product `{}` reported instance ops size {} smaller than expected {}",
                        dll_path.display(),
                        plugin_ops.base.ops_size,
                        std::mem::size_of::<ArcanaCabiInstanceOpsV1>()
                    ));
                }
                (
                    plugin_ops.base.create_instance,
                    plugin_ops.base.destroy_instance,
                    None,
                    Some(plugin_ops.describe_instance),
                    Some(plugin_ops.use_instance),
                    Vec::new(),
                    Vec::new(),
                    None,
                    None,
                    Some(plugin_ops.last_error_alloc),
                    Some(plugin_ops.owned_bytes_free),
                    None,
                )
            }
            ArcanaCabiProductRole::Binding => {
                let binding_ops = unsafe { &*(api.role_ops as *const ArcanaCabiBindingOpsV1) };
                if binding_ops.base.ops_size < std::mem::size_of::<ArcanaCabiInstanceOpsV1>() {
                    unsafe {
                        windows_sys::Win32::Foundation::FreeLibrary(module);
                    }
                    return Err(format!(
                        "native binding product `{}` reported instance ops size {} smaller than expected {}",
                        dll_path.display(),
                        binding_ops.base.ops_size,
                        std::mem::size_of::<ArcanaCabiInstanceOpsV1>()
                    ));
                }
                let imports =
                    unsafe { read_binding_imports(binding_ops.imports, binding_ops.import_count)? };
                let callbacks = unsafe {
                    read_binding_callbacks(binding_ops.callbacks, binding_ops.callback_count)?
                };
                validate_binding_imports(&imports)?;
                validate_binding_callbacks(&callbacks)?;
                (
                    binding_ops.base.create_instance,
                    binding_ops.base.destroy_instance,
                    None,
                    None,
                    None,
                    imports,
                    callbacks,
                    Some(binding_ops.register_callback),
                    Some(binding_ops.unregister_callback),
                    Some(binding_ops.last_error_alloc),
                    Some(binding_ops.owned_bytes_free),
                    Some(binding_ops.owned_str_free),
                )
            }
            ArcanaCabiProductRole::Export => {
                unsafe {
                    windows_sys::Win32::Foundation::FreeLibrary(module);
                }
                return Err(format!(
                    "runtime native product loader does not support role `export` for `{}`",
                    dll_path.display()
                ));
            }
        };

        native_product_probe(
            "load_library",
            format!(
                "path={} package={} product={} role={} contract={}",
                dll_path.display(),
                package_name,
                product_name,
                role.as_str(),
                contract_id
            ),
        );

        Ok(Self {
            module,
            package_name,
            product_name,
            destroy_instance,
            create_instance,
            child_run_entrypoint,
            plugin_describe_instance,
            plugin_use_instance,
            binding_imports,
            binding_callbacks,
            binding_register_callback,
            binding_unregister_callback,
            last_error_alloc,
            owned_bytes_free,
            owned_str_free,
        })
    }

    fn create_instance(&self) -> Result<*mut c_void, String> {
        let instance = unsafe { (self.create_instance)() };
        if instance.is_null() {
            return Err("native product instance factory returned null".to_string());
        }
        Ok(instance)
    }
}

#[cfg(windows)]
unsafe fn read_binding_imports(
    entries: *const ArcanaCabiBindingImportEntryV1,
    count: usize,
) -> Result<Vec<ArcanaCabiBindingImport>, String> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let slice = unsafe { std::slice::from_raw_parts(entries, count) };
    slice
        .iter()
        .map(|entry| {
            Ok(ArcanaCabiBindingImport {
                name: unsafe { read_cabi_c_string(entry.name, "binding import name") }?,
                symbol_name: unsafe {
                    read_cabi_c_string(entry.symbol_name, "binding import symbol")
                }?,
                return_type: ArcanaCabiType::parse(&unsafe {
                    read_cabi_c_string(entry.return_type, "binding import return type")
                }?)?,
                params: unsafe { read_binding_params(entry.params, entry.param_count) }?,
            })
        })
        .collect()
}

#[cfg(windows)]
unsafe fn read_binding_callbacks(
    entries: *const ArcanaCabiBindingCallbackEntryV1,
    count: usize,
) -> Result<Vec<ArcanaCabiBindingCallback>, String> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let slice = unsafe { std::slice::from_raw_parts(entries, count) };
    slice
        .iter()
        .map(|entry| {
            Ok(ArcanaCabiBindingCallback {
                name: unsafe { read_cabi_c_string(entry.name, "binding callback name") }?,
                return_type: ArcanaCabiType::parse(&unsafe {
                    read_cabi_c_string(entry.return_type, "binding callback return type")
                }?)?,
                params: unsafe { read_binding_params(entry.params, entry.param_count) }?,
            })
        })
        .collect()
}

#[cfg(windows)]
unsafe fn read_binding_params(
    entries: *const arcana_cabi::ArcanaCabiExportParamV1,
    count: usize,
) -> Result<Vec<ArcanaCabiExportParam>, String> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let slice = unsafe { std::slice::from_raw_parts(entries, count) };
    slice
        .iter()
        .map(|entry| {
            let write_back_type = if entry.write_back_type.is_null() {
                None
            } else {
                Some(ArcanaCabiType::parse(&unsafe {
                    read_cabi_c_string(entry.write_back_type, "binding param write-back type")
                }?)?)
            };
            Ok(ArcanaCabiExportParam {
                name: unsafe { read_cabi_c_string(entry.name, "binding param name") }?,
                source_mode: ArcanaCabiParamSourceMode::parse(&unsafe {
                    read_cabi_c_string(entry.source_mode, "binding param source mode")
                }?)?,
                pass_mode: ArcanaCabiPassMode::parse(&unsafe {
                    read_cabi_c_string(entry.pass_mode, "binding param pass mode")
                }?)?,
                input_type: ArcanaCabiType::parse(&unsafe {
                    read_cabi_c_string(entry.input_type, "binding param input type")
                }?)?,
                write_back_type,
            })
        })
        .collect()
}

#[cfg(windows)]
unsafe fn read_cabi_c_string(value: *const c_char, label: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(format!("{label} cannot be null"));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(|text| text.to_string())
        .map_err(|_| format!("{label} is not valid utf-8"))
}

#[cfg(windows)]
impl Drop for LoadedNativeLibrary {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::FreeLibrary(self.module);
        }
    }
}

#[cfg(windows)]
fn run_active_child_binding_entrypoint(
    binding: &ActiveChildBinding,
    package_image_text: &str,
    main_routine_key: &str,
) -> Result<i32, String> {
    let routine_key = CString::new(main_routine_key).map_err(|_| {
        format!("main routine key `{main_routine_key}` contains an interior NUL byte")
    })?;
    let mut exit_code = 0i32;
    let ok = unsafe {
        (binding.run_entrypoint)(
            binding.instance.instance,
            package_image_text.as_bytes().as_ptr(),
            package_image_text.len(),
            routine_key.as_ptr(),
            &mut exit_code,
        )
    };
    if ok == 0 {
        let err = read_allocated_utf8_bytes(
            binding.last_error_alloc,
            binding.owned_bytes_free,
            "child last_error_alloc",
        )?
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| {
            format!(
                "child runtime provider `{}:{}` failed without an error message",
                binding.product.package_name, binding.product.product_name
            )
        });
        native_product_probe(
            "child_runtime_provider_error",
            format!(
                "package={} product={} error={}",
                binding.product.package_name, binding.product.product_name, err
            ),
        );
        return Err(err);
    }
    native_product_probe(
        "child_runtime_provider_entrypoint",
        format!(
            "package={} product={} routine={} exit_code={}",
            binding.product.package_name, binding.product.product_name, main_routine_key, exit_code
        ),
    );
    Ok(exit_code)
}

#[cfg(windows)]
unsafe fn read_cabi_utf8_field(ptr: *const c_char, field: &str) -> Result<String, String> {
    if ptr.is_null() {
        return Err(format!(
            "native product descriptor field `{field}` must not be null"
        ));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(ToOwned::to_owned)
        .map_err(|e| format!("native product descriptor field `{field}` is not utf8: {e}"))
}

#[cfg(windows)]
fn read_allocated_utf8_bytes(
    alloc: ArcanaCabiLastErrorAllocFn,
    free: ArcanaCabiOwnedBytesFreeFn,
    context: &str,
) -> Result<Option<String>, String> {
    read_allocated_utf8_from_raw(|out_len| unsafe { alloc(out_len) }, free, context)
}

#[cfg(windows)]
fn read_plugin_description(plugin: &OpenPluginInstance) -> Result<Option<String>, String> {
    read_allocated_utf8_from_raw(
        |out_len| unsafe { (plugin.describe_instance)(plugin.active.instance, out_len) },
        plugin.owned_bytes_free,
        "plugin describe_instance",
    )
}

#[cfg(windows)]
fn read_plugin_use_response(
    plugin: &OpenPluginInstance,
    request: &[u8],
) -> Result<Vec<u8>, String> {
    read_allocated_bytes_from_raw(
        |out_len| unsafe {
            (plugin.use_instance)(
                plugin.active.instance,
                request.as_ptr(),
                request.len(),
                out_len,
            )
        },
        plugin.owned_bytes_free,
        "plugin use_instance",
    )
}

#[cfg(windows)]
fn read_library_last_error(
    alloc: ArcanaCabiLastErrorAllocFn,
    free: ArcanaCabiOwnedBytesFreeFn,
) -> Option<String> {
    read_allocated_utf8_bytes(alloc, free, "native product last_error")
        .ok()
        .flatten()
}

#[cfg(windows)]
fn read_allocated_bytes_from_raw<F>(
    alloc: F,
    free: ArcanaCabiOwnedBytesFreeFn,
    context: &str,
) -> Result<Vec<u8>, String>
where
    F: FnOnce(*mut usize) -> *mut u8,
{
    let mut len = 0usize;
    let ptr = alloc(&mut len);
    if ptr.is_null() {
        if len == 0 {
            return Ok(Vec::new());
        }
        return Err(format!(
            "{context} returned null with non-zero length {len}"
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
    unsafe {
        free(ptr, len);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn read_allocated_utf8_from_raw<F>(
    alloc: F,
    free: ArcanaCabiOwnedBytesFreeFn,
    context: &str,
) -> Result<Option<String>, String>
where
    F: FnOnce(*mut usize) -> *mut u8,
{
    let bytes = read_allocated_bytes_from_raw(alloc, free, context)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|e| format!("{context} returned invalid utf8: {e}"))
}

fn native_product_probe(event: &str, message: impl AsRef<str>) {
    if std::env::var_os(NATIVE_PRODUCT_TEMP_PROBES_ENV).is_some() {
        eprintln!(
            "[arcana-native-product-probe] {event}: {}",
            message.as_ref()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC, RuntimeChildBindingInfo, load_bundle_native_products,
        load_bundle_native_products_from_text, read_embedded_distribution_manifest,
    };
    use arcana_aot::{AotInstanceProductSpec, compile_instance_product};
    use arcana_cabi::{
        ARCANA_CABI_CHILD_CONTRACT_ID, ARCANA_CABI_PLUGIN_CONTRACT_ID, ArcanaCabiProductRole,
    };
    use std::fs;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-runtime-native-product-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn build_instance_product(
        dir: &Path,
        package_name: &str,
        product_name: &str,
        role: ArcanaCabiProductRole,
        contract_id: &str,
        file: &str,
    ) -> PathBuf {
        let project_dir = dir.join("project").join(product_name);
        let artifact_dir = dir.join("target").join(product_name);
        let cargo_target_dir = artifact_dir.join("cargo-target");
        let compiled = compile_instance_product(
            &AotInstanceProductSpec {
                package_id: package_name.to_string(),
                package_name: package_name.to_string(),
                product_name: product_name.to_string(),
                role,
                contract_id: contract_id.to_string(),
                output_file_name: file.to_string(),
                package_image_text: None,
                binding_imports: Vec::new(),
                binding_callbacks: Vec::new(),
                binding_shackle_decls: Vec::new(),
            },
            &project_dir,
            &artifact_dir,
            &cargo_target_dir,
        )
        .expect("instance product should compile");
        let output = dir.join(file);
        fs::copy(compiled.output_path, &output).expect("compiled output should copy");
        output
    }

    #[cfg(windows)]
    #[test]
    fn bundle_native_product_catalog_activates_children_and_opens_plugins() {
        let dir = temp_dir("bundle_native_products");
        build_instance_product(
            &dir,
            "arcana_desktop",
            "default",
            ArcanaCabiProductRole::Child,
            ARCANA_CABI_CHILD_CONTRACT_ID,
            "arcwin.dll",
        );
        build_instance_product(
            &dir,
            "tooling",
            "tools",
            ArcanaCabiProductRole::Plugin,
            ARCANA_CABI_PLUGIN_CONTRACT_ID,
            "tooling_tools.dll",
        );
        fs::write(
            dir.join("arcana.bundle.toml"),
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"arcwin.dll\", \"tooling_tools.dll\"]\n",
                "\n[runtime_child_binding]\n",
                "consumer_member = \"app\"\n",
                "dependency_alias = \"arcana_desktop\"\n",
                "package_id = \"arcana_desktop\"\n",
                "package_name = \"arcana_desktop\"\n",
                "product_name = \"default\"\n",
                "\n[[native_products]]\n",
                "package_id = \"arcana_desktop\"\n",
                "package_name = \"arcana_desktop\"\n",
                "product_name = \"default\"\n",
                "role = \"child\"\n",
                "contract_id = \"arcana.cabi.child.v1\"\n",
                "file = \"arcwin.dll\"\n",
                "\n[[native_products]]\n",
                "package_id = \"tooling\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"tools\"\n",
                "role = \"plugin\"\n",
                "contract_id = \"arcana.cabi.plugin.v1\"\n",
                "file = \"tooling_tools.dll\"\n",
                "\n[[child_bindings]]\n",
                "consumer_member = \"app\"\n",
                "dependency_alias = \"arcana_desktop\"\n",
                "package_id = \"arcana_desktop\"\n",
                "package_name = \"arcana_desktop\"\n",
                "product_name = \"default\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let mut catalog = load_bundle_native_products(&dir).expect("catalog should load");
        assert_eq!(catalog.products().len(), 2);
        assert_eq!(catalog.child_bindings().len(), 1);
        assert_eq!(
            catalog.runtime_child_binding(),
            Some(&RuntimeChildBindingInfo {
                consumer_member: "app".to_string(),
                dependency_alias: "arcana_desktop".to_string(),
                package_id: "arcana_desktop".to_string(),
                package_name: "arcana_desktop".to_string(),
                product_name: "default".to_string(),
            })
        );
        assert_eq!(catalog.plugin_products().len(), 1);

        catalog
            .activate_children()
            .expect("children should activate");
        assert_eq!(catalog.active_child_binding_count(), 1);
        let child_err = catalog
            .run_child_entrypoint("{}", "main")
            .expect_err("invalid package image should surface child provider errors");
        assert!(
            child_err.contains("runtime package image") || child_err.contains("failed to parse"),
            "{child_err}"
        );

        let handle = catalog
            .open_plugin("tooling", "tools")
            .expect("plugin should open");
        assert_eq!(catalog.open_plugin_count(), 1);
        let description = catalog
            .describe_open_plugin(handle)
            .expect("plugin description should resolve");
        assert!(description.contains("tooling:tools"), "{description}");
        let response = catalog
            .use_open_plugin(handle, b"ping")
            .expect("plugin use should resolve");
        let response = String::from_utf8(response).expect("plugin response should be utf8");
        assert!(response.contains("tooling:tools"), "{response}");
        assert!(response.contains("ping"), "{response}");
        catalog
            .release_plugin(handle)
            .expect("plugin should release cleanly");
        assert_eq!(catalog.open_plugin_count(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn bundle_native_product_catalog_scopes_child_runtime_provider_to_root_member() {
        let dir = temp_dir("bundle_native_products_root_scope");
        build_instance_product(
            &dir,
            "arcana_desktop",
            "default",
            ArcanaCabiProductRole::Child,
            ARCANA_CABI_CHILD_CONTRACT_ID,
            "arcwin.dll",
        );
        build_instance_product(
            &dir,
            "tooling",
            "default",
            ArcanaCabiProductRole::Child,
            ARCANA_CABI_CHILD_CONTRACT_ID,
            "tooling_default.dll",
        );
        fs::write(
            dir.join("arcana.bundle.toml"),
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"arcwin.dll\", \"tooling_default.dll\"]\n",
                "\n[runtime_child_binding]\n",
                "consumer_member = \"app\"\n",
                "dependency_alias = \"arcana_desktop\"\n",
                "package_id = \"arcana_desktop\"\n",
                "package_name = \"arcana_desktop\"\n",
                "product_name = \"default\"\n",
                "\n[[native_products]]\n",
                "package_id = \"arcana_desktop\"\n",
                "package_name = \"arcana_desktop\"\n",
                "product_name = \"default\"\n",
                "role = \"child\"\n",
                "contract_id = \"arcana.cabi.child.v1\"\n",
                "file = \"arcwin.dll\"\n",
                "\n[[native_products]]\n",
                "package_id = \"tooling\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"default\"\n",
                "role = \"child\"\n",
                "contract_id = \"arcana.cabi.child.v1\"\n",
                "file = \"tooling_default.dll\"\n",
                "\n[[child_bindings]]\n",
                "consumer_member = \"app\"\n",
                "dependency_alias = \"arcana_desktop\"\n",
                "package_id = \"arcana_desktop\"\n",
                "package_name = \"arcana_desktop\"\n",
                "product_name = \"default\"\n",
                "\n[[child_bindings]]\n",
                "consumer_member = \"tooling\"\n",
                "dependency_alias = \"tools\"\n",
                "package_id = \"tooling\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"default\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let mut catalog = load_bundle_native_products(&dir).expect("catalog should load");
        assert_eq!(catalog.root_member(), Some("app"));
        assert_eq!(
            catalog.runtime_child_binding(),
            Some(&RuntimeChildBindingInfo {
                consumer_member: "app".to_string(),
                dependency_alias: "arcana_desktop".to_string(),
                package_id: "arcana_desktop".to_string(),
                package_name: "arcana_desktop".to_string(),
                product_name: "default".to_string(),
            })
        );
        catalog
            .activate_children()
            .expect("children should activate");
        assert_eq!(catalog.active_child_binding_count(), 2);
        let child_err = catalog.run_child_entrypoint("{}", "main").expect_err(
            "root child provider should run instead of reporting bundle-global ambiguity",
        );
        assert!(
            !child_err.contains("runtime provider selection is ambiguous"),
            "{child_err}"
        );
        assert!(
            child_err.contains("runtime package image") || child_err.contains("failed to parse"),
            "{child_err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn bundle_native_product_catalog_rejects_plugin_open_by_ambiguous_package_name() {
        let dir = temp_dir("bundle_native_products_plugin_ambiguity");
        fs::write(dir.join("tooling_v1.dll"), b"not-a-real-dll").expect("dummy dll should write");
        fs::write(dir.join("tooling_v2.dll"), b"not-a-real-dll").expect("dummy dll should write");
        fs::write(
            dir.join("arcana.bundle.toml"),
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"tooling_v1.dll\", \"tooling_v2.dll\"]\n",
                "\n[[native_products]]\n",
                "package_id = \"registry:local:tooling@1.0.0\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"tools\"\n",
                "role = \"plugin\"\n",
                "contract_id = \"arcana.cabi.plugin.v1\"\n",
                "file = \"tooling_v1.dll\"\n",
                "\n[[native_products]]\n",
                "package_id = \"registry:local:tooling@2.0.0\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"tools\"\n",
                "role = \"plugin\"\n",
                "contract_id = \"arcana.cabi.plugin.v1\"\n",
                "file = \"tooling_v2.dll\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let mut catalog = load_bundle_native_products(&dir).expect("catalog should load");
        let err = catalog
            .open_plugin("tooling", "tools")
            .expect_err("ambiguous package-name plugin open should fail");
        assert!(err.contains("multiple native products named"), "{err}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn bundle_native_product_catalog_falls_back_when_only_transitive_child_bindings_exist() {
        let dir = temp_dir("bundle_native_products_transitive_only");
        build_instance_product(
            &dir,
            "tooling",
            "default",
            ArcanaCabiProductRole::Child,
            ARCANA_CABI_CHILD_CONTRACT_ID,
            "tooling_default.dll",
        );
        fs::write(
            dir.join("arcana.bundle.toml"),
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"tooling_default.dll\"]\n",
                "\n[[native_products]]\n",
                "package_id = \"tooling\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"default\"\n",
                "role = \"child\"\n",
                "contract_id = \"arcana.cabi.child.v1\"\n",
                "file = \"tooling_default.dll\"\n",
                "\n[[child_bindings]]\n",
                "consumer_member = \"tooling\"\n",
                "dependency_alias = \"tools\"\n",
                "package_id = \"tooling\"\n",
                "package_name = \"tooling\"\n",
                "product_name = \"default\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let mut catalog = load_bundle_native_products(&dir).expect("catalog should load");
        assert_eq!(catalog.runtime_child_binding(), None);
        catalog
            .activate_children()
            .expect("children should activate");
        assert_eq!(catalog.active_child_binding_count(), 1);
        assert_eq!(
            catalog
                .run_child_entrypoint("{}", "main")
                .expect("non-root child bindings should not block fallback"),
            None
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn bundle_native_product_catalog_reads_embedded_distribution_manifest() {
        let dir = temp_dir("bundle_native_products_embedded_manifest");
        build_instance_product(
            &dir,
            "arcana_desktop",
            "default",
            ArcanaCabiProductRole::Child,
            ARCANA_CABI_CHILD_CONTRACT_ID,
            "arcwin.dll",
        );
        let root_artifact = dir.join("app.exe");
        fs::write(&root_artifact, b"MZembedded-test").expect("root artifact should write");
        let manifest_text = concat!(
            "format = \"arcana-distribution-bundle-v2\"\n",
            "member = \"app\"\n",
            "target = \"windows-exe\"\n",
            "target_format = \"arcana-native-exe-v1\"\n",
            "root_artifact = \"app.exe\"\n",
            "artifact_hash = \"sha256:test\"\n",
            "toolchain = \"toolchain\"\n",
            "support_files = [\"arcwin.dll\"]\n",
            "\n[runtime_child_binding]\n",
            "consumer_member = \"app\"\n",
            "dependency_alias = \"arcana_desktop\"\n",
            "package_id = \"arcana_desktop\"\n",
            "package_name = \"arcana_desktop\"\n",
            "product_name = \"default\"\n",
            "\n[[native_products]]\n",
            "package_id = \"arcana_desktop\"\n",
            "package_name = \"arcana_desktop\"\n",
            "product_name = \"default\"\n",
            "role = \"child\"\n",
            "contract_id = \"arcana.cabi.child.v1\"\n",
            "file = \"arcwin.dll\"\n",
            "\n[[child_bindings]]\n",
            "consumer_member = \"app\"\n",
            "dependency_alias = \"arcana_desktop\"\n",
            "package_id = \"arcana_desktop\"\n",
            "package_name = \"arcana_desktop\"\n",
            "product_name = \"default\"\n",
        );
        let payload = manifest_text.as_bytes();
        let mut file = OpenOptions::new()
            .append(true)
            .open(&root_artifact)
            .expect("root artifact should reopen");
        file.write_all(payload).expect("payload should write");
        file.write_all(&(payload.len() as u64).to_le_bytes())
            .expect("payload length should write");
        file.write_all(EMBEDDED_DISTRIBUTION_MANIFEST_MAGIC)
            .expect("payload marker should write");

        let embedded = read_embedded_distribution_manifest(&root_artifact)
            .expect("embedded manifest should read")
            .expect("embedded manifest should exist");
        let catalog = load_bundle_native_products_from_text(
            &dir,
            &root_artifact.display().to_string(),
            &embedded,
        )
        .expect("embedded distribution manifest should load");
        assert_eq!(catalog.root_member(), Some("app"));
        assert_eq!(catalog.products().len(), 1);
        assert_eq!(catalog.child_bindings().len(), 1);
        assert_eq!(
            catalog.runtime_child_binding(),
            Some(&RuntimeChildBindingInfo {
                consumer_member: "app".to_string(),
                dependency_alias: "arcana_desktop".to_string(),
                package_id: "arcana_desktop".to_string(),
                package_name: "arcana_desktop".to_string(),
                product_name: "default".to_string(),
            })
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bundle_native_product_catalog_ignores_missing_manifest() {
        let dir = temp_dir("bundle_native_products_missing");
        let catalog = load_bundle_native_products(&dir).expect("missing manifest should not fail");
        assert!(catalog.products().is_empty());
        assert!(catalog.child_bindings().is_empty());
        assert!(catalog.runtime_child_binding().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bundle_native_product_catalog_reads_package_asset_roots() {
        let dir = temp_dir("bundle_native_products_package_assets");
        fs::create_dir_all(dir.join("package-assets").join("abc123"))
            .expect("asset root should create");
        fs::write(
            dir.join("package-assets")
                .join("abc123")
                .join("runtime.txt"),
            "asset\n",
        )
        .expect("asset file should write");
        fs::write(
            dir.join("arcana.bundle.toml"),
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"package-assets/abc123/runtime.txt\"]\n",
                "\n[[package_assets]]\n",
                "package_id = \"path:arcana_text\"\n",
                "package_name = \"arcana_text\"\n",
                "asset_root = \"package-assets/abc123\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let catalog = load_bundle_native_products(&dir).expect("catalog should load");
        assert_eq!(catalog.root_member(), Some("app"));
        assert_eq!(
            catalog.package_asset_root("path:arcana_text"),
            Some("package-assets/abc123")
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
