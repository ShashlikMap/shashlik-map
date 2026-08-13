use rustc_hash::FxHashMap;
use wgpu::{BindGroup, Device};
use crate::mesh_buffers::UniqueBufferId;

#[derive(Eq, PartialEq, Hash)]
pub(crate) struct BindGroupKey {
    layout_id: usize,
    inner: Vec<UniqueBufferId>,
}

impl BindGroupKey {
    pub fn new(layout_id: UniqueBufferId, buffer_ids: &[usize]) -> Self {
        let mut ids = buffer_ids.to_vec();
        ids.sort();
        BindGroupKey { layout_id, inner: ids }
    }
}

pub(crate) struct BindGroupCache {
    device: Device,
    bind_groups: FxHashMap<BindGroupKey, BindGroup>,
    accessed: bool
}

impl BindGroupCache {
    pub fn new(device: &Device) -> Self {
        Self {
            bind_groups: FxHashMap::default(),
            device: device.clone(),
            accessed: false
        }
    }

    pub fn get_bind_group_or_create(
        &mut self,
        bind_group_key: BindGroupKey,
        action: impl FnOnce(&Device) -> BindGroup,
    ) -> &BindGroup {
        self.accessed = true;
        self.bind_groups
            .entry(bind_group_key)
            .or_insert_with(|| action(&self.device))
    }

    /// If the cache hasn't been accessed between frames, clear it!
    pub fn clear_if_needed(&mut self) {
        if !self.accessed && !self.bind_groups.is_empty() {
            self.bind_groups.clear();
        }
        self.accessed = false;
    }
}
