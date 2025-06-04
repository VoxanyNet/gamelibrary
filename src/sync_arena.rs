use diff::Diff;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::{cmp, u32, u64};
use std::collections::{HashMap, HashSet};
use std::iter::{self, Extend, FromIterator, FusedIterator};
use std::mem;
use std::ops::{self};
use std::slice;
use std::vec;

const INVALID_U32: u32 = u32::MAX;

/// The `Arena` allows inserting and removing elements that are referred to by
/// `Index`.
/// 
/// [See the module-level documentation for example usage and motivation.](./index.html)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyncArena<T> {
    pub items: Vec<Entry<T>>,
    pub generation: u32,
    pub free_list_head: Option<u32>,
    pub len: usize,
    
    // this maps a global sync index to local index and generation
    // this is used to resolve the local index and generation with an Index struct
    pub sync_index_map: HashMap<u64, (u32, u32)>,
}


#[derive(Serialize, Deserialize)]
pub struct SyncArenaDiff<T>
where 
    T: Diff,
    T::Repr: Serialize + DeserializeOwned
{   
    pub altered: HashMap<u64, T::Repr>,
    pub removed: HashSet<u64>
}

impl<T> Diff for SyncArena<T>
where 
    T: Diff + PartialEq,
    T::Repr: Serialize + DeserializeOwned {
    type Repr = SyncArenaDiff<T>;

    fn diff(&self, other: &Self) -> Self::Repr {
        let mut diff: SyncArenaDiff<T> = SyncArenaDiff {
            altered: HashMap::new(),
            removed: HashSet::new()
        };
        
        
        for (index, item) in self.items.iter().enumerate() {
            if let Entry::Occupied { generation, sync_id,  value } = item {
                
                match other.get(&mut Index::from_raw_parts(index as u32, *generation, *sync_id)) {

                    // item is in both arenas
                    Some(other_value) => {
                        if other_value != value {
                            diff.altered.insert(
                                *sync_id, 
                                value.diff(other_value)
                            );
                        };
                    },

                    // item has been deleted
                    None => {
                        diff.removed.insert(*sync_id);
                    },
                }
            }
        }   

        for (other_index, other_item) in other.items.iter().enumerate() {
            if let Entry::Occupied { generation: other_generation, sync_id: other_sync_id, value: other_value } =  other_item {

                match self.get(&mut Index::from_raw_parts(other_index as u32, *other_generation, *other_sync_id)) {

                    // item is in both arenas (dont need to do anything)
                    Some(_) => {
                        
                    },

                    // item is not in old arena, it is new
                    None => {
                        diff.altered.insert(
                            *other_sync_id, 
                            T::identity().diff(other_value)
                        );
                    },
                }
            }
        }

        // for (sync_index, client_index) in &self.sync_index_map {

        //     // item is in both arenas
        //     if let Some(other_client_index) = other.sync_index_map.get(&sync_index) {
                
        //         // get the actual value
        //         let value = self.get(*client_index).unwrap(); // this guaranteed to be Some

        //         let other_value = other.get(*other_client_index).unwrap(); // this is guaranteed to be Some


        //         if value != other_value {
        //             diff.altered.insert(*sync_index, value.diff(other_value));
        //         };

        //     // item is not in other (removed)
        //     } else {
        //         diff.removed.insert(*sync_index);
        //     }
        // }

        // for (sync_index, client_index) in &other.sync_index_map {

        //     // item is not in self (its new)
        //     if let None = self.sync_index_map.get(sync_index) {

        //         let value = other.get(*client_index).unwrap();

        //         diff.altered.insert(*sync_index, T::identity().diff(value));
        //     }
        // };

        diff
    }

    fn apply(&mut self, diff: &Self::Repr) {
        // THIS IS WHERE THE SYNC IDs REALLY MATTER

        diff.removed.iter().for_each(|deleted_sync_index| {
            let (client_index, client_generation) = self.sync_index_map.get(deleted_sync_index).unwrap(); // we might actually want to check this if its already been deleted
            self.remove(Index::from_raw_parts(*client_index, *client_generation, *deleted_sync_index));
        });

        for (sync_index, item_diff) in &diff.altered {

            // item only changed (its already in the sync index map)
            if let Some((original_item_client_index, original_item_client_generation)) = self.sync_index_map.get(sync_index) {

                let original_item = self.get_mut(&mut Index::from_raw_parts(*original_item_client_index, *original_item_client_generation, *sync_index)).unwrap();

                original_item.apply(item_diff);

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize )]
pub enum Entry<T> {
    Free { next_free: Option<u32> },
    Occupied { generation: u32, sync_id: u64, value: T },
}

fn u32_max() -> u32 {
    u32::MAX
}

/// An index (and generation) into an `Arena`.
///
/// To get an `Index`, insert an element into an `Arena`, and the `Index` for
/// that element will be returned.
///
/// # Examples
///
/// ```ignore
/// use rapier::data::arena::Arena;
///
/// let mut arena = Arena::new();
/// let idx = arena.insert(123);
/// assert_eq!(arena[idx], 123);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Index {

    #[serde(skip_serializing, default = "u32_max")]
    index: u32,
    #[serde(skip_serializing, default = "u32_max")]
    generation: u32,
    // we need this because we cannot resolve the local indices of this sync index when applying the diff for the index (we dont have access to the data or the item might not even be in the arena yet)
    // the local index and generation are resolved
    #[serde(skip)]
    synced: bool, // index and generation are INVALID if this is false
    sync_id: u64,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct IndexDiff {
    sync_id: Option<u64>
}



impl Diff for Index {
    type Repr = IndexDiff;

    fn diff(&self, other: &Self) -> Self::Repr {

        let mut diff = Self::Repr {
            sync_id: None,
        };

        // we only need to check the sync id because this inequality implies that the local index has changed as well
        if self.sync_id != other.sync_id {
            diff.sync_id = Some(other.sync_id)
        };

        diff
    }

    fn apply(&mut self, diff: &Self::Repr) {
        
        if let Some(sync_id) = diff.sync_id {
            self.sync_id = sync_id;

            // this indicates that the local index and generation are no longer valid
            self.synced = false;

            // not really needed but will produce a runtime error that could be useful for debugging
            self.index = u32::MAX;
            self.generation = u32::MAX;
        }
    }

    fn identity() -> Self {
        <Index as Default>::default()
    }
}


impl Default for Index {
    fn default() -> Self {
        Self::from_raw_parts(INVALID_U32, INVALID_U32, INVALID_U32 as u64)
    }
}
impl Index {
    /// Create a new `Index` from its raw parts.
    ///
    /// The parts must have been returned from an earlier call to
    /// `into_raw_parts`.
    ///
    /// Providing arbitrary values will lead to malformed indices and ultimately
    /// panics.
    pub fn from_raw_parts(index: u32, generation: u32, sync_id: u64) -> Index {
        Index { 
            index,
            generation,
            sync_id,
            synced: true    
        }
    }

    /// Convert this `Index` into its raw parts.
    ///
    /// This niche method is useful for converting an `Index` into another
    /// identifier type. Usually, you should prefer a newtype wrapper around
    /// `Index` like `pub struct MyIdentifier(Index);`.  However, for external
    /// types whose definition you can't customize, but which you can construct
    /// instances of, this method can be useful.
    pub fn into_raw_parts(self) -> (u32, u32) {
        (self.index, self.generation)
    }
}

const DEFAULT_CAPACITY: usize = 4;

impl<T> Default for SyncArena<T> {
    fn default() -> SyncArena<T> {
        SyncArena::new()
    }
}

impl<T> SyncArena<T> {
    /// Constructs a new, empty `Arena`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::<usize>::new();
    /// # let _ = arena;
    /// ```
    pub fn new() -> SyncArena<T> {
        SyncArena::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn set_free_list_head(&mut self, free_list_head: u32) {
        self.free_list_head = Some(free_list_head);
    }

    /// Constructs a new, empty `Arena<T>` with the specified capacity.
    ///
    /// The `Arena<T>` will be able to hold `n` elements without further allocation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::with_capacity(10);
    ///
    /// // These insertions will not require further allocation.
    /// for i in 0..10 {
    ///     assert!(arena.try_insert(i).is_ok());
    /// }
    ///
    /// // But now we are at capacity, and there is no more room.
    /// assert!(arena.try_insert(99).is_err());
    /// ```
    pub fn with_capacity(n: usize) -> SyncArena<T> {
        let n = cmp::max(n, 1);
        let mut arena = SyncArena {
            sync_index_map: HashMap::new(),
            items: Vec::new(),
            generation: 0,
            free_list_head: None,
            len: 0
        };
        arena.reserve(n);
        arena
    }

    /// Clear all the items inside the arena, but keep its allocation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::with_capacity(1);
    /// arena.insert(42);
    /// arena.insert(43);
    ///
    /// arena.clear();
    ///
    /// assert_eq!(arena.capacity(), 2);
    /// ```
    pub fn clear(&mut self) {
        self.items.clear();

        let end = self.items.capacity() as u32;
        self.items.extend((0..end).map(|i| {
            if i == end - 1 {
                Entry::Free { next_free: None }
            } else {
                Entry::Free {
                    next_free: Some(i + 1),
                }
            }
        }));
        self.free_list_head = Some(0);
        self.len = 0;
    }

    /// Attempts to insert `value` into the arena using existing capacity.
    ///
    /// This method will never allocate new capacity in the arena.
    ///
    /// If insertion succeeds, then the `value`'s index is returned. If
    /// insertion fails, then `Err(value)` is returned to give ownership of
    /// `value` back to the caller.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    ///
    /// match arena.try_insert(42) {
    ///     Ok(idx) => {
    ///         // Insertion succeeded.
    ///         assert_eq!(arena[idx], 42);
    ///     }
    ///     Err(x) => {
    ///         // Insertion failed.
    ///         assert_eq!(x, 42);
    ///     }
    /// };
    /// ```
    #[inline]
    pub fn try_insert(&mut self, value: T) -> Result<Index, T> {
        match self.try_alloc_next_index() {
            None => Err(value),
            Some(index) => {
                self.items[index.index as usize] = Entry::Occupied {
                    generation: self.generation,
                    value,
                    sync_id: index.sync_id
                };
                Ok(index)
            }
        }
    }

    /// Attempts to insert the value returned by `create` into the arena using existing capacity.
    /// `create` is called with the new value's associated index, allowing values that know their own index.
    ///
    /// This method will never allocate new capacity in the arena.
    ///
    /// If insertion succeeds, then the new index is returned. If
    /// insertion fails, then `Err(create)` is returned to give ownership of
    /// `create` back to the caller.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::{Arena, Index};
    ///
    /// let mut arena = Arena::new();
    ///
    /// match arena.try_insert_with(|idx| (42, idx)) {
    ///     Ok(idx) => {
    ///         // Insertion succeeded.
    ///         assert_eq!(arena[idx].0, 42);
    ///         assert_eq!(arena[idx].1, idx);
    ///     }
    ///     Err(x) => {
    ///         // Insertion failed.
    ///     }
    /// };
    /// ```
    #[inline]
    pub fn try_insert_with<F: FnOnce(Index) -> T>(&mut self, create: F) -> Result<Index, F> {
        match self.try_alloc_next_index() {
            None => Err(create),
            Some(index) => {
                self.items[index.index as usize] = Entry::Occupied {
                    generation: self.generation,
                    value: create(index),
                    sync_id: index.sync_id
                };
                Ok(index)
            }
        }
    }

    #[inline]
    fn try_alloc_next_index(&mut self) -> Option<Index> {
        match self.free_list_head {
            None => None,
            Some(i) => match self.items[i as usize] {
                Entry::Occupied { .. } => panic!("corrupt free list"),
                Entry::Free { next_free } => {
                    self.free_list_head = next_free;
                    self.len += 1;
                    Some(Index {
                        index: i,
                        generation: self.generation,
                        sync_id: uuid::Uuid::new_v4().as_u64_pair().0,
                        synced: true
                    })
                }
            },
        }
    }

    /// Insert `value` into the arena, allocating more capacity if necessary.
    ///
    /// The `value`'s associated index in the arena is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    ///
    /// let idx = arena.insert(42);
    /// assert_eq!(arena[idx], 42);
    /// ```
    #[inline]
    pub fn insert(&mut self, value: T) -> Index {
        let index = match self.try_insert(value) {
            Ok(i) => i,
            Err(value) => self.insert_slow_path(value),
        };

        self.sync_index_map.insert(index.sync_id, (index.index, index.generation));

        index

    }

    #[inline]
    pub fn insert_with_known_sync_id(&mut self, value: T, sync_id: u64) -> Index {
        let mut index = match self.try_insert(value) {
            Ok(i) => i,
            Err(value) => self.insert_slow_path(value),
        };

        // This is a band-aid fix but its only about 1 microsecond
        // the entry itself contains the sync id but the internal insertion methods dont have a way of passing a sync id manually yet 
        if let Entry::Occupied { generation, sync_id: old_sync_id, value } = &mut self.items[index.index as usize] {
            *old_sync_id = sync_id
        }

        index.sync_id = sync_id;


        self.sync_index_map.insert(sync_id, (index.index, index.generation));

        index
    }


    /// Insert the value returned by `create` into the arena, allocating more capacity if necessary.
    /// `create` is called with the new value's associated index, allowing values that know their own index.
    ///
    /// The new value's associated index in the arena is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::{Arena, Index};
    ///
    /// let mut arena = Arena::new();
    ///
    /// let idx = arena.insert_with(|idx| (42, idx));
    /// assert_eq!(arena[idx].0, 42);
    /// assert_eq!(arena[idx].1, idx);
    /// ```
    #[inline]
    pub fn insert_with(&mut self, create: impl FnOnce(Index) -> T) -> Index {
        match self.try_insert_with(create) {
            Ok(i) => i,
            Err(create) => self.insert_with_slow_path(create),
        }
    }

    #[inline(never)]
    fn insert_slow_path(&mut self, value: T) -> Index {
        let len = self.items.len();
        self.reserve(len);
        self.try_insert(value)
            .map_err(|_| ())
            .expect("inserting will always succeed after reserving additional space")
    }

    #[inline(never)]
    fn insert_with_slow_path(&mut self, create: impl FnOnce(Index) -> T) -> Index {
        let len = self.items.len();
        self.reserve(len);
        self.try_insert_with(create)
            .map_err(|_| ())
            .expect("inserting will always succeed after reserving additional space")
    }

    /// Remove the element at index `i` from the arena.
    ///
    /// If the element at index `i` is still in the arena, then it is
    /// returned. If it is not in the arena, then `None` is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let idx = arena.insert(42);
    ///
    /// assert_eq!(arena.remove(idx), Some(42));
    /// assert_eq!(arena.remove(idx), None);
    /// ```
    pub fn remove(&mut self, i: Index) -> Option<T> {
        if i.index >= self.items.len() as u32 {
            return None;
        }

        match self.items[i.index as usize] {
            Entry::Occupied { generation, .. } if i.generation == generation => {
                let entry = mem::replace(
                    &mut self.items[i.index as usize],
                    Entry::Free {
                        next_free: self.free_list_head,
                    },
                );

                self.generation += 1;
                self.free_list_head = Some(i.index);
                self.len -= 1;

                match entry {
                    Entry::Occupied {
                        generation: _,
                        value,
                        sync_id,
                    } => {
                        
                        self.sync_index_map.remove(&sync_id).expect("could not find sync index in sync_index map when removing");

                        Some(value)
                    },
                    _ => unreachable!(),
                }
            }
            _ => None,
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all indices such that `predicate(index, &value)` returns `false`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut crew = Arena::new();
    /// crew.extend(&["Jim Hawkins", "John Silver", "Alexander Smollett", "Israel Hands"]);
    /// let pirates = ["John Silver", "Israel Hands"]; // too dangerous to keep them around
    /// crew.retain(|_index, member| !pirates.contains(member));
    /// let mut crew_members = crew.iter().map(|(_, member)| **member);
    /// assert_eq!(crew_members.next(), Some("Jim Hawkins"));
    /// assert_eq!(crew_members.next(), Some("Alexander Smollett"));
    /// assert!(crew_members.next().is_none());
    /// ```
    pub fn retain(&mut self, mut predicate: impl FnMut(Index, &mut T) -> bool) {
        for i in 0..self.capacity() as u32 {
            let remove = match &mut self.items[i as usize] {
                Entry::Occupied { generation, value, sync_id } => {
                    let index = Index {
                        index: i,
                        generation: *generation,
                        sync_id: *sync_id,
                        synced: true
                    };
                    if predicate(index, value) {
                        None
                    } else {
                        Some(index)
                    }
                }

                _ => None,
            };
            if let Some(index) = remove {
                self.remove(index);
            }
        }
    }

    /// Is the element at index `i` in the arena?
    ///
    /// Returns `true` if the element at `i` is in the arena, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let idx = arena.insert(42);
    ///
    /// assert!(arena.contains(idx));
    /// arena.remove(idx);
    /// assert!(!arena.contains(idx));
    /// ```
    pub fn contains(&self, i: &mut Index) -> bool {
        self.get(i).is_some()
    }

    /// Get a shared reference to the element at index `i` if it is in the
    /// arena.
    ///
    /// If the element at index `i` is not in the arena, then `None` is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let idx = arena.insert(42);
    ///
    /// assert_eq!(arena.get(idx), Some(&42));
    /// arena.remove(idx);
    /// assert!(arena.get(idx).is_none());
    /// ```
    pub fn get(&self, i: &mut Index) -> Option<&T> {

        // we need to resolve the local index
        if i.synced == false {

            let (local_index, local_generation) = self.sync_index_map.get(&i.sync_id).unwrap();

            i.index = *local_index;

            i.generation = *local_generation;

            i.synced = true;

        }

        match self.items.get(i.index as usize) {
            // we dont need to check the sync id because a given local index will only ever match to one sync id
            Some(Entry::Occupied { generation, value, sync_id: _ }) if *generation == i.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    /// Get an exclusive reference to the element at index `i` if it is in the
    /// arena.
    ///
    /// If the element at index `i` is not in the arena, then `None` is returned.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let idx = arena.insert(42);
    ///
    /// *arena.get_mut(idx).unwrap() += 1;
    /// assert_eq!(arena.remove(idx), Some(43));
    /// assert!(arena.get_mut(idx).is_none());
    /// ```
    pub fn get_mut(&mut self, i: &mut Index) -> Option<&mut T> {

         // we need to resolve the local index
        if i.synced == false {
            let (local_index, local_generation) = self.sync_index_map.get(&i.sync_id).unwrap();

            i.index = *local_index;

            i.generation = *local_generation;

            i.synced = true;

        }

        match self.items.get_mut(i.index as usize) {
            Some(Entry::Occupied { generation, value, sync_id: _ }) if *generation == i.generation => {
                Some(value)
            }
            _ => None,
        }
    }

    /// Get a pair of exclusive references to the elements at index `i1` and `i2` if it is in the
    /// arena.
    ///
    /// If the element at index `i1` or `i2` is not in the arena, then `None` is returned for this
    /// element.
    ///
    /// # Panics
    ///
    /// Panics if `i1` and `i2` are pointing to the same item of the arena.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let idx1 = arena.insert(0);
    /// let idx2 = arena.insert(1);
    ///
    /// {
    ///     let (item1, item2) = arena.get2_mut(idx1, idx2);
    ///
    ///     *item1.unwrap() = 3;
    ///     *item2.unwrap() = 4;
    /// }
    ///
    /// assert_eq!(arena[idx1], 3);
    /// assert_eq!(arena[idx2], 4);
    /// ```
    pub fn get2_mut(&mut self, i1: &mut Index, i2: &mut Index) -> (Option<&mut T>, Option<&mut T>) {
        let len = self.items.len() as u32;

        if i1.index == i2.index {
            assert!(i1.generation != i2.generation);

            if i1.generation > i2.generation {
                return (self.get_mut(i1), None);
            }
            return (None, self.get_mut(i2));
        }

        if i1.index >= len {
            return (None, self.get_mut(i2));
        } else if i2.index >= len {
            return (self.get_mut(i1), None);
        }

        let (raw_item1, raw_item2) = {
            let (xs, ys) = self
                .items
                .split_at_mut(cmp::max(i1.index, i2.index) as usize);
            if i1.index < i2.index {
                (&mut xs[i1.index as usize], &mut ys[0])
            } else {
                (&mut ys[0], &mut xs[i2.index as usize])
            }
        };

        let item1 = match raw_item1 {
            Entry::Occupied { generation, value, sync_id } if *generation == i1.generation => Some(value),
            _ => None,
        };

        let item2 = match raw_item2 {
            Entry::Occupied { generation, value, sync_id } if *generation == i2.generation => Some(value),
            _ => None,
        };

        (item1, item2)
    }

    /// Get the length of this arena.
    ///
    /// The length is the number of elements the arena holds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// assert_eq!(arena.len(), 0);
    ///
    /// let idx = arena.insert(42);
    /// assert_eq!(arena.len(), 1);
    ///
    /// let _ = arena.insert(0);
    /// assert_eq!(arena.len(), 2);
    ///
    /// assert_eq!(arena.remove(idx), Some(42));
    /// assert_eq!(arena.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the arena contains no elements
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// assert!(arena.is_empty());
    ///
    /// let idx = arena.insert(42);
    /// assert!(!arena.is_empty());
    ///
    /// assert_eq!(arena.remove(idx), Some(42));
    /// assert!(arena.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity of this arena.
    ///
    /// The capacity is the maximum number of elements the arena can hold
    /// without further allocation, including however many it currently
    /// contains.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::with_capacity(10);
    /// assert_eq!(arena.capacity(), 10);
    ///
    /// // `try_insert` does not allocate new capacity.
    /// for i in 0..10 {
    ///     assert!(arena.try_insert(1).is_ok());
    ///     assert_eq!(arena.capacity(), 10);
    /// }
    ///
    /// // But `insert` will if the arena is already at capacity.
    /// arena.insert(0);
    /// assert!(arena.capacity() > 10);
    /// ```
    pub fn capacity(&self) -> usize {
        self.items.len()
    }

    /// Allocate space for `additional_capacity` more elements in the arena.
    ///
    /// # Panics
    ///
    /// Panics if this causes the capacity to overflow.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::with_capacity(10);
    /// arena.reserve(5);
    /// assert_eq!(arena.capacity(), 15);
    /// # let _: Arena<usize> = arena;
    /// ```
    pub fn reserve(&mut self, additional_capacity: usize) {
        let start = self.items.len();
        let end = self.items.len() + additional_capacity;
        let old_head = self.free_list_head;
        self.items.reserve_exact(additional_capacity);
        self.items.extend((start..end).map(|i| {
            if i == end - 1 {
                Entry::Free {
                    next_free: old_head,
                }
            } else {
                Entry::Free {
                    next_free: Some(i as u32 + 1),
                }
            }
        }));
        self.free_list_head = Some(start as u32);
    }

    /// Iterate over shared references to the elements in this arena.
    ///
    /// Yields pairs of `(Index, &T)` items.
    ///
    /// Order of iteration is not defined.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// for i in 0..10 {
    ///     arena.insert(i * i);
    /// }
    ///
    /// for (idx, value) in arena.iter() {
    ///     println!("{} is at index {:?}", value, idx);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<T> {
        Iter {
            len: self.len,
            inner: self.items.iter().enumerate(),
        }
    }

    /// Iterate over exclusive references to the elements in this arena.
    ///
    /// Yields pairs of `(Index, &mut T)` items.
    ///
    /// Order of iteration is not defined.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// for i in 0..10 {
    ///     arena.insert(i * i);
    /// }
    ///
    /// for (_idx, value) in arena.iter_mut() {
    ///     *value += 5;
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            len: self.len,
            inner: self.items.iter_mut().enumerate(),
        }
    }

    /// Iterate over elements of the arena and remove them.
    ///
    /// Yields pairs of `(Index, T)` items.
    ///
    /// Order of iteration is not defined.
    ///
    /// Note: All elements are removed even if the iterator is only partially consumed or not consumed at all.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rapier::data::arena::Arena;
    ///
    /// let mut arena = Arena::new();
    /// let idx_1 = arena.insert("hello");
    /// let idx_2 = arena.insert("world");
    ///
    /// assert!(arena.get(idx_1).is_some());
    /// assert!(arena.get(idx_2).is_some());
    /// for (idx, value) in arena.drain() {
    ///     assert!((idx == idx_1 && value == "hello") || (idx == idx_2 && value == "world"));
    /// }
    /// assert!(arena.get(idx_1).is_none());
    /// assert!(arena.get(idx_2).is_none());
    /// ```
    pub fn drain(&mut self) -> Drain<T> {
        Drain {
            inner: self.items.drain(..).enumerate(),
        }
    }

    /// Given an i of `usize` without a generation, get a shared reference
    /// to the element and the matching `Index` of the entry behind `i`.
    ///
    /// This method is useful when you know there might be an element at the
    /// position i, but don't know its generation or precise Index.
    ///
    /// Use cases include using indexing such as Hierarchical BitMap Indexing or
    /// other kinds of bit-efficient indexing.
    ///
    /// You should use the `get` method instead most of the time.
    pub fn get_unknown_gen(&self, i: u32) -> Option<(&T, Index)> {
        match self.items.get(i as usize) {
            Some(Entry::Occupied { generation, value, sync_id }) => Some((
                value,
                Index {
                    generation: *generation,
                    index: i,
                    sync_id: *sync_id,
                    synced: true
                },
            )),
            _ => None,
        }
    }

    /// Given an i of `usize` without a generation, get an exclusive reference
    /// to the element and the matching `Index` of the entry behind `i`.
    ///
    /// This method is useful when you know there might be an element at the
    /// position i, but don't know its generation or precise Index.
    ///
    /// Use cases include using indexing such as Hierarchical BitMap Indexing or
    /// other kinds of bit-efficient indexing.
    ///
    /// You should use the `get_mut` method instead most of the time.
    pub fn get_unknown_gen_mut(&mut self, i: u32) -> Option<(&mut T, Index)> {
        match self.items.get_mut(i as usize) {
            Some(Entry::Occupied { generation, value, sync_id }) => Some((
                value,
                Index {
                    generation: *generation,
                    index: i,
                    synced: true,
                    sync_id: *sync_id
                },
            )),
            _ => None,
        }
    }
}

impl<T> IntoIterator for SyncArena<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            len: self.len,
            inner: self.items.into_iter(),
        }
    }
}

/// An iterator over the elements in an arena.
///
/// Yields `T` items.
///
/// Order of iteration is not defined.
///
/// # Examples
///
/// ```ignore
/// use rapier::data::arena::Arena;
///
/// let mut arena = Arena::new();
/// for i in 0..10 {
///     arena.insert(i * i);
/// }
///
/// for value in arena {
///     assert!(value < 100);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct IntoIter<T> {
    len: usize,
    inner: vec::IntoIter<Entry<T>>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(Entry::Free { .. }) => continue,
                Some(Entry::Occupied { value, .. }) => {
                    self.len -= 1;
                    return Some(value);
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next_back() {
                Some(Entry::Free { .. }) => continue,
                Some(Entry::Occupied { value, .. }) => {
                    self.len -= 1;
                    return Some(value);
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<T> FusedIterator for IntoIter<T> {}

impl<'a, T> IntoIterator for &'a SyncArena<T> {
    type Item = (Index, &'a T);
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over shared references to the elements in an arena.
///
/// Yields pairs of `(Index, &T)` items.
///
/// Order of iteration is not defined.
///
/// # Examples
///
/// ```ignore
/// use rapier::data::arena::Arena;
///
/// let mut arena = Arena::new();
/// for i in 0..10 {
///     arena.insert(i * i);
/// }
///
/// for (idx, value) in &arena {
///     println!("{} is at index {:?}", value, idx);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Iter<'a, T: 'a> {
    len: usize,
    inner: iter::Enumerate<slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Index, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some((_, &Entry::Free { .. })) => continue,
                Some((
                    index,
                    &Entry::Occupied {
                        generation,
                        ref value,
                        sync_id,
                    },
                )) => {
                    self.len -= 1;
                    let idx = Index {
                        index: index as u32,
                        generation,
                        sync_id,
                        synced: true
                    };
                    return Some((idx, value));
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next_back() {
                Some((_, &Entry::Free { .. })) => continue,
                Some((
                    index,
                    &Entry::Occupied {
                        generation,
                        ref value, 
                        sync_id },
                )) => {
                    self.len -= 1;
                    let idx = Index {
                        index: index as u32,
                        generation,
                        sync_id,
                        synced: true
                    };
                    return Some((idx, value));
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

impl<'a, T> IntoIterator for &'a mut SyncArena<T> {
    type Item = (Index, &'a mut T);
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over exclusive references to elements in this arena.
///
/// Yields pairs of `(Index, &mut T)` items.
///
/// Order of iteration is not defined.
///
/// # Examples
///
/// ```ignore
/// use rapier::data::arena::Arena;
///
/// let mut arena = Arena::new();
/// for i in 0..10 {
///     arena.insert(i * i);
/// }
///
/// for (_idx, value) in &mut arena {
///     *value += 5;
/// }
/// ```
#[derive(Debug)]
pub struct IterMut<'a, T: 'a> {
    len: usize,
    inner: iter::Enumerate<slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Index, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some((_, &mut Entry::Free { .. })) => continue,
                Some((
                    index,
                    &mut Entry::Occupied {
                        generation,
                        ref mut value,
                        sync_id
                    },
                )) => {
                    self.len -= 1;
                    let idx = Index {
                        index: index as u32,
                        generation,
                        sync_id,
                        synced: true
                    };
                    return Some((idx, value));
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next_back() {
                Some((_, &mut Entry::Free { .. })) => continue,
                Some((
                    index,
                    &mut Entry::Occupied {
                        generation,
                        ref mut value, 
                        sync_id 
                    },
                )) => {
                    self.len -= 1;
                    let idx = Index {
                        index: index as u32,
                        generation,
                        sync_id,
                        synced: true
                    };
                    return Some((idx, value));
                }
                None => {
                    debug_assert_eq!(self.len, 0);
                    return None;
                }
            }
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}

/// An iterator that removes elements from the arena.
///
/// Yields pairs of `(Index, T)` items.
///
/// Order of iteration is not defined.
///
/// Note: All elements are removed even if the iterator is only partially consumed or not consumed at all.
///
/// # Examples
///
/// ```ignore
/// use rapier::data::arena::Arena;
///
/// let mut arena = Arena::new();
/// let idx_1 = arena.insert("hello");
/// let idx_2 = arena.insert("world");
///
/// assert!(arena.get(idx_1).is_some());
/// assert!(arena.get(idx_2).is_some());
/// for (idx, value) in arena.drain() {
///     assert!((idx == idx_1 && value == "hello") || (idx == idx_2 && value == "world"));
/// }
/// assert!(arena.get(idx_1).is_none());
/// assert!(arena.get(idx_2).is_none());
/// ```
#[derive(Debug)]
pub struct Drain<'a, T: 'a> {
    inner: iter::Enumerate<vec::Drain<'a, Entry<T>>>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = (Index, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some((_, Entry::Free { .. })) => continue,
                Some((index, Entry::Occupied { generation, value, sync_id })) => {
                    let idx = Index {
                        index: index as u32,
                        generation,
                        sync_id,
                        synced: true
                    };
                    return Some((idx, value));
                }
                None => return None,
            }
        }
    }
}

impl<T> Extend<T> for SyncArena<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for t in iter {
            self.insert(t);
        }
    }
}

impl<T> FromIterator<T> for SyncArena<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        let cap = upper.unwrap_or(lower);
        let cap = cmp::max(cap, 1);
        let mut arena = SyncArena::with_capacity(cap);
        arena.extend(iter);
        arena
    }
}

impl<T> ops::Index<&mut Index> for SyncArena<T> {
    type Output = T;

    fn index(&self, index: &mut Index) -> &Self::Output {
        self.get(index).expect("No element at index")
    }
}

impl<T> ops::IndexMut<&mut Index> for SyncArena<T> {
    fn index_mut(&mut self, index: &mut Index) -> &mut Self::Output {
        self.get_mut(index).expect("No element at index")
    }
}
