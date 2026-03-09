import types

#boundary[target = "lua"]
fn lua_value(read value: Int) -> Int:
    return value

#boundary[target = "sql"]
fn sql_row(read row: types.Row) -> types.Row:
    return row

fn main() -> Int:
    let row = types.Row :: id = 7 :: call
    let next = lua_value :: 2 :: call
    let stored = sql_row :: row :: call
    return next + stored.id
