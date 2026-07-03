# TODO

## brdb / brz loading (first pass done)

New-format loading (`.brdb` Worlds, `.brz` Prefabs) lands via `src/brdb_load.rs`,
which converts brdb bricks into the legacy `brickadia::save::SaveData`. Known gaps:

- **Dynamic brick grids skipped.** Only the main static grid (grid 1) is read.
  Dynamic grids are entity-relative and need entity transforms to place correctly.
- **Components not translated.** brdb component data (lights, etc.) is not mapped
  to `brickadia`'s `UnrealType`, so brick-driven point/spot lights won't spawn.
- Colors are emitted per-brick as `BrickColor::Unique`; no palette dedup.

## Performance

- Large builds tank framerate — reduce VRAM / pack vertex buffers more efficiently.
