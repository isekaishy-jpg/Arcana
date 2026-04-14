import std.text
import arcana_process.fs
import arcana_process.io
import std.result
use std.result.Result

record Token:
    name: Str

impl std.cleanup.Cleanup[Token] for Token:
    fn cleanup(take self: Token) -> Result[Unit, Str]:
        let _ = self
        arcana_process.io.print_line[Str] :: "default token cleanup" :: call
        return Result.Ok[Unit, Str] :: :: call

fn cleanup_token_logged(take token: Token) -> Result[Unit, Str]:
    let _ = token
    arcana_process.io.print_line[Str] :: "override token cleanup" :: call
    return Result.Ok[Unit, Str] :: :: call

// Bare cleanup covers the whole owning scope, so the local stream closes on every exit path.
fn read_prefix_len(path: Str) -> Result[Int, Str]:
    let mut stream = (arcana_process.fs.stream_open_read :: path :: call) :: :: ?
    let bytes = (arcana_process.fs.stream_read :: stream, 16 :: call) :: :: ?
    return Result.Ok[Int, Str] :: (std.text.bytes_len :: bytes :: call) :: call
-cleanup

// A targeted cleanup footer can use an explicit handler for one binding.
fn token_cleanup_logged() -> Result[Int, Str]:
    let token = Token :: name = "session" :: call
    return Result.Ok[Int, Str] :: 1 :: call
-cleanup[target = token, handler = cleanup_token_logged]

// Bare cleanup and targeted override can stack on one owner.
fn stacked_cleanup_logged() -> Result[Int, Str]:
    let left = Token :: name = "left" :: call
    let right = Token :: name = "right" :: call
    let _ = left
    return Result.Ok[Int, Str] :: 2 :: call
-cleanup
-cleanup[target = right, handler = cleanup_token_logged]

fn main() -> Int:
    return 0
