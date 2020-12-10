use std::collections::BTreeMap;

use crate::{FieldId, IndexedPos};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionMap {
    pos_to_field: Vec<FieldId>,
    field_to_pos: BTreeMap<FieldId, IndexedPos>,
}

impl PositionMap {
    pub fn insert(&mut self, id: FieldId, pos: IndexedPos) {
        let mut upos = pos.0 as usize;
        if let Some(old_pos) = self.field_to_pos.get(&id) {
            let uold_pos = old_pos.0 as usize;
            self.pos_to_field.remove(uold_pos);
            if uold_pos < upos {
                upos += 1;
            }
        }

        if upos < self.len() {
            self.pos_to_field.insert(upos, id);
        } else {
            self.push(id);
        };

        self.field_to_pos.clear();
        self.field_to_pos.extend(
            self.pos_to_field
                .iter()
                .enumerate()
                .map(|(p, f)| (*f, IndexedPos(p as u16))),
        );
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
        // changing position removes from old position
        map.insert(0.into(), 0.into());
        map.insert(1.into(), 1.into());
        assert_matches_inline_snapshot!(
            format!("{:?}", map),
            r##"PositionMap { pos_to_field: [FieldId(0), FieldId(1)], field_to_pos: {FieldId(0): IndexedPos(0), FieldId(1): IndexedPos(1)} }"##
        );
        map.insert(0.into(), 1.into());
        assert_matches_inline_snapshot!(
            format!("{:?}", map),
            r##"PositionMap { pos_to_field: [FieldId(1), FieldId(0)], field_to_pos: {FieldId(0): IndexedPos(1), FieldId(1): IndexedPos(0)} }"##
        );
        map.insert(2.into(), 1.into());
        assert_matches_inline_snapshot!(
            format!("{:?}", map),
            r##"PositionMap { pos_to_field: [FieldId(1), FieldId(2), FieldId(0)], field_to_pos: {FieldId(0): IndexedPos(2), FieldId(1): IndexedPos(0), FieldId(2): IndexedPos(1)} }"##
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
