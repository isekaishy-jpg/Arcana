export fn scene_label(index: Int) -> Str:
    if index == 0:
        return "S1_BOOT_IO"
    if index == 1:
        return "S2_MOVE_BORROW"
    if index == 2:
        return "S3_ECS_TRAITS"
    if index == 3:
        return "S4_MEMORY_MIX"
    if index == 4:
        return "S5_CHAIN_SCORE"
    if index == 5:
        return "S6_CONCURRENCY_TELEMETRY"
    if index == 6:
        return "S7_STRESS_BURST"
    return "S8_FINAL"
