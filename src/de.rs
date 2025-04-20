use crate::error::SerdeError;
use crate::spanned::Spanned;
use crate::tag::TaggedValue;
use crate::tag::serde::TagStringVisitor;
use crate::{Mapping, Sequence, Value, number};
use serde::de::value::{BorrowedStrDeserializer, StrDeserializer};
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error as _, Expected, MapAccess,
    SeqAccess, Unexpected, VariantAccess, Visitor,
};
use serde::forward_to_deserialize_any;

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any YAML value")
            }

            fn visit_bool<E>(self, b: bool) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Bool(b))
            }

            fn visit_i64<E>(self, i: i64) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Number(i.into()))
            }

            fn visit_u64<E>(self, u: u64) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Number(u.into()))
            }

            fn visit_f64<E>(self, f: f64) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Number(f.into()))
            }

            fn visit_str<E>(self, s: &str) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(s.to_owned()))
            }

            fn visit_string<E>(self, s: String) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(s))
            }

            fn visit_unit<E>(self) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Null)
            }

            fn visit_none<E>(self) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let de = serde::de::value::SeqAccessDeserializer::new(data);
                let sequence = Sequence::deserialize(de)?;
                Ok(Value::Sequence(sequence))
            }

            fn visit_map<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let de = serde::de::value::MapAccessDeserializer::new(data);
                let mapping = Mapping::deserialize(de)?;
                Ok(Value::Mapping(mapping))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (tag, contents) = data.variant_seed(TagStringVisitor)?;
                let value = contents.newtype_variant()?;
                // let value: Value = contents.newtype_variant()?;
                Ok(Value::Tagged(Box::new(TaggedValue {
                    tag,
                    value,
                    // value: Spanned::dummy(value),
                })))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Value {
    fn deserialize_number<'de, V>(&self, visitor: V) -> Result<V::Value, SerdeError>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Number(n) => n.deserialize_any(visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }
}

fn visit_sequence<'de, V>(sequence: Sequence, visitor: V) -> Result<V::Value, SerdeError>
where
    V: Visitor<'de>,
{
    let len = sequence.len();
    let mut deserializer = SeqDeserializer::new(sequence);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(SerdeError::invalid_length(
            len,
            &"fewer elements in sequence",
        ))
    }
}

fn visit_sequence_ref<'de, V>(sequence: &'de Sequence, visitor: V) -> Result<V::Value, SerdeError>
where
    V: Visitor<'de>,
{
    let len = sequence.len();
    let mut deserializer = SeqRefDeserializer::new(sequence);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(SerdeError::invalid_length(
            len,
            &"fewer elements in sequence",
        ))
    }
}

fn visit_mapping<'de, V>(mapping: Mapping, visitor: V) -> Result<V::Value, SerdeError>
where
    V: Visitor<'de>,
{
    let len = mapping.len();
    let mut deserializer = MapDeserializer::new(mapping);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(SerdeError::invalid_length(len, &"fewer elements in map"))
    }
}

fn visit_mapping_ref<'de, V>(mapping: &'de Mapping, visitor: V) -> Result<V::Value, SerdeError>
where
    V: Visitor<'de>,
{
    let len = mapping.len();
    let mut deserializer = MapRefDeserializer::new(mapping);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.unwrap().len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(SerdeError::invalid_length(len, &"fewer elements in map"))
    }
}

impl<'de> Deserializer<'de> for Value {
    type Error = SerdeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Number(n) => n.deserialize_any(visitor),
            Value::String(v) => visitor.visit_string(v),
            Value::Sequence(v) => visit_sequence(v, visitor),
            Value::Mapping(v) => visit_mapping(v, visitor),
            Value::Tagged(tagged) => visitor.visit_enum(*tagged),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::Bool(v) => visitor.visit_bool(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::String(v) => visitor.visit_string(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::String(v) => visitor.visit_string(v),
            Value::Sequence(v) => visit_sequence(v, visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::Sequence(v) => visit_sequence(v, visitor),
            Value::Null => visit_sequence(Sequence::new(), visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::Mapping(v) => visit_mapping(v, visitor),
            Value::Null => visit_mapping(Mapping::new(), visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag;
        visitor.visit_enum(match self {
            Value::Tagged(tagged) => EnumDeserializer {
                tag: {
                    tag = tagged.tag.string;
                    crate::tag::nobang(&tag)
                },
                value: Some(tagged.value.inner),
            },
            Value::String(variant) => EnumDeserializer {
                tag: {
                    tag = variant;
                    &tag
                },
                value: None,
            },
            other => {
                return Err(Self::Error::invalid_type(
                    other.unexpected(),
                    &"a Value::Tagged enum",
                ));
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}

struct EnumDeserializer<'a> {
    tag: &'a str,
    value: Option<Value>,
}

impl<'de> EnumAccess<'de> for EnumDeserializer<'_> {
    type Error = SerdeError;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = StrDeserializer::<Self::Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantDeserializer { value: self.value };
        Ok((variant, visitor))
    }
}

struct VariantDeserializer {
    value: Option<Value>,
    // value: Option<Spanned<Value>>,
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = SerdeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            // Some(value) => value.inner.unit_variant(),
            Some(value) => value.unit_variant(),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            // Some(value) => value.inner.newtype_variant_seed(seed),
            Some(value) => value.newtype_variant_seed(seed),
            None => Err(Self::Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            // Some(value) => value.inner.tuple_variant(len, visitor),
            Some(value) => value.tuple_variant(len, visitor),
            None => Err(Self::Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            // Some(value) => value.inner.struct_variant(fields, visitor),
            Some(value) => value.struct_variant(fields, visitor),
            None => Err(Self::Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

pub(crate) struct SeqDeserializer {
    iter: std::vec::IntoIter<Spanned<Value>>,
}

impl SeqDeserializer {
    // pub(crate) fn new(vec: Vec<Value>) -> Self {
    pub(crate) fn new(vec: Vec<Spanned<Value>>) -> Self {
        SeqDeserializer {
            iter: vec.into_iter(),
        }
    }
}

impl<'de> Deserializer<'de> for SeqDeserializer {
    type Error = SerdeError;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = visitor.visit_seq(&mut self)?;
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(Self::Error::invalid_length(
                    len,
                    &"fewer elements in sequence",
                ))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = SerdeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value.inner).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub(crate) struct MapDeserializer {
    iter: <Mapping as IntoIterator>::IntoIter,
    value: Option<Value>,
}

impl MapDeserializer {
    pub(crate) fn new(map: Mapping) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = SerdeError;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value.inner);
                seed.deserialize(key.inner).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => panic!("visit_value called before visit_key"),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de> Deserializer<'de> for MapDeserializer {
    type Error = SerdeError;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, SerdeError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, SerdeError>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> Deserializer<'de> for &'de Value {
    type Error = SerdeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(*v),
            Value::Number(n) => n.deserialize_any(visitor),
            Value::String(v) => visitor.visit_borrowed_str(v),
            Value::Sequence(v) => visit_sequence_ref(v, visitor),
            Value::Mapping(v) => visit_mapping_ref(v, visitor),
            Value::Tagged(tagged) => visitor.visit_enum(&**tagged),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Bool(v) => visitor.visit_bool(*v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::String(v) => visitor.visit_borrowed_str(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::String(v) => visitor.visit_borrowed_str(v),
            Value::Sequence(v) => visit_sequence_ref(v, visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        static EMPTY: Sequence = Sequence::new();
        match self.untag_ref() {
            Value::Sequence(v) => visit_sequence_ref(v, visitor),
            Value::Null => visit_sequence_ref(&EMPTY, visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Mapping(v) => visit_mapping_ref(v, visitor),
            Value::Null => visitor.visit_map(&mut MapRefDeserializer {
                iter: None,
                value: None,
            }),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(match self {
            Value::Tagged(tagged) => EnumRefDeserializer {
                tag: crate::tag::nobang(&tagged.tag.string),
                value: Some(&tagged.value),
            },
            Value::String(variant) => EnumRefDeserializer {
                tag: variant,
                value: None,
            },
            other => {
                return Err(Self::Error::invalid_type(
                    other.unexpected(),
                    &"a Value::Tagged enum",
                ));
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct EnumRefDeserializer<'de> {
    tag: &'de str,
    value: Option<&'de Value>,
}

impl<'de> EnumAccess<'de> for EnumRefDeserializer<'de> {
    type Error = SerdeError;
    type Variant = VariantRefDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = BorrowedStrDeserializer::<Self::Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantRefDeserializer { value: self.value };
        Ok((variant, visitor))
    }
}

struct VariantRefDeserializer<'de> {
    value: Option<&'de Value>,
}

impl<'de> VariantAccess<'de> for VariantRefDeserializer<'de> {
    type Error = SerdeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            Some(value) => value.unit_variant(),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => value.newtype_variant_seed(seed),
            None => Err(Self::Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.tuple_variant(len, visitor),
            None => Err(Self::Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.struct_variant(fields, visitor),
            None => Err(Self::Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

pub(crate) struct SeqRefDeserializer<'de> {
    // iter: std::slice::Iter<'de, Value>,
    iter: std::slice::Iter<'de, Spanned<Value>>,
}

impl<'de> SeqRefDeserializer<'de> {
    // pub(crate) fn new(slice: &'de [Value]) -> Self {
    pub(crate) fn new(slice: &'de [Spanned<Value>]) -> Self {
        SeqRefDeserializer { iter: slice.iter() }
    }
}

impl<'de> Deserializer<'de> for SeqRefDeserializer<'de> {
    type Error = SerdeError;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = visitor.visit_seq(&mut self)?;
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(Self::Error::invalid_length(
                    len,
                    &"fewer elements in sequence",
                ))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> SeqAccess<'de> for SeqRefDeserializer<'de> {
    type Error = SerdeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(&value.inner).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub(crate) struct MapRefDeserializer<'de> {
    iter: Option<<&'de Mapping as IntoIterator>::IntoIter>,
    value: Option<&'de Value>,
}

impl<'de> MapRefDeserializer<'de> {
    pub(crate) fn new(map: &'de Mapping) -> Self {
        MapRefDeserializer {
            iter: Some(map.iter()),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapRefDeserializer<'de> {
    type Error = SerdeError;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.as_mut().and_then(Iterator::next) {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(&key.inner).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => panic!("visit_value called before visit_key"),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.as_ref()?.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de> Deserializer<'de> for MapRefDeserializer<'de> {
    type Error = SerdeError;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl Value {
    #[cold]
    fn invalid_type<E>(&self, exp: &dyn Expected) -> E
    where
        E: serde::de::Error,
    {
        serde::de::Error::invalid_type(self.unexpected(), exp)
    }

    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
        match self {
            Value::Null => Unexpected::Unit,
            Value::Bool(b) => Unexpected::Bool(*b),
            Value::Number(n) => number::serde::unexpected(n),
            Value::String(s) => Unexpected::Str(s),
            Value::Sequence(_) => Unexpected::Seq,
            Value::Mapping(_) => Unexpected::Map,
            Value::Tagged(_) => Unexpected::Enum,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mapping, Sequence, Spanned, Tag, TaggedValue, Value};
    use color_eyre::eyre;
    use indoc::indoc;
    use similar_asserts::assert_eq as sim_assert_eq;

    #[test]
    fn test_tag_resolution() -> eyre::Result<()> {
        crate::tests::init();
        // https://yaml.org/spec/1.2.2/#1032-tag-resolution
        let yaml = indoc! {"
            - null
            - Null
            - NULL
            - ~
            -
            - true
            - True
            - TRUE
            - false
            - False
            - FALSE
            - y
            - Y
            - yes
            - Yes
            - YES
            - n
            - N
            - no
            - No
            - NO
            - on
            - On
            - ON
            - off
            - Off
            - OFF
        "};

        let expected = Value::Sequence(
            [
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null, // could also be a String("")
                Value::Bool(true),
                Value::Bool(true),
                Value::Bool(true),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
                Value::String("y".to_owned()),
                Value::String("Y".to_owned()),
                Value::String("yes".to_owned()),
                Value::String("Yes".to_owned()),
                Value::String("YES".to_owned()),
                Value::String("n".to_owned()),
                Value::String("N".to_owned()),
                Value::String("no".to_owned()),
                Value::String("No".to_owned()),
                Value::String("NO".to_owned()),
                Value::String("on".to_owned()),
                Value::String("On".to_owned()),
                Value::String("ON".to_owned()),
                Value::String("off".to_owned()),
                Value::String("Off".to_owned()),
                Value::String("OFF".to_owned()),
            ]
            .into_iter()
            .map(Spanned::dummy)
            .collect(),
        );

        sim_assert_eq!(
            &crate::from_str(yaml)?.cleared_spans().into_inner(),
            &expected
        );
        Ok(())
    }

    #[test]
    fn test_literal_quoted_number() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {r#"
            "foo": |-
                7200
        "#};

        sim_assert_eq!(
            serde_yaml::from_str::<serde_yaml::Value>(yaml)?,
            serde_yaml::Value::from(serde_yaml::Mapping::from_iter([(
                "foo".into(),
                "7200".into()
            )]))
        );
        sim_assert_eq!(
            crate::from_str(yaml)?.cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([(
                "foo".to_string().into(),
                "7200".into(),
            )]))
        );

        let yaml = indoc! {r#"
            "foo": !!int |-
                7200
        "#};

        sim_assert_eq!(
            serde_yaml::from_str::<serde_yaml::Value>(yaml)?,
            serde_yaml::Value::from(serde_yaml::Mapping::from_iter([(
                "foo".into(),
                7200.into()
            )]))
        );
        sim_assert_eq!(
            crate::from_str(yaml)?.cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([(
                "foo".to_string().into(),
                7200.into(),
            )]))
        );
        Ok(())
    }

    #[test]
    fn test_python_safe_dump() -> eyre::Result<()> {
        crate::tests::init();

        // This matches output produced by PyYAML's `yaml.safe_dump` when using the
        // default_style parameter.
        //
        //    >>> import yaml
        //    >>> d = {"foo": 7200}
        //    >>> print(yaml.safe_dump(d, default_style="|"))
        //    "foo": !!int |-
        //      7200
        //
        let yaml = indoc! {r#"
            "foo": !!int |-
                7200
        "#};

        sim_assert_eq!(
            crate::from_str(yaml)?.cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([(
                "foo".to_string().into(),
                7200.into(),
            )]))
        );

        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct Frob {
            foo: u32,
        }
        sim_assert_eq!(
            crate::from_value::<Frob>(crate::from_str(yaml)?.as_ref())?,
            Frob { foo: 7200 }
        );
        Ok(())
    }

    #[test]
    fn test_empty_scalar() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = "thing:\n";

        sim_assert_eq!(
            serde_yaml::from_str::<serde_yaml::Value>(yaml)?,
            serde_yaml::Value::from(serde_yaml::Mapping::from_iter([(
                "thing".into(),
                serde_yaml::Value::Null,
            )]))
        );

        sim_assert_eq!(
            crate::from_str(yaml)?.cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([(
                "thing".to_string().into(),
                Value::Null.into(),
            )]))
        );

        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct Struct<T> {
            thing: T,
        }

        sim_assert_eq!(
            crate::from_value::<Struct<_>>(crate::from_str(yaml)?.as_ref())?,
            Struct {
                thing: Sequence::new()
            }
        );
        Ok(())
    }

    #[test]
    fn test_no_required_fields() -> eyre::Result<()> {
        use std::collections::BTreeMap;

        crate::tests::init();

        #[derive(serde::Deserialize, PartialEq, Debug)]
        pub struct NoRequiredFields {
            optional: Option<usize>,
        }

        for document in ["", "# comment\n"] {
            let expected = NoRequiredFields { optional: None };
            let deserialized: Spanned<Value> = crate::from_str(document)?;
            sim_assert_eq!(deserialized, Value::Null);
            sim_assert_eq!(
                crate::from_value::<NoRequiredFields>(&deserialized)?,
                expected
            );

            let expected = Vec::<String>::new();
            let deserialized: Spanned<Value> = crate::from_str(document)?;
            sim_assert_eq!(deserialized, Value::Null);
            sim_assert_eq!(crate::from_value::<Vec<String>>(&deserialized)?, expected);

            let expected: BTreeMap<char, usize> = BTreeMap::new();
            let deserialized: Spanned<Value> = crate::from_str(document)?;
            sim_assert_eq!(deserialized, Value::Null);
            sim_assert_eq!(
                crate::from_value::<BTreeMap<_, _>>(&deserialized)?,
                expected
            );

            let expected = None;
            let deserialized: Spanned<Value> = crate::from_str(document)?;
            sim_assert_eq!(deserialized, Value::Null);
            sim_assert_eq!(
                crate::from_value::<Option<String>>(&deserialized)?,
                expected
            );
        }

        Ok(())
    }

    #[test]
    fn test_ignore_tag() -> eyre::Result<()> {
        use std::collections::BTreeMap;
        crate::tests::init();

        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Data {
            struc: Struc, // spellcheck:ignore-line
            tuple: Tuple,
            newtype: Newtype,
            map: BTreeMap<char, usize>,
            vec: Vec<usize>,
        }

        // spellcheck:ignore-block
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Struc {
            x: usize,
        }

        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Tuple(usize, usize);

        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Newtype(usize);

        // spellcheck:ignore-block
        let yaml = indoc! {"
            struc: !wat
              x: 0
            tuple: !wat
              - 0
              - 0
            newtype: !wat 0
            map: !wat
              x: 0
            vec: !wat
              - 0
        "};

        let value = crate::from_str(yaml)?;
        let expected: Value = Mapping::from_iter([
            (
                "struc".into(), // spellcheck:ignore-line
                TaggedValue {
                    tag: "wat".into(), // spellcheck:ignore-line
                    value: Mapping::from_iter([("x".into(), 0.into())]).into(),
                }
                .into(),
            ),
            (
                "tuple".into(),
                TaggedValue {
                    tag: Tag::new("wat"),
                    value: Sequence::from_iter([0.into(), 0.into()]).into(),
                }
                .into(),
            ),
            (
                "newtype".into(),
                TaggedValue {
                    tag: Tag::new("wat"),
                    value: 0.into(),
                }
                .into(),
            ),
            (
                "map".into(),
                TaggedValue {
                    tag: Tag::new("wat"),
                    value: Mapping::from_iter([("x".into(), 0.into())]).into(),
                }
                .into(),
            ),
            (
                "vec".into(),
                TaggedValue {
                    tag: Tag::new("wat"),
                    value: Sequence::from_iter([0.into()]).into(),
                }
                .into(),
            ),
        ])
        .into();
        sim_assert_eq!(value.clone().cleared_spans().into_inner(), expected);

        let expected = Data {
            struc: Struc { x: 0 }, // spellcheck:ignore-line
            tuple: Tuple(0, 0),
            newtype: Newtype(0),
            map: {
                let mut map = BTreeMap::new();
                map.insert('x', 0);
                map
            },
            vec: vec![0],
        };
        sim_assert_eq!(crate::from_value::<Data>(&value)?, expected);
        Ok(())
    }

    #[test]
    fn test_nan() -> eyre::Result<()> {
        crate::tests::init();
        // There is no negative NaN in YAML.
        assert!(crate::from_value::<f32>(crate::from_str(".nan")?.as_ref())?.is_sign_positive());
        assert!(crate::from_value::<f32>(crate::from_str(".nan")?.as_ref())?.is_sign_positive());
        Ok(())
    }

    #[ignore = "we need to prevent these ddos attacks"]
    #[test]
    fn test_bomb() -> eyre::Result<()> {
        crate::tests::init();
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Data {
            expected: String,
        }

        // This would deserialize an astronomical number of elements if we were vulnerable.
        let yaml = indoc! {"
            a: &a ~
            b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]
            c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]
            d: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]
            e: &e [*d,*d,*d,*d,*d,*d,*d,*d,*d]
            f: &f [*e,*e,*e,*e,*e,*e,*e,*e,*e]
            g: &g [*f,*f,*f,*f,*f,*f,*f,*f,*f]
            h: &h [*g,*g,*g,*g,*g,*g,*g,*g,*g]
            i: &i [*h,*h,*h,*h,*h,*h,*h,*h,*h]
            j: &j [*i,*i,*i,*i,*i,*i,*i,*i,*i]
            k: &k [*j,*j,*j,*j,*j,*j,*j,*j,*j]
            l: &l [*k,*k,*k,*k,*k,*k,*k,*k,*k]
            m: &m [*l,*l,*l,*l,*l,*l,*l,*l,*l]
            n: &n [*m,*m,*m,*m,*m,*m,*m,*m,*m]
            o: &o [*n,*n,*n,*n,*n,*n,*n,*n,*n]
            p: &p [*o,*o,*o,*o,*o,*o,*o,*o,*o]
            q: &q [*p,*p,*p,*p,*p,*p,*p,*p,*p]
            r: &r [*q,*q,*q,*q,*q,*q,*q,*q,*q]
            s: &s [*r,*r,*r,*r,*r,*r,*r,*r,*r]
            t: &t [*s,*s,*s,*s,*s,*s,*s,*s,*s]
            u: &u [*t,*t,*t,*t,*t,*t,*t,*t,*t]
            v: &v [*u,*u,*u,*u,*u,*u,*u,*u,*u]
            w: &w [*v,*v,*v,*v,*v,*v,*v,*v,*v]
            x: &x [*w,*w,*w,*w,*w,*w,*w,*w,*w]
            y: &y [*x,*x,*x,*x,*x,*x,*x,*x,*x]
            z: &z [*y,*y,*y,*y,*y,*y,*y,*y,*y]
            expected: string
        "};

        let _expected = Data {
            expected: "string".to_owned(),
        };

        let value = crate::from_str(yaml)?;
        sim_assert_eq!(
            value.cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([("expected".into(), "string".into())])),
        );
        Ok(())
    }

    #[test]
    fn test_byte_order_mark() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = "\u{feff}- 0\n";
        let value = crate::from_str(yaml)?;
        sim_assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Sequence::from_iter([0.into()])),
        );

        sim_assert_eq!(crate::from_value::<Vec<u64>>(&value)?, vec![0]);
        Ok(())
    }

    #[ignore = "stateful deserialization not supported, as it requires deserializtion directly from YAML"]
    #[allow(dead_code)]
    #[test]
    fn test_stateful() -> eyre::Result<()> {
        crate::tests::init();

        struct Seed(i64);

        impl<'de> serde::de::DeserializeSeed<'de> for Seed {
            type Value = i64;
            fn deserialize<D>(self, deserializer: D) -> Result<i64, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                struct Visitor(i64);
                impl serde::de::Visitor<'_> for Visitor {
                    type Value = i64;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(formatter, "an integer")
                    }

                    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<i64, E> {
                        Ok(v * self.0)
                    }

                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<i64, E> {
                        Ok(v as i64 * self.0)
                    }
                }

                deserializer.deserialize_any(Visitor(self.0))
            }
        }

        let cases = [("3", 5, 15), ("6", 7, 42), ("-5", 9, -45)];
        for &(yaml, _seed, _expected) in &cases {
            // let deserializer = crate::de::Deserializer::from_str(yaml);
            // let deserialized = Seed(seed).deserialize(deserializer)?;
            // sim_assert_eq!(expected, deserialized);

            crate::from_str(yaml)?;
        }
        Ok(())
    }
}
