use crate::sync_arena::{SyncArena, Entry};

pub struct SyncArenaIterator<'a, T> {

    index: usize, // the index of the currently removed entry
    first: bool, // is this dumb?? we need to check if this is the first element for the first .next()
    restore_generation: u32, // the generation of the currently removed entry
    restore_sync_id: u64, // the sync id of the currently removed entry
    arena: &'a mut SyncArena<T>,
    restored: bool // was the previously removed value restored
}

impl<'a, T> SyncArenaIterator<'a, T> {

    /// Find the first valid entry, starting at specified index
    fn find_next_entry(items: &Vec<Entry<T>>, mut index: usize) -> Option<usize> {
        
        loop {

            if index >= items.len() {
                break None;
            }
            
            match &items[index] {
                Entry::Occupied { generation: _, value: _, sync_id: _} => {
                    return Some(index)
                },
                _ => {
                    index +=1;

                    continue;
                }
            };

        }
        
    }

    pub fn new(arena: &'a mut SyncArena<T>) -> Self {

        let iterator = Self {
            index: 0,
            first: true,
            arena,
            restore_generation: 0, // this too,
            restore_sync_id: 0,
            restored: true // initial is true because nothing has been removed
        };

        iterator

    }

    pub fn restore(&mut self, item: T) {


        self.arena.items[self.index] = Entry::Occupied { 
            generation: self.restore_generation, 
            value: item,
            sync_id: self.restore_sync_id
        };

        // only increase the length of the arena if we didn't already restore
        if !self.restored {
            self.arena.len += 1;
        }

        self.restored = true;

    }

    pub fn next(&mut self) -> Option<(T, &mut SyncArena<T>)> {

        match self.restored {
            true => {},
            false => {
                
                // arena generation increments when an element is removed
                self.arena.generation += 1;

                 // the free list head could have changed between the time the item was removed and restored
                self.arena.items[self.index] = Entry::Free { next_free: self.arena.free_list_head };

                // update the free list head to tell the arena its safe to reclaim the index
                self.arena.free_list_head = Some(self.index as u32);
            },
        }

        // if this is the first .next(), we want to start our search from 0, but if its not, we want to search AFTER the current index (or else we would just get the same value)
        if !self.first {
            self.index += 1;
        }
        else {
            self.first = false // now we can update self.first for the next .next()
        }

        self.index = match SyncArenaIterator::find_next_entry(&self.arena.items, self.index) {
            Some(next_index) => next_index,
            None => return None, // there are no more occupied entries in the arena
        };

        // replace the entry with a free entry, but dont update the free list head yet (we will do that only if the user decides not to restore the value)
        let entry = std::mem::replace(
            &mut self.arena.items[self.index], 
            Entry::Free { next_free: Some(u32::MAX) } // set next free as max just in case
        );

        self.arena.len -= 1;
        
        // get the actual value out of the entry to make it easier for the user
        let value = match entry {
            Entry::Free { next_free: _ } => unreachable!(), // we already identified this entry as occupied
            Entry::Occupied { generation, value, sync_id } => {
                
                self.restore_generation = generation;
                self.restore_sync_id = sync_id;

                value
                
            },
        };

        self.restored = false;

        return Some((value, self.arena))
        
    }

    
}
