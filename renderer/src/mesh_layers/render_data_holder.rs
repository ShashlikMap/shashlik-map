use linked_hash_map::LinkedHashMap;

pub struct RenderDataHolder<T> {
    pub holder: LinkedHashMap<String, T>,
}

impl<T> RenderDataHolder<T> {
    pub fn new() -> Self {
        Self {
            holder: LinkedHashMap::new(),
        }
    }

    pub fn add(&mut self, key: String, data: T) {
        self.holder.insert(key, data);
    }

    pub fn remove(&mut self, key: String) {
        self.holder.remove(&key);
    }
}
