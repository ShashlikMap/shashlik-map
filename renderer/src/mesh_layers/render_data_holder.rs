use linked_hash_map::{Entry, LinkedHashMap};

pub struct RenderDataHolder<T> {
    holder: LinkedHashMap<String, Vec<T>>,
}

impl<T> RenderDataHolder<T> {
    pub fn new() -> Self {
        Self {
            holder: LinkedHashMap::new(),
        }
    }

    pub fn add(&mut self, key: String, data: T) {
        match self.holder.entry(key) {
            Entry::Occupied(mut entry) => entry.get_mut().push(data),
            Entry::Vacant(entry) => {
                entry.insert(vec![data]);
            }
        };
    }

    pub fn remove(&mut self, key: String) {
        self.holder.remove(&key);
    }

    pub fn run_mut_action<F>(&mut self, mut block: F)
    where
        F: FnMut(&mut T),
    {
        self.holder.iter_mut().for_each(|(_, items)| {
            items.iter_mut().for_each(|item| {
                block(item);
            });
        });
    }
}
