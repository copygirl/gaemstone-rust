use {
  super::*,
  amethyst::{derive::SystemDesc, ecs::world::Index},
  std::collections::HashMap,
};

#[derive(Default)]
pub struct ChunkLookup {
  entity_lookup: HashMap<ChunkPos, Entity>,
  pos_lookup: HashMap<Index, ChunkPos>,
}

impl ChunkLookup {
  pub fn get(&self, pos: ChunkPos) -> Option<Entity> {
    self.entity_lookup.get(&pos).cloned()
  }
}

#[derive(SystemDesc)]
#[system_desc(name(ChunkLookupSystemDesc))]
pub struct ChunkLookupSystem {
  #[system_desc(flagged_storage_reader(Chunk))]
  reader: ReaderId<ComponentEvent>,
}

impl ChunkLookupSystem {
  pub fn new(reader: ReaderId<ComponentEvent>) -> Self {
    Self { reader }
  }
}

impl<'a> System<'a> for ChunkLookupSystem {
  type SystemData = (Entities<'a>, Write<'a, ChunkLookup>, ReadStorage<'a, Chunk>);

  fn run(&mut self, (entities, mut lookup, chunks): Self::SystemData) {
    use ComponentEvent::*;
    for event in chunks.channel().read(&mut self.reader) {
      match event {
        Inserted(index) => {
          let entity = entities.entity(*index);
          if let Some(chunk) = chunks.get(entity) {
            lookup.entity_lookup.insert(chunk.pos, entity);
            lookup.pos_lookup.insert(*index, chunk.pos);
          }
        }
        Modified(index) => {
          // NOTE: Chunk should not be modified once added, but just in case..

          if let Some(pos) = lookup.pos_lookup.remove(index) {
            lookup.entity_lookup.remove(&pos);
          }

          let entity = entities.entity(*index);
          if let Some(chunk) = chunks.get(entity) {
            lookup.entity_lookup.insert(chunk.pos, entity);
            lookup.pos_lookup.insert(*index, chunk.pos);
          }
        }
        Removed(index) => {
          if let Some(pos) = lookup.pos_lookup.remove(index) {
            lookup.entity_lookup.remove(&pos);
          }
        }
      };
    }
  }
}
