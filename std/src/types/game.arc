export record EntityId:
    value: Int

export record Health:
    value: Int

export record Damage:
    value: Int

export record Score:
    value: Int

export record TeamId:
    value: Int

export fn entity_id(value: Int) -> std.types.game.EntityId:
    return std.types.game.EntityId :: value = value :: call

export fn health(value: Int) -> std.types.game.Health:
    return std.types.game.Health :: value = value :: call

export fn damage(value: Int) -> std.types.game.Damage:
    return std.types.game.Damage :: value = value :: call

export fn score(value: Int) -> std.types.game.Score:
    return std.types.game.Score :: value = value :: call

export fn team_id(value: Int) -> std.types.game.TeamId:
    return std.types.game.TeamId :: value = value :: call
