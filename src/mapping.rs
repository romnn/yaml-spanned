use crate::spanned::Spanned;
use crate::value::Value;
use indexmap::IndexMap;
use std::cmp::Ordering;

struct Key<'a>(&'a Spanned<Value>);
impl std::fmt::Debug for Key<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            Value::Null => write!(f, "NULL"),
            Value::Bool(boolean) => write!(f, "{boolean}"),
            Value::String(value) => write!(f, "{value:?}"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Sequence(sequence) => f.debug_list().entries(sequence).finish(),
            Value::Mapping(mapping) => std::fmt::Display::fmt(mapping, f),
            Value::Tagged(tagged) => std::fmt::Display::fmt(tagged, f),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Mapping(pub IndexMap<Spanned<Value>, Spanned<Value>>);

impl std::fmt::Display for Mapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (Key(k), crate::fmt::Display(v))))
            .finish()
    }
}

pub struct StringValueRepr<'a>(&'a Mapping);

impl std::fmt::Debug for StringValueRepr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl std::fmt::Display for StringValueRepr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (Key(k), v.string_value_repr())))
            .finish()
    }
}

impl Mapping {
    #[must_use]
    pub fn string_value_repr(&self) -> StringValueRepr<'_> {
        StringValueRepr(self)
    }

    // pub fn string_value(&self) -> String {
    //     StringValueRepr(self).to_string()
    // }
}

impl AsMut<IndexMap<Spanned<Value>, Spanned<Value>>> for Mapping {
    fn as_mut(&mut self) -> &mut IndexMap<Spanned<Value>, Spanned<Value>> {
        &mut self.0
    }
}

impl AsRef<IndexMap<Spanned<Value>, Spanned<Value>>> for Mapping {
    fn as_ref(&self) -> &IndexMap<Spanned<Value>, Spanned<Value>> {
        &self.0
    }
}

impl Mapping {
    /// Creates an empty YAML map.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty YAML map with the given initial capacity.
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(IndexMap::with_capacity(capacity))
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// into the map. The map may reserve more space to avoid frequent
    /// allocations.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity of the map as much as possible. It will drop down
    /// as much as possible while maintaining the internal rules and possibly
    /// leaving some space in accordance with the resize policy.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Inserts a key-value pair into the map. If the key already existed, the
    /// old value is returned.
    #[inline]
    pub fn insert(&mut self, k: Spanned<Value>, v: Spanned<Value>) -> Option<Spanned<Value>> {
        self.0.insert(k, v)
    }

    /// Checks if the map contains the given key.
    #[inline]
    pub fn contains_key<I: Index>(&self, index: I) -> bool {
        index.is_key_into(self)
    }

    /// Returns the value corresponding to the key in the map.
    #[inline]
    pub fn get<I: Index>(&self, index: I) -> Option<&Spanned<Value>> {
        index.index_into(self)
    }

    /// Returns the mutable reference corresponding to the key in the map.
    #[inline]
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Spanned<Value>> {
        index.index_into_mut(self)
    }

    /// Gets the given key's corresponding entry in the map for insertion and/or
    /// in-place manipulation.
    #[inline]
    pub fn entry(&mut self, k: Spanned<Value>) -> Entry {
        match self.0.entry(k) {
            indexmap::map::Entry::Occupied(occupied) => Entry::Occupied(OccupiedEntry { occupied }),
            indexmap::map::Entry::Vacant(vacant) => Entry::Vacant(VacantEntry { vacant }),
        }
    }

    /// Removes and returns the value corresponding to the key from the map.
    ///
    /// This is equivalent to [`.swap_remove(index)`][Self::swap_remove],
    /// replacing this entry's position with the last element. If you need to
    /// preserve the relative order of the keys in the map, use
    /// [`.shift_remove(key)`][Self::shift_remove] instead.
    #[inline]
    pub fn remove<I: Index>(&mut self, index: I) -> Option<Spanned<Value>> {
        self.swap_remove(index)
    }

    /// Remove and return the key-value pair.
    ///
    /// This is equivalent to [`.swap_remove_entry(index)`][Self::swap_remove_entry],
    /// replacing this entry's position with the last element. If you need to
    /// preserve the relative order of the keys in the map, use
    /// [`.shift_remove_entry(key)`][Self::shift_remove_entry] instead.
    #[inline]
    pub fn remove_entry<I: Index>(&mut self, index: I) -> Option<(Spanned<Value>, Spanned<Value>)> {
        self.swap_remove_entry(index)
    }

    /// Removes and returns the value corresponding to the key from the map.
    ///
    /// Like [`Vec::swap_remove`], the entry is removed by swapping it with the
    /// last element of the map and popping it off. This perturbs the position
    /// of what used to be the last element!
    #[inline]
    pub fn swap_remove<I: Index>(&mut self, index: I) -> Option<Spanned<Value>> {
        index.swap_remove_from(self)
    }

    /// Remove and return the key-value pair.
    ///
    /// Like [`Vec::swap_remove`], the entry is removed by swapping it with the
    /// last element of the map and popping it off. This perturbs the position
    /// of what used to be the last element!
    #[inline]
    pub fn swap_remove_entry<I: Index>(
        &mut self,
        index: I,
    ) -> Option<(Spanned<Value>, Spanned<Value>)> {
        index.swap_remove_entry_from(self)
    }

    /// Removes and returns the value corresponding to the key from the map.
    ///
    /// Like [`Vec::remove`], the entry is removed by shifting all of the
    /// elements that follow it, preserving their relative order. This perturbs
    /// the index of all of those elements!
    #[inline]
    pub fn shift_remove<I: Index>(&mut self, index: I) -> Option<Spanned<Value>> {
        index.shift_remove_from(self)
    }

    /// Remove and return the key-value pair.
    ///
    /// Like [`Vec::remove`], the entry is removed by shifting all of the
    /// elements that follow it, preserving their relative order. This perturbs
    /// the index of all of those elements!
    #[inline]
    pub fn shift_remove_entry<I: Index>(
        &mut self,
        index: I,
    ) -> Option<(Spanned<Value>, Spanned<Value>)> {
        index.shift_remove_entry_from(self)
    }

    /// Scan through each key-value pair in the map and keep those where the
    /// closure `keep` returns true.
    #[inline]
    pub fn retain<F>(&mut self, keep: F)
    where
        F: FnMut(&Spanned<Value>, &mut Spanned<Value>) -> bool,
    {
        self.0.retain(keep);
    }

    /// Returns the maximum number of key-value pairs the map can hold without
    /// reallocating.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns the number of key-value pairs in the map.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the map is currently empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clears the map of all key-value pairs.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns a double-ended iterator visiting all key-value pairs in order of
    /// insertion. Iterator element type is `(&'a Value, &'a Value)`.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter {
        Iter {
            iter: self.0.iter(),
        }
    }

    /// Returns a double-ended iterator visiting all key-value pairs in order of
    /// insertion. Iterator element type is `(&'a Value, &'a mut Value)`.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            iter: self.0.iter_mut(),
        }
    }

    /// Return an iterator over the keys of the map.
    #[must_use]
    pub fn keys(&self) -> Keys {
        Keys {
            iter: self.0.keys(),
        }
    }

    /// Return an owning iterator over the keys of the map.
    #[must_use]
    pub fn into_keys(self) -> IntoKeys {
        IntoKeys {
            iter: self.0.into_keys(),
        }
    }

    /// Return an iterator over the values of the map.
    #[must_use]
    pub fn values(&self) -> Values {
        Values {
            iter: self.0.values(),
        }
    }

    /// Return an iterator over mutable references to the values of the map.
    pub fn values_mut(&mut self) -> ValuesMut {
        ValuesMut {
            iter: self.0.values_mut(),
        }
    }

    /// Return an owning iterator over the values of the map.
    #[must_use]
    pub fn into_values(self) -> IntoValues {
        IntoValues {
            iter: self.0.into_values(),
        }
    }
}

/// A type that can be used to index into a `yaml_spanned::Mapping`. See the
/// methods `get`, `get_mut`, `contains_key`, and `remove` of `Value`.
///
/// This trait is sealed and cannot be implemented for types outside of `yaml_spanned`.
pub trait Index: crate::private::Sealed {
    #[doc(hidden)]
    fn is_key_into(&self, v: &Mapping) -> bool;

    #[doc(hidden)]
    fn index_into<'a>(&self, v: &'a Mapping) -> Option<&'a Spanned<Value>>;

    #[doc(hidden)]
    fn index_into_mut<'a>(&self, v: &'a mut Mapping) -> Option<&'a mut Spanned<Value>>;

    #[doc(hidden)]
    fn swap_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>>;

    #[doc(hidden)]
    fn swap_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)>;

    #[doc(hidden)]
    fn shift_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>>;

    #[doc(hidden)]
    fn shift_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)>;
}

struct HashLikeValue<'a>(&'a str);

impl indexmap::Equivalent<Spanned<Value>> for HashLikeValue<'_> {
    fn equivalent(&self, key: &Spanned<Value>) -> bool {
        match key.as_ref() {
            Value::String(string) => self.0 == string,
            _ => false,
        }
    }
}

impl indexmap::Equivalent<Value> for HashLikeValue<'_> {
    fn equivalent(&self, key: &Value) -> bool {
        match key {
            Value::String(string) => self.0 == string,
            _ => false,
        }
    }
}

// NOTE: This impl must be consistent with Value's Hash impl.
impl std::hash::Hash for HashLikeValue<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        const STRING: Value = Value::String(String::new());
        std::mem::discriminant(&STRING).hash(state);
        self.0.hash(state);
    }
}

impl Index for Spanned<Value> {
    fn is_key_into(&self, v: &Mapping) -> bool {
        v.0.contains_key(self)
    }
    fn index_into<'a>(&self, v: &'a Mapping) -> Option<&'a Spanned<Value>> {
        v.0.get(self)
    }
    fn index_into_mut<'a>(&self, v: &'a mut Mapping) -> Option<&'a mut Spanned<Value>> {
        v.0.get_mut(self)
    }
    fn swap_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        v.0.swap_remove(self)
    }
    fn swap_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        v.0.swap_remove_entry(self)
    }
    fn shift_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        v.0.shift_remove(self)
    }
    fn shift_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        v.0.shift_remove_entry(self)
    }
}

impl Index for Value {
    fn is_key_into(&self, v: &Mapping) -> bool {
        v.0.contains_key(self)
    }
    fn index_into<'a>(&self, v: &'a Mapping) -> Option<&'a Spanned<Value>> {
        v.0.get(self)
    }
    fn index_into_mut<'a>(&self, v: &'a mut Mapping) -> Option<&'a mut Spanned<Value>> {
        v.0.get_mut(self)
    }
    fn swap_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        v.0.swap_remove(self)
    }
    fn swap_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        v.0.swap_remove_entry(self)
    }
    fn shift_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        v.0.shift_remove(self)
    }
    fn shift_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        v.0.shift_remove_entry(self)
    }
}

impl Index for str {
    fn is_key_into(&self, v: &Mapping) -> bool {
        v.0.contains_key(&HashLikeValue(self))
    }
    fn index_into<'a>(&self, v: &'a Mapping) -> Option<&'a Spanned<Value>> {
        v.0.get(&HashLikeValue(self))
    }
    fn index_into_mut<'a>(&self, v: &'a mut Mapping) -> Option<&'a mut Spanned<Value>> {
        v.0.get_mut(&HashLikeValue(self))
    }
    fn swap_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        v.0.swap_remove(&HashLikeValue(self))
    }
    fn swap_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        v.0.swap_remove_entry(&HashLikeValue(self))
    }
    fn shift_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        v.0.shift_remove(&HashLikeValue(self))
    }
    fn shift_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        v.0.shift_remove_entry(&HashLikeValue(self))
    }
}

impl Index for String {
    fn is_key_into(&self, v: &Mapping) -> bool {
        self.as_str().is_key_into(v)
    }
    fn index_into<'a>(&self, v: &'a Mapping) -> Option<&'a Spanned<Value>> {
        self.as_str().index_into(v)
    }
    fn index_into_mut<'a>(&self, v: &'a mut Mapping) -> Option<&'a mut Spanned<Value>> {
        self.as_str().index_into_mut(v)
    }
    fn swap_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        self.as_str().swap_remove_from(v)
    }
    fn swap_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        self.as_str().swap_remove_entry_from(v)
    }
    fn shift_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        self.as_str().shift_remove_from(v)
    }
    fn shift_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        self.as_str().shift_remove_entry_from(v)
    }
}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn is_key_into(&self, v: &Mapping) -> bool {
        (**self).is_key_into(v)
    }
    fn index_into<'a>(&self, v: &'a Mapping) -> Option<&'a Spanned<Value>> {
        (**self).index_into(v)
    }
    fn index_into_mut<'a>(&self, v: &'a mut Mapping) -> Option<&'a mut Spanned<Value>> {
        (**self).index_into_mut(v)
    }
    fn swap_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        (**self).swap_remove_from(v)
    }
    fn swap_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        (**self).swap_remove_entry_from(v)
    }
    fn shift_remove_from(&self, v: &mut Mapping) -> Option<Spanned<Value>> {
        (**self).shift_remove_from(v)
    }
    fn shift_remove_entry_from(&self, v: &mut Mapping) -> Option<(Spanned<Value>, Spanned<Value>)> {
        (**self).shift_remove_entry_from(v)
    }
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl std::hash::Hash for Mapping {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hasher;
        // Hash the kv pairs in a way that is not sensitive to their order.
        let mut xor = 0;
        for (k, v) in &self.0 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            k.hash(&mut hasher);
            v.hash(&mut hasher);
            xor ^= hasher.finish();
        }
        xor.hash(state);
    }
}

impl PartialOrd for Mapping {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut self_entries = Vec::from_iter(self);
        let mut other_entries = Vec::from_iter(other);

        // Sort in an arbitrary order that is consistent with Value's PartialOrd
        // impl.
        fn total_cmp(a: &Value, b: &Value) -> Ordering {
            match (a, b) {
                (Value::Null, Value::Null) => Ordering::Equal,
                (Value::Null, _) => Ordering::Less,
                (_, Value::Null) => Ordering::Greater,

                (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
                (Value::Bool(_), _) => Ordering::Less,
                (_, Value::Bool(_)) => Ordering::Greater,

                (Value::Number(a), Value::Number(b)) => a.total_cmp(b),
                (Value::Number(_), _) => Ordering::Less,
                (_, Value::Number(_)) => Ordering::Greater,

                (Value::String(a), Value::String(b)) => a.cmp(b),
                (Value::String(_), _) => Ordering::Less,
                (_, Value::String(_)) => Ordering::Greater,

                (Value::Sequence(a), Value::Sequence(b)) => {
                    iter_cmp_by(a, b, |a, b| total_cmp(a.as_ref(), b.as_ref()))
                }
                (Value::Sequence(_), _) => Ordering::Less,
                (_, Value::Sequence(_)) => Ordering::Greater,

                (Value::Mapping(a), Value::Mapping(b)) => {
                    iter_cmp_by(a, b, |(ak, av), (bk, bv)| {
                        total_cmp(ak.as_ref(), bk.as_ref())
                            .then_with(|| total_cmp(av.as_ref(), bv.as_ref()))
                    })
                }
                (Value::Mapping(_), _) => Ordering::Less,
                (_, Value::Mapping(_)) => Ordering::Greater,

                (Value::Tagged(a), Value::Tagged(b)) => a
                    .tag
                    .cmp(&b.tag)
                    .then_with(|| total_cmp(a.value.as_ref(), b.value.as_ref())),
            }
        }

        fn iter_cmp_by<I, F>(this: I, other: I, mut cmp: F) -> Ordering
        where
            I: IntoIterator,
            F: FnMut(I::Item, I::Item) -> Ordering,
        {
            let mut this = this.into_iter();
            let mut other = other.into_iter();

            loop {
                let x = match this.next() {
                    None => {
                        if other.next().is_none() {
                            return Ordering::Equal;
                        }
                        return Ordering::Less;
                    }
                    Some(val) => val,
                };

                let y = match other.next() {
                    None => return Ordering::Greater,
                    Some(val) => val,
                };

                match cmp(x, y) {
                    Ordering::Equal => {}
                    non_eq => return non_eq,
                }
            }
        }

        // While sorting by map key, we get to assume that no two keys are
        // equal, otherwise they wouldn't both be in the map. This is not a safe
        // assumption outside of this situation.
        let total_cmp = |&(a, _): &(&Spanned<Value>, &Spanned<Value>),
                         &(b, _): &(&Spanned<Value>, &Spanned<Value>)| {
            total_cmp(a.as_ref(), b.as_ref())
        };
        self_entries.sort_by(total_cmp);
        other_entries.sort_by(total_cmp);
        self_entries.partial_cmp(&other_entries)
    }
}

impl<I> std::ops::Index<I> for Mapping
where
    I: Index,
{
    type Output = Spanned<Value>;

    #[inline]
    #[track_caller]
    fn index(&self, index: I) -> &Self::Output {
        index.index_into(self).unwrap()
    }
}

impl<I> std::ops::IndexMut<I> for Mapping
where
    I: Index,
{
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut Spanned<Value> {
        index.index_into_mut(self).unwrap()
    }
}

impl Extend<(Spanned<Value>, Spanned<Value>)> for Mapping {
    #[inline]
    fn extend<I: IntoIterator<Item = (Spanned<Value>, Spanned<Value>)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl FromIterator<(Spanned<Value>, Spanned<Value>)> for Mapping {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (Spanned<Value>, Spanned<Value>)>>(iter: I) -> Self {
        Mapping(IndexMap::from_iter(iter))
    }
}

macro_rules! delegate_iterator {
    (($name:ident $($generics:tt)*) => $item:ty) => {
        impl $($generics)* Iterator for $name $($generics)* {
            type Item = $item;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl $($generics)* ExactSizeIterator for $name $($generics)* {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
    }
}

/// Iterator over `&yaml_spanned::Mapping`.
pub struct Iter<'a> {
    iter: indexmap::map::Iter<'a, Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((Iter<'a>) => (&'a Spanned<Value>, &'a Spanned<Value>));

impl<'a> IntoIterator for &'a Mapping {
    type Item = (&'a Spanned<Value>, &'a Spanned<Value>);
    type IntoIter = Iter<'a>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.0.iter(),
        }
    }
}

/// Iterator over `&mut yaml_spanned::Mapping`.
pub struct IterMut<'a> {
    iter: indexmap::map::IterMut<'a, Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((IterMut<'a>) => (&'a Spanned<Value>, &'a mut Spanned<Value>));

impl<'a> IntoIterator for &'a mut Mapping {
    type Item = (&'a Spanned<Value>, &'a mut Spanned<Value>);
    type IntoIter = IterMut<'a>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            iter: self.0.iter_mut(),
        }
    }
}

/// Iterator over `yaml_spanned::Mapping` by value.
pub struct IntoIter {
    iter: indexmap::map::IntoIter<Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((IntoIter) => (Spanned<Value>, Spanned<Value>));

impl IntoIterator for Mapping {
    type Item = (Spanned<Value>, Spanned<Value>);
    type IntoIter = IntoIter;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.0.into_iter(),
        }
    }
}

/// Iterator of the keys of a `&yaml_spanned::Mapping`.
pub struct Keys<'a> {
    iter: indexmap::map::Keys<'a, Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((Keys<'a>) => &'a Spanned<Value>);

/// Iterator of the keys of a `yaml_spanned::Mapping`.
pub struct IntoKeys {
    iter: indexmap::map::IntoKeys<Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((IntoKeys) => Spanned<Value>);

/// Iterator of the values of a `&yaml_spanned::Mapping`.
pub struct Values<'a> {
    iter: indexmap::map::Values<'a, Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((Values<'a>) => &'a Spanned<Value>);

/// Iterator of the values of a `&mut yaml_spanned::Mapping`.
pub struct ValuesMut<'a> {
    iter: indexmap::map::ValuesMut<'a, Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((ValuesMut<'a>) => &'a mut Spanned<Value>);

/// Iterator of the values of a `yaml_spanned::Mapping`.
pub struct IntoValues {
    iter: indexmap::map::IntoValues<Spanned<Value>, Spanned<Value>>,
}

delegate_iterator!((IntoValues) => Spanned<Value>);

/// Entry for an existing key-value pair or a vacant location to insert one.
pub enum Entry<'a> {
    /// Existing slot with equivalent key.
    Occupied(OccupiedEntry<'a>),
    /// Vacant slot (no equivalent key in the map).
    Vacant(VacantEntry<'a>),
}

/// A view into an occupied entry in a [`Mapping`]. It is part of the [`Entry`]
/// enum.
pub struct OccupiedEntry<'a> {
    occupied: indexmap::map::OccupiedEntry<'a, Spanned<Value>, Spanned<Value>>,
}

/// A view into a vacant entry in a [`Mapping`]. It is part of the [`Entry`]
/// enum.
pub struct VacantEntry<'a> {
    vacant: indexmap::map::VacantEntry<'a, Spanned<Value>, Spanned<Value>>,
}

impl<'a> Entry<'a> {
    /// Returns a reference to this entry's key.
    #[must_use]
    pub fn key(&self) -> &Spanned<Value> {
        match self {
            Entry::Vacant(e) => e.key(),
            Entry::Occupied(e) => e.key(),
        }
    }

    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    #[must_use]
    pub fn or_insert(self, default: Spanned<Value>) -> &'a mut Spanned<Value> {
        match self {
            Entry::Vacant(entry) => entry.insert(default),
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns a mutable reference to the value in the
    /// entry.
    pub fn or_insert_with<F>(self, default: F) -> &'a mut Spanned<Value>
    where
        F: FnOnce() -> Spanned<Value>,
    {
        match self {
            Entry::Vacant(entry) => entry.insert(default()),
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Spanned<Value>),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }
}

impl<'a> OccupiedEntry<'a> {
    /// Gets a reference to the key in the entry.
    #[inline]
    #[must_use]
    pub fn key(&self) -> &Spanned<Value> {
        self.occupied.key()
    }

    /// Gets a reference to the value in the entry.
    #[inline]
    #[must_use]
    pub fn get(&self) -> &Spanned<Value> {
        self.occupied.get()
    }

    /// Gets a mutable reference to the value in the entry.
    #[inline]
    pub fn get_mut(&mut self) -> &mut Spanned<Value> {
        self.occupied.get_mut()
    }

    /// Converts the entry into a mutable reference to its value.
    #[inline]
    #[must_use]
    pub fn into_mut(self) -> &'a mut Spanned<Value> {
        self.occupied.into_mut()
    }

    /// Sets the value of the entry with the `OccupiedEntry`'s key, and returns
    /// the entry's old value.
    #[inline]
    pub fn insert(&mut self, value: Spanned<Value>) -> Spanned<Value> {
        self.occupied.insert(value)
    }

    /// Takes the value of the entry out of the map, and returns it.
    #[inline]
    #[must_use]
    pub fn remove(self) -> Spanned<Value> {
        self.occupied.swap_remove()
    }

    /// Remove and return the key, value pair stored in the map for this entry.
    #[inline]
    #[must_use]
    pub fn remove_entry(self) -> (Spanned<Value>, Spanned<Value>) {
        self.occupied.swap_remove_entry()
    }
}

impl<'a> VacantEntry<'a> {
    /// Gets a reference to the key that would be used when inserting a value
    /// through the `VacantEntry`.
    #[inline]
    #[must_use]
    pub fn key(&self) -> &Spanned<Value> {
        self.vacant.key()
    }

    /// Takes ownership of the key, leaving the entry vacant.
    #[inline]
    #[must_use]
    pub fn into_key(self) -> Spanned<Value> {
        self.vacant.into_key()
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns a
    /// mutable reference to it.
    #[inline]
    #[must_use]
    pub fn insert(self, value: Spanned<Value>) -> &'a mut Spanned<Value> {
        self.vacant.insert(value)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Mapping {
    #[inline]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map_serializer = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map_serializer.serialize_entry(k, v)?;
        }
        map_serializer.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Mapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Mapping;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a YAML mapping")
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Mapping::new())
            }

            #[inline]
            fn visit_map<A>(self, mut data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut mapping = Mapping::new();

                // while let Some(key) = data.next_key::<Value>()? {
                while let Some(key) = data.next_key()? {
                    // match mapping.entry(Spanned::dummy(key)) {
                    match mapping.entry(key) {
                        Entry::Occupied(entry) => {
                            return Err(serde::de::Error::custom(DuplicateKeyError { entry }));
                        }
                        Entry::Vacant(entry) => {
                            // let value: Value = data.next_value()?;
                            let value = data.next_value()?;
                            // entry.insert(Spanned::dummy(value));
                            _ = entry.insert(value);
                        }
                    }
                }

                Ok(mapping)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

struct DuplicateKeyError<'a> {
    entry: OccupiedEntry<'a>,
}

impl std::fmt::Display for DuplicateKeyError<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("duplicate entry ")?;
        match self.entry.key().as_ref() {
            Value::Null => formatter.write_str("with null key"),
            Value::Bool(boolean) => write!(formatter, "with key `{boolean}`"),
            Value::Number(number) => write!(formatter, "with key {number}"),
            Value::String(string) => write!(formatter, "with key {string:?}"),
            Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => {
                formatter.write_str("in YAML map")
            }
        }
    }
}

// #[allow(clippy::derived_hash_with_manual_eq)]
// impl std::hash::Hash for Mapping {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         use std::collections::hash_map::DefaultHasher;
//         use std::hash::Hasher;
//         // Hash the kv pairs in a way that is not sensitive to their order.
//         let mut xor = 0;
//         for (k, v) in self.0.iter() {
//             let mut hasher = DefaultHasher::new();
//             k.hash(&mut hasher);
//             v.hash(&mut hasher);
//             xor ^= hasher.finish();
//         }
//         xor.hash(state);
//     }
// }

#[cfg(test)]
mod tests {
    use crate::{Mapping, Value};
    use color_eyre::eyre;
    use indoc::indoc;

    #[test]
    fn test_mapping() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            substructure:
              a: 'foo'
              b: 'bar'
        "};

        let value = crate::from_str(yaml)?;
        similar_asserts::assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([(
                "substructure".into(),
                Mapping::from_iter([("a".into(), "foo".into()), ("b".into(), "bar".into()),])
                    .into()
            ),]))
        );

        #[cfg(feature = "serde")]
        {
            #[derive(Debug, serde::Deserialize, PartialEq)]
            struct Data {
                pub substructure: serde_yaml::Mapping,
            }

            let mut expected = Data {
                substructure: serde_yaml::Mapping::new(),
            };
            expected.substructure.insert(
                serde_yaml::Value::String("a".to_owned()),
                serde_yaml::Value::String("foo".to_owned()),
            );
            expected.substructure.insert(
                serde_yaml::Value::String("b".to_owned()),
                serde_yaml::Value::String("bar".to_owned()),
            );

            similar_asserts::assert_eq!(crate::from_value::<Data>(&value)?, expected);
        }

        Ok(())
    }
}
