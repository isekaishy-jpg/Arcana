enum Packet:
    Data(Int)

fn main() -> Int:
    let packet = construct yield Packet.Data -return 0
        payload = 1
        payload = 2
    return 0
