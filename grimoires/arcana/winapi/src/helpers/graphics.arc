export native fn gdi_memory_surface_stride(read width: Int, read height: Int) -> Int = helpers.graphics.gdi_memory_surface_stride
export native fn gdi_hidden_window_present() -> Bool = helpers.graphics.gdi_hidden_window_present
export native fn dxgi_adapter_count() -> Int = helpers.graphics.dxgi_adapter_count
export native fn bootstrap_d3d12_warp() -> Bool = helpers.graphics.bootstrap_d3d12_warp
export native fn bootstrap_dxgi_hidden_window_swapchain() -> Bool = helpers.graphics.bootstrap_dxgi_hidden_window_swapchain
export native fn bootstrap_d2d_factory() -> Bool = helpers.graphics.bootstrap_d2d_factory
export native fn bootstrap_wic_factory() -> Bool = helpers.graphics.bootstrap_wic_factory
