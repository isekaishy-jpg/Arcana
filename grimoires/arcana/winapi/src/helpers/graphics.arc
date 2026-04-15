use arcana_winapi.graphics_handles.GdiWindowSurface

export native fn gdi_memory_surface_stride(read width: Int, read height: Int) -> Int = helpers.graphics.gdi_memory_surface_stride
export native fn gdi_hidden_window_present() -> Bool = helpers.graphics.gdi_hidden_window_present
export native fn gdi_window_surface_open(read hwnd: arcana_winapi.raw.types.HWND) -> GdiWindowSurface = helpers.graphics.gdi_window_surface_open
export native fn gdi_window_surface_configure(edit surface: GdiWindowSurface, read width: Int, read height: Int) -> Bool = helpers.graphics.gdi_window_surface_configure
export native fn gdi_window_surface_destroy(take surface: GdiWindowSurface) -> Bool = helpers.graphics.gdi_window_surface_destroy
export native fn gdi_window_surface_buffer_count(read surface: GdiWindowSurface) -> Int = helpers.graphics.gdi_window_surface_buffer_count
export native fn gdi_window_surface_pixel_len(read surface: GdiWindowSurface, read slot: Int) -> Int = helpers.graphics.gdi_window_surface_pixel_len
export native fn gdi_window_surface_pixel_at(read surface: GdiWindowSurface, read slot: Int, read index: Int) -> Int = helpers.graphics.gdi_window_surface_pixel_at
export native fn gdi_window_surface_pixel_set(edit surface: GdiWindowSurface, read slot: Int, read packed: Int) = helpers.graphics.gdi_window_surface_pixel_set
export native fn gdi_window_surface_present(read surface: GdiWindowSurface, read slot: Int) -> Bool = helpers.graphics.gdi_window_surface_present
export native fn gdi_window_surface_present_bounded(read surface: GdiWindowSurface, read slot: Int, read rect: arcana_winapi.raw.types.RECT) -> Bool = helpers.graphics.gdi_window_surface_present_bounded
export native fn gdi_window_surface_take_last_error() -> Str = helpers.graphics.gdi_window_surface_take_last_error
export native fn dxgi_adapter_count() -> Int = helpers.graphics.dxgi_adapter_count
export native fn bootstrap_d3d12_warp() -> Bool = helpers.graphics.bootstrap_d3d12_warp
export native fn bootstrap_dxgi_hidden_window_swapchain() -> Bool = helpers.graphics.bootstrap_dxgi_hidden_window_swapchain
export native fn bootstrap_d2d_factory() -> Bool = helpers.graphics.bootstrap_d2d_factory
export native fn bootstrap_wic_factory() -> Bool = helpers.graphics.bootstrap_wic_factory
