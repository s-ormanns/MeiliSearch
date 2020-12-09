use std::collections::BTreeMap;

use crate::{FieldId, IndexedPos};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionMap {
    pos_to_field: Vec<FieldId>,
    field_to_pos: BTreeMap<FieldId, IndexedPos>,
}

impl PositionMap {
    /// Inserts `id` at `pos`. If a value was present, returns `Some(id)`.
    /// panics if pos > self.len()
    pub fn insert(&mut self, id: FieldId, pos: IndexedPos) -> Option<FieldId> {
        let upos = pos.0 as usize;
        assert!(upos <= self.len());
        if upos != self.len() {
            let old = self.pos_to_field[upos];
            self.pos_to_field[upos] = id;
            self.field_to_pos.remove(&old);
            self.field_to_pos.insert(id, pos);
            Some(old)
        } else {
            self.push(id);
            None
        }
    }

    /// Pushes `id` in last position
    pub fn push(&mut self, id: FieldId) {
        let pos = self.len();
        self.pos_to_field.push(id);
        self.field_to_pos.insert(id, IndexedPos(pos as u16));
    }

    pub fn len(&self) -> usize {
        self.pos_to_field.len()
    }

    pub fn field_to_pos(&self, id: FieldId) -> Option<IndexedPos> {
        self.field_to_pos.get(&id).cloned()
    }

    pub fn pos_to_field(&self, pos: IndexedPos) -> Option<FieldId> {
        let pos = pos.0 as usize;
        self.pos_to_field.get(pos).cloned()
    }

    pub fn field_pos(&self) -> impl Iterator<Item = (FieldId, IndexedPos)> + '_ {
        self.pos_to_field
            .iter()
            .enumerate()
            .map(|(i, f)| (*f, IndexedPos(i as u16)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use k9::*;

    #[test]
    fn test_default() {
        assert_matches_inline_snapshot!(
            format!("{:?}", PositionMap::default()),
            r##"PositionMap { pos_to_field: [], field_to_pos: {} }"##
        );
    }

    #[test]
    fn test_insert() {
        let mut map = PositionMap::default();
        assert_eq!(map.insert(0.into(), 0.into()), None);
        assert_matches_inline_snapshot!(
            format!("{:?}", map),
            r##"PositionMap { pos_to_field: [FieldId(0)], field_to_pos: {FieldId(0): IndexedPos(0)} }"##
        );
        assert_eq!(map.insert(1.into(), 0.into()), Some(0.into()));
        assert_matches_inline_snapshot!(
            format!("{:?}", map),
            r##"PositionMap { pos_to_field: [FieldId(1)], field_to_pos: {FieldId(1): IndexedPos(0)} }"##
        );
    }

    #[test]
    fn test_push() {
        let mut map = PositionMap::default();
        map.push(0.into());
        map.push(2.into());
        assert_eq!(map.len(), 2);
        assert_matches_inline_snapshot!(
            format!("{:?}", map),
            r##"PositionMap { pos_to_field: [FieldId(0), FieldId(2)], field_to_pos: {FieldId(0): IndexedPos(0), FieldId(2): IndexedPos(1)} }"##
        );
    }

    #[test]
    #[should_panic]
    fn test_insert_out_of_bounds() {
        let mut map = PositionMap::default();
        map.insert(0.into(), 2.into());
    }

    #[test]
    fn test_field_to_pos() {
        let mut map = PositionMap::default();
        map.push(0.into());
        map.push(2.into());
        assert_eq!(map.field_to_pos(2.into()), Some(1.into()));
        assert_eq!(map.field_to_pos(0.into()), Some(0.into()));
        assert_eq!(map.field_to_pos(4.into()), None);
    }

    #[test]
    fn test_pos_to_field() {
        let mut map = PositionMap::default();
        map.push(0.into());
        map.push(2.into());
        assert_eq!(map.pos_to_field(0.into()), Some(0.into()));
        assert_eq!(map.pos_to_field(1.into()), Some(2.into()));
        assert_eq!(map.pos_to_field(3.into()), None);
    }

    #[test]
    fn test_field_pos() {
        let mut map = PositionMap::default();
        map.push(0.into());
        map.push(2.into());
        let mut iter = map.field_pos();
        assert_eq!(iter.next(), Some((0.into(), 0.into())));
        assert_eq!(iter.next(), Some((2.into(), 1.into())));
        assert_eq!(iter.next(), None);
    }
}
