use crate::{FieldsMap, FieldId, SResult, Error, IndexedPos};
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::borrow::Cow;

use crate::position_map::PositionMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum OptionAll<T> {
    All,
    Some(T),
    None,
}

impl<T> OptionAll<T> {
    // replace the value with None and return the previous value
    pub(crate) fn take(&mut self) -> OptionAll<T> {
        std::mem::replace(self, OptionAll::None)
    }

    pub(crate) fn map<U, F: FnOnce(T) -> U>(self, f: F) -> OptionAll<U> {
        match self {
            OptionAll::Some(x) => OptionAll::Some(f(x)),
            OptionAll::All => OptionAll::All,
            OptionAll::None => OptionAll::None,
        }
    }

    pub(crate) fn is_all(&self) -> bool {
        matches!(self, OptionAll::All)
    }
}

impl<T> Default for OptionAll<T> {
    fn default() -> OptionAll<T> {
        OptionAll::All
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Schema {
    fields_map: FieldsMap,

    primary_key: Option<FieldId>,
    ranked: HashSet<FieldId>,
    displayed: OptionAll<HashSet<FieldId>>,

    searchable: OptionAll<Vec<FieldId>>,
    indexed_position: PositionMap,
}

impl Schema {
    pub fn with_primary_key(name: &str) -> Schema {
        let mut fields_map = FieldsMap::default();
        let field_id = fields_map.insert(name).unwrap();
        let indexed_position = PositionMap::default();

        Schema {
            fields_map,
            primary_key: Some(field_id),
            ranked: HashSet::new(),
            displayed: OptionAll::All,
            searchable: OptionAll::All,
            indexed_position,
        }
    }

    pub fn primary_key(&self) -> Option<&str> {
        self.primary_key.map(|id| self.fields_map.name(id).unwrap())
    }

    pub fn set_primary_key(&mut self, name: &str) -> SResult<FieldId> {
        if self.primary_key.is_some() {
            return Err(Error::PrimaryKeyAlreadyPresent)
        }

        let id = self.insert(name)?;
        self.primary_key = Some(id);

        Ok(id)
    }

    pub fn id(&self, name: &str) -> Option<FieldId> {
        self.fields_map.id(name)
    }

    pub fn name<I: Into<FieldId>>(&self, id: I) -> Option<&str> {
        self.fields_map.name(id)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.fields_map.iter().map(|(k, _)| k.as_ref())
    }

    /// add `name` to the list of known fields
    pub fn insert(&mut self, name: &str) -> SResult<FieldId> {
        self.fields_map.insert(name)
    }


    /// Adds `name` to the list of known fields, and in the last position of the indexed_position map. This
    /// field is taken into acccount when `searchableAttribute` or `displayedAttributes` is set to `"*"`
    pub fn insert_with_position(&mut self, name: &str) -> SResult<(FieldId, IndexedPos)> {
        let field_id = self.fields_map.insert(name)?;
        Ok((field_id, self.insert_position_last(field_id)))
    }

    fn insert_position_last(&mut self, id: FieldId) -> IndexedPos {
        let position = self.indexed_position.len() as u16;
        self.indexed_position.push(id);
        position.into()
    }

    pub fn ranked(&self) -> &HashSet<FieldId> {
        &self.ranked
    }

    fn displayed(&self) -> Cow<HashSet<FieldId>> {
        match self.displayed {
            OptionAll::Some(ref v) => Cow::Borrowed(v),
            OptionAll::All => {
                let fields = self
                    .fields_map
                    .iter()
                    .map(|(_, &v)| v)
                    .collect();
                Cow::Owned(fields)
            }
            OptionAll::None => Cow::Owned(HashSet::new())
        }
    }

    pub fn is_displayed_all(&self) -> bool {
        self.displayed.is_all()
    }

    pub fn displayed_names(&self) -> HashSet<&str> {
        self.displayed().iter().filter_map(|&f| self.name(f)).collect()
    }

    fn searchable_attributes(&self) -> Cow<[FieldId]> {
        match self.searchable {
            OptionAll::Some(ref v) => Cow::Borrowed(v),
            OptionAll::All => {
                let fields = self
                    .indexed_position
                    .field_pos()
                    .map(|(f, _)| f)
                    .collect();
                Cow::Owned(fields)
            },
            OptionAll::None => Cow::Owned(Vec::new())
        }
    }

    pub fn searchable_attributes_str(&self) -> Vec<&str> {
        self
            .searchable_attributes()
            .iter()
            .filter_map(|a| self.name(*a))
            .collect()
    }

    pub(crate) fn set_ranked(&mut self, name: &str) -> SResult<FieldId> {
        let id = self.fields_map.insert(name)?;
        self.ranked.insert(id);
        Ok(id)
    }

    fn set_displayed(&mut self, name: &str) -> SResult<FieldId> {
        let id = self.fields_map.insert(name)?;
        self.displayed = match self.displayed.take() {
            OptionAll::All => OptionAll::All,
            OptionAll::None => {
                let mut displayed = HashSet::new();
                displayed.insert(id);
                OptionAll::Some(displayed)
            },
            OptionAll::Some(mut v) => {
                v.insert(id);
                OptionAll::Some(v)
            }
        };
        Ok(id)
    }


    pub fn clear_ranked(&mut self) {
        self.ranked.clear();
    }

    pub fn is_ranked(&self, id: FieldId) -> bool {
        self.ranked.get(&id).is_some()
    }

    pub fn is_displayed(&self, id: FieldId) -> bool {
        match self.displayed {
            OptionAll::Some(ref v) => v.contains(&id),
            OptionAll::All => true,
            OptionAll::None => false,
        }
    }

    pub fn get_position(&self, id: FieldId) -> Option<IndexedPos> {
        self.indexed_position.field_to_pos(id)
    }

    pub fn is_searchable_all(&self) -> bool {
        self.searchable.is_all()
    }

    pub fn indexed_pos_to_field_id<I: Into<IndexedPos>>(&self, pos: I) -> Option<FieldId> {
        self.indexed_position.pos_to_field(pos.into())
    }

    pub fn update_ranked<S: AsRef<str>>(&mut self, data: impl IntoIterator<Item = S>) -> SResult<()> {
        self.ranked.clear();
        for name in data {
            self.set_ranked(name.as_ref())?;
        }
        Ok(())
    }

    pub fn update_displayed<S: AsRef<str>>(&mut self, data: impl IntoIterator<Item = S>) -> SResult<()> {
        self.displayed = match self.displayed.take() {
            OptionAll::Some(mut v) => {
                v.clear();
                OptionAll::Some(v)
            }
            _ => OptionAll::Some(HashSet::new())
        };
        for name in data {
            self.set_displayed(name.as_ref())?;
        }
        Ok(())
    }

    pub fn update_searchable<S: AsRef<str>>(&mut self, data: Vec<S>) -> SResult<()> {
        let mut searchable = Vec::with_capacity(data.len());
        // adds each field to the position it appears in data. If a conflict is found, the old
        // value is sent to the end of the indexed map.
        for (pos, name) in data.iter().enumerate() {
            let id = self.insert(name.as_ref())?;
            if let Some(id) = self.indexed_position.insert(id, IndexedPos(pos as u16)) {
                self.indexed_position.push(id);
            }
            searchable.push(id);
        }
        self.searchable = OptionAll::Some(searchable);
        Ok(())
    }

    pub fn set_all_fields_as_indexed(&mut self) {
        self.searchable = OptionAll::All;
    }

    pub fn set_all_fields_as_displayed(&mut self) {
        self.displayed = OptionAll::All
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use k9::*;

    #[test]
    fn test_with_primary_key() {
        let schema = Schema::with_primary_key("test");
        assert_matches_snapshot!(format!("{:?}", schema));
    }

    #[test]
    fn primary_key() {
        let schema = Schema::with_primary_key("test");
        assert_eq!(schema.primary_key(), Some("test"));
    }

    #[test]
    fn insert_last() {
        let mut schema = Schema::default();
        assert_eq!(schema.insert_position_last(1.into()), IndexedPos(0));
        assert_eq!(schema.insert_position_last(2.into()), IndexedPos(1));
    }

    #[test]
    fn test_insert_with_position_base() {
        let mut schema = Schema::default();
        let (id, position) = schema.insert_with_position("foo").unwrap();
        assert!(schema.searchable.is_all());
        assert!(schema.displayed.is_all());
        assert_eq!(id, 0.into());
        assert_eq!(position, 0.into());
        let (id, position) = schema.insert_with_position("bar").unwrap();
        assert_eq!(id, 1.into());
        assert_eq!(position, 1.into());
    }

    #[test]
    fn test_insert_with_position_primary_key() {
        let mut schema = Schema::with_primary_key("test");
        let (id, position) = schema.insert_with_position("foo").unwrap();
        assert!(schema.searchable.is_all());
        assert!(schema.displayed.is_all());
        assert_eq!(id, 1.into());
        assert_eq!(position, 0.into());
        let (id, position) = schema.insert_with_position("test").unwrap();
        assert_eq!(id, 0.into());
        assert_eq!(position, 1.into());
    }

    #[test]
    fn test_insert_with_position_non_all_searchable_attributes() {
    }

    #[test]
    fn test_insert() {
        let mut schema = Schema::default();
        let field_id = schema.insert("foo").unwrap();
        assert!(schema.fields_map.name(field_id).is_some());
        assert!(schema.searchable.is_all());
        assert!(schema.displayed.is_all());
    }

    #[test]
    fn test_set_searchable() {
    }
}
