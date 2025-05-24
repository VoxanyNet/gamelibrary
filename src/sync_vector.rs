use std::{collections::{HashMap, HashSet}, slice::IterMut, sync::mpsc, thread, time::Instant};

use diff::Diff;
use serde::{de::DeserializeOwned, Deserialize, Serialize};


#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SyncIndex {
    id: u64
}

impl SyncIndex {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().as_u64_pair().0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
struct SyncVector<T> {
    sync_map: HashMap<SyncIndex, usize>, // map sync id to local indices
    vec: Vec<T>
}

impl<T> SyncVector<T> {
    pub fn new() -> Self {
        Self {
            sync_map: HashMap::new(),
            vec: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn get(&self, sync_index: SyncIndex) -> Option<&T> {
        match self.sync_map.get(&sync_index) {
            Some(local_index) => {
                Some(&self.vec[*local_index])
            },
            None => {
                None
            },
        }
    }

    pub fn get_mut(&mut self, sync_index: SyncIndex) -> Option<&mut T> {
        match self.sync_map.get(&sync_index) {
            Some(local_index) => {
                Some(&mut self.vec[*local_index])
            },
            None => {
                None
            },
        }
    }

    pub fn push(&mut self, item: T) -> SyncIndex {

        let local_index = self.vec.len();

        self.vec.push(item);

        let sync_id = SyncIndex::new();

        self.sync_map.insert(sync_id, local_index);

        return sync_id


    }

    pub fn insert_with_known_sync_id(&mut self, item: T, sync_id: SyncIndex) {

        let local_index = self.vec.len();

        self.vec.push(item);

        self.sync_map.insert(sync_id, local_index);
    }

    pub fn remove(&mut self, sync_id: SyncIndex) -> Option<T> {

        match self.sync_map.get(&sync_id) {
            Some(local_index) => {
                Some(self.vec.remove(*local_index))
            },
            None => {
                None
            },
        }

    }
}
impl<'a, T> IntoIterator for &'a mut SyncVector<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.iter_mut()
    }
}

impl<T> Default for SyncVector<T> {
    fn default() -> Self {
        Self { sync_map: Default::default(), vec: Default::default() }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SyncVectorDiff<T>
where 
    T: Diff,
    T::Repr: Serialize + DeserializeOwned
{   
    pub altered: HashMap<SyncIndex, T::Repr>,
    pub removed: HashSet<SyncIndex>
}

impl<T> Diff for SyncVector<T>
where 
    T: Diff + PartialEq,
    T::Repr: Serialize + DeserializeOwned {
    type Repr = SyncVectorDiff<T>;

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff: SyncVectorDiff<T> = SyncVectorDiff {
            altered: HashMap::new(),
            removed: HashSet::new()
        };
        
        
        for (sync_index, local_index) in self.sync_map.iter() {  
            match other.get(*sync_index) {

                // item is in both arenas
                Some(other_value) => {

                    // our value
                    let value = self.get(*sync_index).unwrap(); // the sync map will always contain valid values

                    if other_value != value {
                        diff.altered.insert(
                            *sync_index, 
                            value.diff(other_value)
                        );
                    };
                },

                // item has been deleted
                None => {
                    diff.removed.insert(*sync_index);
                },
            }

        }   

        for (other_sync_index, other_local_index) in other.sync_map.iter() {

            match self.get(*other_sync_index) {

                // item is in both arenas (dont need to do anything)
                Some(_) => {
                    
                },

                // item is not in old arena, it is new
                None => {

                    let other_value = other.get(*other_sync_index).unwrap();
                    
                    diff.altered.insert(
                        *other_sync_index, 
                        T::identity().diff(other_value)
                    );
                },
            }
        }

        diff
    }

    fn apply(&mut self, diff: &Self::Repr) {

        diff.removed.iter().for_each(|deleted_sync_index| {
        
            self.remove(*deleted_sync_index).expect("tried to remove an item that was already removed");
        });

        for (sync_index, item_diff) in &diff.altered {

            // item only changed (its already in the sync index map)
            if let Some(value) = self.get_mut(*sync_index) {

                value.apply(item_diff);

            // item is new (its not already in the sync index map)
            } else {
                self.insert_with_known_sync_id(T::identity().apply_new(item_diff), *sync_index);
            }
        }
    }

    fn identity() -> Self {
        Self::default()
    }
}
