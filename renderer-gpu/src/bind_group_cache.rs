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
}

impl BindGroupCache {
    pub fn new(device: &Device) -> Self {
        Self {
            bind_groups: FxHashMap::default(),
            device: device.clone(),
        }
    }

    pub fn get_bind_group_or_create(
        &mut self,
        bind_group_key: BindGroupKey,
        action: impl FnOnce(&Device) -> BindGroup,
    ) -> &BindGroup {
        self.bind_groups
            .entry(bind_group_key)
            .or_insert_with(|| action(&self.device))
    }
}
