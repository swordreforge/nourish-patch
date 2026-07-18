use compositor_support_system_storage_token_base::base::{Token, TokenMut};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ComponentId(pub Uuid);

pub type OrderKey = f64;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct DrawLayer(pub i32);

impl DrawLayer {
    pub const CONTENT: DrawLayer = DrawLayer(0);
    pub const GROUP: DrawLayer = DrawLayer(-100);
    pub const OVERLAY: DrawLayer = DrawLayer(100);
}

#[derive(Default)]
pub struct DrawOrder {
    entries: Vec<(ComponentId, DrawLayer, OrderKey)>,
    index: HashMap<ComponentId, usize>,
    next: OrderKey,
}

impl DrawOrder {
    pub fn new() -> Self {
        Self { entries: Vec::new(), index: HashMap::new(), next: 1.0 }
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, (id, _, _)) in self.entries.iter().enumerate() {
            self.index.insert(*id, i);
        }
    }

    pub fn insert_top(&mut self, id: ComponentId, layer: DrawLayer) -> OrderKey {
        let key = self.next;
        self.next += 1.0;
        if let Some(&idx) = self.index.get(&id) {
            self.entries[idx] = (id, layer, key);
        } else {
            let idx = self.entries.len();
            self.entries.push((id, layer, key));
            self.index.insert(id, idx);
        }
        key
    }

    pub fn remove(&mut self, id: ComponentId) {
        if let Some(idx) = self.index.remove(&id) {
            self.entries.remove(idx);
            self.rebuild_index();
        }
    }

    pub fn reassign(&mut self, old: ComponentId, new: ComponentId) -> bool {
        if old == new { return self.key(old).is_some(); }
        if let Some(new_idx) = self.index.remove(&new) {
            self.entries.remove(new_idx);
            self.rebuild_index();
        }
        let Some(&idx) = self.index.get(&old) else { return false; };
        self.entries[idx].0 = new;
        self.index.remove(&old);
        self.index.insert(new, idx);
        true
    }

    pub fn raise(&mut self, id: ComponentId) {
        if let Some(&idx) = self.index.get(&id) {
            let key = self.next;
            self.next += 1.0;
            self.entries[idx].2 = key;
        } else {
            self.insert_top(id, DrawLayer::CONTENT);
        }
    }

    pub fn key(&self, id: ComponentId) -> Option<OrderKey> {
        self.index.get(&id).map(|&idx| self.entries[idx].2)
    }

    pub fn ordered(&self) -> Vec<(ComponentId, OrderKey)> {
        let mut v = self.entries.clone();
        v.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.total_cmp(&b.2)));
        v.into_iter().map(|(c, _, k)| (c, k)).collect()
    }
}

pub static DRAW_ORDER: Token<DrawOrder> = Token::new();
pub static DRAW_ORDER_MUT: TokenMut<DrawOrder> = TokenMut::new(&DRAW_ORDER);
