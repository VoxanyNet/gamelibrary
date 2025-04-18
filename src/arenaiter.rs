use rapier2d::data::{arena::Entry, Arena};

pub struct ArenaIterator<T> {
    next_index: Option<usize>, // where the next occupied entry is
    restore_index: usize, // the index where the currently removed value needs to be restored to
    restore_generation: u32, // the generation of the last entry we removed
    arena: Arena<T>,
}

impl<T> ArenaIterator<T> {

    pub fn new(mut arena: Arena<T>) -> (Self, T) 
    {

        let mut index = 0;

        // loop until we find the first element
        let first_value: T = loop {

            let has_value = match &arena.items[index] {
                Entry::Occupied { generation: _, value: _ } => {
                    true
                },
                _ => {
                    false
                }
            };
    
            if !has_value {
    
                index += 1;
                
                continue;
            }
    
            // replace the entry with a free entry, but dont update the free list head yet (we will do that only if the user decides not to restore the value)
            let entry = std::mem::replace(
                &mut arena.items[index], 
                Entry::Free { next_free: Some(u32::MAX) } // set next free as max just in case
            );

            //println!("{:?}", &mut arena.items[index]);
    
            let value = match entry {
                Entry::Free { next_free: _ } => unreachable!(), // we already identified this entry as occupied
                Entry::Occupied { generation: _, value } => {
                    value
                },
            };

            break value

        }; 

        arena.len -= 1;

        let mut next_index = 0;

        // loop until we find the next index if any
        let next_index: Option<usize> = loop {

            if next_index >= arena.items.len() {
                break None;
            }

            match &arena.items[next_index] {
                Entry::Occupied { generation: _, value: _ } => {

                    break Some(next_index);
                },
                Entry::Free { next_free: _ } => {
                    next_index += 1;

                    continue;
                }
            };


        };  

        //std::fs::write("new.json", serde_json::to_string_pretty(&arena).unwrap()).unwrap();

        let iterator = Self {
            restore_index: index,
            arena,
            restore_generation: 0,
            next_index
        };

        

        (iterator, first_value)


    }

    pub fn next(&mut self, value_to_restore: Option<T>) -> Option<T> {

        //let then_restore_value = Instant::now();
        match value_to_restore {
            Some(value) => {
                self.arena.items[self.restore_index] = Entry::Occupied {
                    generation: self.restore_generation, 
                    value
                };

                self.arena.len += 1;
            },
            None => {

                //println!("{:?}", self.arena.free_list_head);
                // properly mark the entry as being removed and free
                self.arena.items[self.restore_index] = Entry::Free { next_free: self.arena.free_list_head };

                self.arena.generation += 1;
                self.arena.free_list_head = Some(self.restore_index as u32);
        
            },
        }

        //println!("restore item: {:?}", then_restore_value.elapsed());

        let mut next_index = match self.next_index {
            Some(next_index) => next_index,
            None => return None,
        };

        //let then_replace_with_dummy = Instant::now();

        // replace the entry with a free entry, but dont update the free list head yet (we will do that only if the user decides not to restore the value)
        let entry = std::mem::replace(
            &mut self.arena.items[next_index], 
            Entry::Free { next_free: Some(u32::MAX) } // set next free as max just in case
        );

        //println!("dummy swap: {:?}", then_replace_with_dummy.elapsed());

        self.arena.len -= 1;

        let value = match entry {
            Entry::Free { next_free: _ } => unreachable!(), // we already identified this entry as occupied
            Entry::Occupied { generation, value } => {
                //println!("generation: {}", generation);
                self.restore_generation = generation;
                value
            },
        };

        self.restore_index = self.next_index.unwrap();

        //let then_find_next = Instant::now();

        // loop until we find the next occupied index (for the next .next())
        self.next_index = loop {

            if next_index >= self.arena.items.len() {
                break None
            }
            match &self.arena.items[next_index] {
                Entry::Occupied { generation: _, value: _ } => {
                    break Some(next_index)

                },
                Entry::Free { next_free: _ } => {

                    // check the next entry
                    next_index += 1;

                    continue;
                }
            };

    
        };

        //println!("time to find next value: {:?}", then_find_next.elapsed());

        return Some(value)


        
    }
}