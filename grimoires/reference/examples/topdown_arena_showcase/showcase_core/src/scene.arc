import showcase_core.rng
import showcase_core.traits
import showcase_core.chains
import showcase_core.ecs_world
import showcase_core.concurrency
import showcase_core.checksums
import std.ecs
import std.memory
import std.collections.array
import std.collections.map

use std.ecs as ecs
use std.memory as memory
use std.collections.array as array
use std.collections.map as map

fn id_int(v: Int) -> Int:
    return v

fn scene_tick_limit() -> Int:
    return 20

fn scene_count() -> Int:
    return 8

fn init_components(seed: Int):
    let mut state = map.new[Str, Int] :: :: call
    state :: "tick", 0 :: set
    state :: "scene_index", 0 :: set
    state :: "scene_tick", 0 :: set
    state :: "player_x", 160 :: set
    state :: "player_y", 90 :: set
    state :: "rng_state", seed :: set
    state :: "checksum", 0 :: set
    state :: "ticks", 0 :: set
    state :: "shots", 0 :: set
    state :: "hits", 0 :: set
    state :: "events", 0 :: set
    state :: "score", 0 :: set
    state :: "waves", 0 :: set
    state :: "telemetry", 0 :: set
    let sums = array.new[Int] :: 8, 0 :: call
    ecs.set_component[Map[Str, Int]] :: state :: call
    ecs.set_component[Array[Int]] :: sums :: call

fn apply_scene(edit state: Map[Str, Int], input_blob: Int) -> Int:
    let fire = (input_blob % 2) == 1
    let packed = input_blob / 2
    let packed_io = packed % 4194304
    let packed_move = packed / 4194304
    let move_x = (packed_move / 2048) - 1000
    let move_y = (packed_move % 2048) - 1000
    let io_events = (packed_io / 2048) - 1000
    let io_wheel = (packed_io % 2048) - 1000

    if state["scene_index"] == 0:
        showcase_core.chains.seed :: state["tick"] :: call
            plan :=> showcase_core.chains.plus7 => showcase_core.chains.mul2
        return 11 + io_events + io_wheel

    if state["scene_index"] == 1:
        let local_x = state["player_x"]
        let mut local_y = state["player_y"]
        let x_ref = &local_x
        let mut nx = *x_ref + move_x
        if nx < 8:
            nx = 8
        if nx > 312:
            nx = 312
        let y_mut = &mut local_y
        let ny = *y_mut + move_y

        state["player_x"] = nx
        state["player_y"] = ny

        if fire:
            state["score"] += 3
        return state["player_x"] + ny + state["score"]

    if state["scene_index"] == 2:
        let ecs_val = showcase_core.ecs_world.tick :: state["tick"] :: call
        let metric = showcase_core.traits.score_metric :: ecs_val, 2 :: call
        return metric

    if state["scene_index"] == 3:
        let mut arena_static = memory.new[Int] :: 8 :: call
        let mut frame_scratch = memory.frame_new[Int] :: 16 :: call
        let mut pool_live = memory.pool_new[Int] :: 16 :: call

        let aid = arena: arena_static :> state["tick"] <: id_int
        let fid = frame: frame_scratch :> state["player_x"] + state["player_y"] <: id_int
        let pid = pool: pool_live :> state["tick"] + state["shots"] <: id_int

        let mut out = arena_static :: aid :: get
        out += frame_scratch :: fid :: get
        out += pool_live :: pid :: get
        if fire:
            pool_live :: pid :: remove
            state["shots"] += 1
        return out

    if state["scene_index"] == 4:
        showcase_core.chains.run_scene5_chains :: state["tick"] :: call
        let seeded = showcase_core.chains.seed :: state["tick"] :: call
        let tuned = showcase_core.chains.score_formula :: seeded :: call
        return tuned

    if state["scene_index"] == 5:
        let c = showcase_core.concurrency.tick :: state["tick"] :: call
        state["telemetry"] = c
        return c

    if state["scene_index"] == 6:
        let mut burst = 0
        for i in 0..6:
            let ecs_tick = showcase_core.ecs_world.tick :: (state["tick"] + i) :: call
            burst += ecs_tick
        let mut pool = memory.pool_new[Int] :: 12 :: call
        let id = pool: pool :> burst + state["tick"] <: id_int
        burst += pool :: id :: get
        state["waves"] += 1
        return burst + state["waves"]

    let total_stats = state["ticks"] + state["shots"] + state["events"]
    return state["score"] + state["telemetry"] + total_stats

export fn reset(seed: Int):
    init_components :: seed :: call

export fn step_packed(packed_move: Int, fire: Bool, packed_io: Int) -> Bool:
    let has_state = ecs.has_component[Map[Str, Int]] :: :: call
    if not has_state:
        init_components :: 0 :: call

    let mut state = ecs.get_component[Map[Str, Int]] :: :: call
    let mut sums = ecs.get_component[Array[Int]] :: :: call

    let mut fire_bit = 0
    if fire:
        fire_bit = 1
    let input_blob = ((packed_move * 4194304) + packed_io) * 2 + fire_bit

    let packed = input_blob / 2
    let packed_io_local = packed % 4194304
    let io_events = (packed_io_local / 2048) - 1000

    let r = showcase_core.rng.next :: state["rng_state"] :: call
    state["rng_state"] = r.0
    state["tick"] += 1
    state["ticks"] += 1
    state["events"] += io_events

    let delta = apply_scene :: state, input_blob :: call
    let salted = delta + (r.1 % 97)

    let idx = state["scene_index"]
    let prev_scene = sums[idx]
    let next_scene = showcase_core.checksums.mix :: prev_scene, salted, idx + 1 :: call
    sums[idx] = next_scene
    state["checksum"] = showcase_core.checksums.mix :: state["checksum"], salted, 99 + idx :: call

    state["scene_tick"] += 1
    let tick_limit = scene_tick_limit :: :: call
    if state["scene_tick"] >= tick_limit:
        state["scene_tick"] = 0
        state["scene_index"] += 1

    let total_scenes = scene_count :: :: call
    let done = state["scene_index"] >= total_scenes
    ecs.set_component[Map[Str, Int]] :: state :: call
    ecs.set_component[Array[Int]] :: sums :: call
    return done

export fn scene_index() -> Int:
    let has_state = ecs.has_component[Map[Str, Int]] :: :: call
    if not has_state:
        return 0
    let state = ecs.get_component[Map[Str, Int]] :: :: call
    return state["scene_index"]

export fn final_checksum() -> Int:
    let has_state = ecs.has_component[Map[Str, Int]] :: :: call
    if not has_state:
        return 0
    let state = ecs.get_component[Map[Str, Int]] :: :: call
    return state["checksum"]

export fn scene_checksum(index: Int) -> Int:
    let has_sums = ecs.has_component[Array[Int]] :: :: call
    if not has_sums:
        return 0
    let sums = ecs.get_component[Array[Int]] :: :: call
    return sums[index]
