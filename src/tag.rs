use crate::spanned::Spanned;
use crate::value::Value;
use std::cmp::Ordering;

/// A representation of YAML's `!Tag` syntax, used for enums.
///
/// Refer to the example code on [`TaggedValue`] for an example of deserializing
/// tagged values.
#[derive(Clone, Default)]
pub struct Tag {
    pub(crate) string: String,
}

impl From<String> for Tag {
    fn from(value: String) -> Self {
        Tag::new(value)
    }
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        Tag::new(value.to_string())
    }
}

/// A `Tag` + `Value` representing a tagged YAML scalar, sequence, or mapping.
///
/// ```
/// use yaml_spanned::TaggedValue;
/// use std::collections::BTreeMap;
///
/// let yaml = "
///     scalar: !Thing x
///     sequence_flow: !Thing [first]
///     sequence_block: !Thing
///       - first
///     mapping_flow: !Thing {k: v}
///     mapping_block: !Thing
///       k: v
/// ";
///
/// let value: yaml_spanned::SpannedValue = yaml_spanned::from_str(yaml).unwrap();
/// let data: BTreeMap<String, TaggedValue> = yaml_spanned::from_value(&value).unwrap();
/// assert!(data["scalar"].tag == "Thing");
/// assert!(data["sequence_flow"].tag == "Thing");
/// assert!(data["sequence_block"].tag == "Thing");
/// assert!(data["mapping_flow"].tag == "Thing");
/// assert!(data["mapping_block"].tag == "Thing");
///
/// // The leading '!' in tags are not significant. The following is also true.
/// assert!(data["scalar"].tag == "!Thing");
/// ```
#[derive(Clone, PartialEq, PartialOrd, Default, Hash, Debug)]
pub struct TaggedValue {
    pub tag: Tag,
    pub value: Spanned<Value>,
}

impl TaggedValue {
    pub fn new(tag: impl Into<Tag>, value: impl Into<Spanned<Value>>) -> Self {
        Self {
            tag: tag.into(),
            value: value.into(),
        }
    }
}

impl std::fmt::Display for TaggedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.value, f)
    }
}

impl Tag {
    /// Create tag.
    ///
    /// The leading '!' is not significant. It may be provided, but does not
    /// have to be. The following are equivalent:
    ///
    /// ```
    /// use yaml_spanned::Tag;
    ///
    /// assert_eq!(Tag::new("!Thing"), Tag::new("Thing"));
    ///
    /// let tag = Tag::new("Thing");
    /// assert!(tag == "Thing");
    /// assert!(tag == "!Thing");
    /// assert!(tag.to_string() == "!Thing");
    ///
    /// let tag = Tag::new("!Thing");
    /// assert!(tag == "Thing");
    /// assert!(tag == "!Thing");
    /// assert!(tag.to_string() == "!Thing");
    /// ```
    ///
    /// Such a tag would serialize to `!Thing` in YAML regardless of whether a
    /// '!' was included in the call to `Tag::new`.
    ///
    /// # Panics
    ///
    /// Panics if `string.is_empty()`. There is no syntax in YAML for an empty
    /// tag.
    pub fn new(string: impl Into<String>) -> Self {
        let tag: String = string.into();
        assert!(!tag.is_empty(), "empty YAML tag is not allowed");
        Tag { string: tag }
    }
}

impl Value {
    pub(crate) fn untag(self) -> Self {
        let mut cur = self;
        while let Value::Tagged(tagged) = cur {
            cur = tagged.value.inner;
        }
        cur
    }

    pub(crate) fn untag_ref(&self) -> &Self {
        let mut cur = self;
        while let Value::Tagged(tagged) = cur {
            cur = &tagged.value.inner;
        }
        cur
    }

    pub(crate) fn untag_mut(&mut self) -> &mut Self {
        let mut cur = self;
        while let Value::Tagged(tagged) = cur {
            cur = &mut tagged.value.inner;
        }
        cur
    }
}

impl Spanned<Value> {
    pub(crate) fn untag(self) -> Self {
        let mut cur = self;
        while let Value::Tagged(tagged) = cur.inner {
            cur = tagged.value;
        }
        cur
    }

    pub(crate) fn untag_ref(&self) -> &Self {
        let mut cur = self;
        while let Value::Tagged(ref tagged) = cur.inner {
            cur = &tagged.value;
        }
        cur
    }

    pub(crate) fn untag_mut(&mut self) -> &mut Self {
        let mut cur = self;
        while let Value::Tagged(ref mut tagged) = cur.inner {
            cur = &mut tagged.value;
        }
        cur
    }
}

pub(crate) fn nobang(maybe_banged: &str) -> &str {
    match maybe_banged.strip_prefix('!') {
        Some("") | None => maybe_banged,
        Some(unbanged) => unbanged,
    }
}

impl Eq for Tag {}

impl PartialEq for Tag {
    fn eq(&self, other: &Tag) -> bool {
        PartialEq::eq(nobang(&self.string), nobang(&other.string))
    }
}

impl<T> PartialEq<T> for Tag
where
    T: ?Sized + AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        PartialEq::eq(nobang(&self.string), nobang(other.as_ref()))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(nobang(&self.string), nobang(&other.string))
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Tag {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        nobang(&self.string).hash(hasher);
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "!{}", nobang(&self.string))
    }
}

impl std::fmt::Debug for Tag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

#[cfg(feature = "serde")]
pub mod serde {
    use super::{Tag, TaggedValue, Value};
    use serde::de::Error as _;

    impl serde::Serialize for TaggedValue {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeMap;
            struct SerializeTag<'a>(&'a Tag);

            impl<'a> serde::Serialize for SerializeTag<'a> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    serializer.collect_str(self.0)
                }
            }

            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry(&SerializeTag(&self.tag), &self.value)?;
            map.end()
        }
    }

    impl<'de> serde::Deserialize<'de> for TaggedValue {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct TaggedValueVisitor;

            impl<'de> serde::de::Visitor<'de> for TaggedValueVisitor {
                type Value = TaggedValue;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a YAML value with a !Tag")
                }

                fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::EnumAccess<'de>,
                {
                    use serde::de::VariantAccess;
                    let (tag, contents) = data.variant_seed(TagStringVisitor)?;
                    // let value: Value = contents.newtype_variant()?;
                    let value = contents.newtype_variant()?;
                    Ok(TaggedValue {
                        tag,
                        // value: Spanned::dummy(value),
                        value,
                    })
                }
            }

            deserializer.deserialize_any(TaggedValueVisitor)
        }
    }

    impl<'de> serde::Deserializer<'de> for TaggedValue {
        type Error = crate::error::SerdeError;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.visit_enum(self)
        }

        fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            drop(self);
            visitor.visit_unit()
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
            byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
            map struct enum identifier
        }
    }

    impl<'de> serde::de::EnumAccess<'de> for TaggedValue {
        type Error = crate::error::SerdeError;
        type Variant = Value;

        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
        where
            V: serde::de::DeserializeSeed<'de>,
        {
            let tag = serde::de::value::StrDeserializer::<Self::Error>::new(super::nobang(
                &self.tag.string,
            ));
            let value = seed.deserialize(tag)?;
            Ok((value, self.value.inner))
        }
    }

    impl<'de> serde::de::VariantAccess<'de> for Value {
        type Error = crate::error::SerdeError;

        fn unit_variant(self) -> Result<(), Self::Error> {
            serde::Deserialize::deserialize(self)
        }

        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: serde::de::DeserializeSeed<'de>,
        {
            seed.deserialize(self)
        }

        fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            if let Value::Sequence(v) = self {
                serde::Deserializer::deserialize_any(
                    crate::de::SeqDeserializer::new(v),
                    // crate::de::SeqDeserializer {
                    //     iter: v.into_iter().map(Spanned::into_inner),
                    // },
                    visitor,
                )
            } else {
                Err(Self::Error::invalid_type(
                    self.unexpected(),
                    &"tuple variant",
                ))
            }
        }

        fn struct_variant<V>(
            self,
            _fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            if let Value::Mapping(v) = self {
                serde::Deserializer::deserialize_any(crate::de::MapDeserializer::new(v), visitor)
            } else {
                Err(Self::Error::invalid_type(
                    self.unexpected(),
                    &"struct variant",
                ))
            }
        }
    }

    impl<'de> serde::Deserializer<'de> for &'de TaggedValue {
        type Error = crate::error::SerdeError;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.visit_enum(self)
        }

        fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.visit_unit()
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
            byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
            map struct enum identifier
        }
    }

    impl<'de> serde::de::EnumAccess<'de> for &'de TaggedValue {
        type Error = crate::error::SerdeError;
        type Variant = &'de Value;

        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
        where
            V: serde::de::DeserializeSeed<'de>,
        {
            let tag = serde::de::value::BorrowedStrDeserializer::<Self::Error>::new(super::nobang(
                &self.tag.string,
            ));
            let value = seed.deserialize(tag)?;
            Ok((value, &self.value))
        }
    }

    impl<'de> serde::de::VariantAccess<'de> for &'de Value {
        type Error = crate::error::SerdeError;

        fn unit_variant(self) -> Result<(), Self::Error> {
            serde::Deserialize::deserialize(self)
        }

        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: serde::de::DeserializeSeed<'de>,
        {
            seed.deserialize(self)
        }

        fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            if let Value::Sequence(v) = self {
                serde::Deserializer::deserialize_any(crate::de::SeqRefDeserializer::new(v), visitor)
            } else {
                Err(Self::Error::invalid_type(
                    self.unexpected(),
                    &"tuple variant",
                ))
            }
        }

        fn struct_variant<V>(
            self,
            _fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            if let Value::Mapping(v) = self {
                serde::Deserializer::deserialize_any(crate::de::MapRefDeserializer::new(v), visitor)
            } else {
                Err(Self::Error::invalid_type(
                    self.unexpected(),
                    &"struct variant",
                ))
            }
        }
    }

    pub(crate) struct TagStringVisitor;

    impl<'de> serde::de::Visitor<'de> for TagStringVisitor {
        type Value = Tag;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a YAML tag string")
        }

        fn visit_str<E>(self, string: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_string(string.to_owned())
        }

        fn visit_string<E>(self, string: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if string.is_empty() {
                return Err(E::custom("empty YAML tag is not allowed"));
            }
            Ok(Tag::new(string))
        }
    }

    impl<'de> serde::de::DeserializeSeed<'de> for TagStringVisitor {
        type Value = Tag;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_string(self)
        }
    }

    pub(crate) enum MaybeTag<T> {
        Tag(String),
        NotTag(T),
    }

    pub(crate) fn check_for_tag<T>(value: &T) -> MaybeTag<String>
    where
        T: ?Sized + std::fmt::Display,
    {
        enum CheckForTag {
            Empty,
            Bang,
            Tag(String),
            NotTag(String),
        }

        impl std::fmt::Write for CheckForTag {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                if s.is_empty() {
                    return Ok(());
                }
                match self {
                    CheckForTag::Empty => {
                        if s == "!" {
                            *self = CheckForTag::Bang;
                        } else {
                            *self = CheckForTag::NotTag(s.to_owned());
                        }
                    }
                    CheckForTag::Bang => {
                        *self = CheckForTag::Tag(s.to_owned());
                    }
                    CheckForTag::Tag(string) => {
                        let mut string = std::mem::take(string);
                        string.push_str(s);
                        *self = CheckForTag::NotTag(string);
                    }
                    CheckForTag::NotTag(string) => {
                        string.push_str(s);
                    }
                }
                Ok(())
            }
        }

        let mut check_for_tag = CheckForTag::Empty;
        std::fmt::write(&mut check_for_tag, format_args!("{}", value)).unwrap();
        match check_for_tag {
            CheckForTag::Empty => MaybeTag::NotTag(String::new()),
            CheckForTag::Bang => MaybeTag::NotTag("!".to_owned()),
            CheckForTag::Tag(string) => MaybeTag::Tag(string),
            CheckForTag::NotTag(string) => MaybeTag::NotTag(string),
        }
    }
}
