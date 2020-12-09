use crate::{FieldsMap, FieldId, SResult, Error, IndexedPos};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::borrow::Cow;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    indexed_position: HashMap<FieldId, IndexedPos>,
}

impl Schema {
    pub fn new() -> Schema {
        Schema::default()
    }

    pub fn with_primary_key(name: &str) -> Schema {
        let mut fields_map = FieldsMap::default();
        let field_id = fields_map.insert(name).unwrap();

        let mut displayed = HashSet::new();
        let mut indexed_position = HashMap::new();

        displayed.insert(field_id);
        indexed_position.insert(field_id, 0.into());

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
        self.set_indexed(name)?;
        self.set_displayed(name)?;

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
        let position = self.indexed_position.len() as u16;
        let previous = self.insert_position(field_id, position.into());
        debug_assert!(previous.is_none());
        Ok((field_id, position.into()))
    }

    /// insert a field `field_id` at `position` in the `indexed_position` map, and return the
    /// previous value if there was one.
    fn insert_position(&mut self, field_id: FieldId, position: IndexedPos) -> Option<IndexedPos> {
        self.indexed_position.insert(field_id, position)
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
                    .iter()
                    .map(|(&f, _)| f)
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

    pub(crate) fn set_displayed(&mut self, name: &str) -> SResult<FieldId> {
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

    pub(crate) fn set_indexed(&mut self, name: &str) -> SResult<(FieldId, IndexedPos)> {
        let id = self.fields_map.insert(name)?;
        if let Some(indexed_pos) = self.indexed_position.get(&id) {
            return Ok((id, *indexed_pos))
        };
        let pos = self.indexed_position.len() as u16;
        let value = self.insert_position(id, pos.into());
        debug_assert!(value.is_none());
        self.searchable = self.searchable.take().map(|mut v| {
            v.push(id);
            v
        });
        Ok((id, pos.into()))
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

    pub fn get_position(&self, id: FieldId) -> Option<&IndexedPos> {
        self.indexed_position.get(&id)
    }

    pub fn is_searchable_all(&self) -> bool {
        self.searchable.is_all()
    }

    pub fn indexed_pos_to_field_id<I: Into<IndexedPos>>(&self, pos: I) -> Option<FieldId> {
        let indexed_pos = pos.into().0;
        self
            .indexed_position
            .iter()
            .find(|(_, &v)| v.0 == indexed_pos)
            .map(|(&k, _)| k)
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

    pub fn update_indexed<S: AsRef<str>>(&mut self, data: Vec<S>) -> SResult<()> {
        self.searchable = match self.searchable.take() {
            OptionAll::Some(mut v) => {
                v.clear();
                OptionAll::Some(v)
            },
            _ => OptionAll::Some(Vec::new()),
        };
        self.indexed_position.clear();
        for name in data {
            self.set_indexed(name.as_ref())?;
        }
        Ok(())
    }

    pub fn set_all_fields_as_indexed(&mut self) {
        self.searchable = OptionAll::All;
    }

    pub fn set_all_fields_as_displayed(&mut self) {
        self.displayed = OptionAll::All
    }
}
