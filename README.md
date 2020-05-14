# gæmstone Bloxel Framework

The **gæmstone** project, pronounced /ɡɛmstoʊn/ (like "gem" but with the hard G from "game"), aims to be a framework for multiplayer bloxel games with its main goal being modularity. Written in [Rust](https://www.rust-lang.org/) and built on the [Amethyst](https://amethyst.rs/) game engine, it makes heavy use of [Entity Component System][ecs] (ECS) design.

At the moment, the project is very early in development and far from usable as anything except maybe as a reference on how to implement some aspects of it in another codebase.

[ecs]: https://en.wikipedia.org/wiki/Entity_component_system

## Goals

- Multiple independent worlds ("levels")
- Pseudo-infinite worlds in all 3 dimensions
- Physics-enabled entities made up of voxels (e.g. vehicles)
- [Palette-based compression][palette] for chunk block storage (unlimited block types)
- Levels and chunks are entities, chunk related storages are just components on chunks  
  Allows expanding chunks with additional data (fluid density, block shape, color) easily
- Block access API that supports setting arbitrary data on any block, creating block entities on the fly
- In-game entity, block and component inspector / editor
- Far in the future, multiplayer-enabled scripting, modifying and adding mechanics and assets

[palette]: https://www.reddit.com/r/VoxelGameDev/comments/9yu8qy/palettebased_compression_for_chunked_discrete/
