use indexmap::IndexMap;

#[derive(Clone)]
pub struct RenderDataHolder<T> {
    holder: IndexMap<String, Vec<T>>,
}

impl<T> RenderDataHolder<T> {
    pub fn new() -> Self {
        Self {
            holder: IndexMap::new(),
        }
    }

    pub fn set(&mut self, key: String, data: Vec<T>) {
        self.holder.insert(key, data);
    }
    
    pub fn remove(&mut self, key: &str) {
        // shift_remove is O(n) operation
        self.holder.shift_remove(&key.to_string());
    }

    pub fn run_mut_action_with_key<F>(&mut self, key: &str, mut block: F)
    where
        F: FnMut(&mut T),
    {
        if let Some(items) = self.holder.get_mut(key) {
            items.iter_mut().for_each(&mut block)
        }
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
