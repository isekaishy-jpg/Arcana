export record Rect:
    pos: (Int, Int)
    size: (Int, Int)

export record ViewLayout:
    window_size: (Int, Int)
    header_height: Int
    gutter: Int
    left_panel: layout.Rect
    center_panel: layout.Rect
    right_panel: layout.Rect
    button_cols: Int
    button_size: (Int, Int)
    button_gap: (Int, Int)

fn max_int(left: Int, right: Int) -> Int:
    if left > right:
        return left
    return right

fn min_int(left: Int, right: Int) -> Int:
    if left < right:
        return left
    return right

fn clamp_int(value: Int, low: Int, high: Int) -> Int:
    return layout.max_int :: low, (layout.min_int :: value, high :: call) :: call

export fn for_window(window_size: (Int, Int)) -> layout.ViewLayout:
    let width = layout.max_int :: window_size.0, 960 :: call
    let height = layout.max_int :: window_size.1, 640 :: call
    let gutter = 18
    let header_height = 84
    let body_height = height - header_height - gutter
    let available_width = width - gutter * 4
    let mut left_width = layout.clamp_int :: available_width * 30 / 100, 320, 420 :: call
    let mut center_width = layout.clamp_int :: available_width * 34 / 100, 300, 520 :: call
    let min_right_width = 280
    let min_center_width = 280
    let min_left_width = 280
    let mut right_width = available_width - left_width - center_width
    if right_width < min_right_width:
        let deficit = min_right_width - right_width
        let center_slack = center_width - min_center_width
        if center_slack >= deficit:
            center_width -= deficit
        else:
            center_width = min_center_width
            left_width = layout.max_int :: min_left_width, left_width - (deficit - center_slack) :: call
        right_width = available_width - left_width - center_width
    let left_panel = layout.Rect :: pos = (gutter, header_height), size = (left_width, body_height) :: call
    let center_panel = layout.Rect :: pos = (gutter * 2 + left_width, header_height), size = (center_width, body_height) :: call
    let right_panel = layout.Rect :: pos = (gutter * 3 + left_width + center_width, header_height), size = (right_width, body_height) :: call
    let inner_button_width = left_panel.size.0 - gutter * 2
    let button_cols = 3
    let button_gap = (8, 8)
    let button_width = (inner_button_width - (button_cols - 1) * button_gap.0) / button_cols
    let button_size = (button_width, 30)
    let mut view = layout.ViewLayout :: window_size = (width, height), header_height = header_height, gutter = gutter :: call
    view.left_panel = left_panel
    view.center_panel = center_panel
    view.right_panel = right_panel
    view.button_cols = button_cols
    view.button_size = button_size
    view.button_gap = button_gap
    return view

export fn button_rect(read view: layout.ViewLayout, id: Int) -> layout.Rect:
    let col = id % view.button_cols
    let row = id / view.button_cols
    let x = view.left_panel.pos.0 + view.gutter + col * (view.button_size.0 + view.button_gap.0)
    let y = view.left_panel.pos.1 + view.gutter + row * (view.button_size.1 + view.button_gap.1)
    return layout.Rect :: pos = (x, y), size = view.button_size :: call

export fn button_at(read view: layout.ViewLayout, point: (Int, Int)) -> Int:
    let start_x = view.left_panel.pos.0 + view.gutter
    let start_y = view.left_panel.pos.1 + view.gutter
    let local_x = point.0 - start_x
    let local_y = point.1 - start_y
    if local_x < 0 or local_y < 0:
        return -1
    let stride_x = view.button_size.0 + view.button_gap.0
    let stride_y = view.button_size.1 + view.button_gap.1
    if stride_x <= 0 or stride_y <= 0:
        return -1
    let col = local_x / stride_x
    let row = local_y / stride_y
    if col < 0 or col >= view.button_cols or row < 0:
        return -1
    let in_button_x = local_x - col * stride_x
    let in_button_y = local_y - row * stride_y
    if in_button_x >= view.button_size.0 or in_button_y >= view.button_size.1:
        return -1
    let id = row * view.button_cols + col
    if id < 0 or id >= 36:
        return -1
    return id
