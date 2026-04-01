foreword alias app.meta.local = tool.meta.trace

record Session:
    #app.meta.local[label = "field"]
    value: Int

#tool.exec.note[slot = "smoke"]
#test
fn smoke() -> Int:
    return helper :: 7 :: call

#app.meta.local[label = "fn"]
fn helper(#app.meta.local[label = "param"] seed: Int) -> Int:
    return seed + 1

fn main() -> Int:
    let score = smoke :: :: call
    let session = Session :: value = score :: call
    return session.value
