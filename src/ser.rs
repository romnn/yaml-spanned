use crate::error::SerdeError;

type Result<T, E = SerdeError> = std::result::Result<T, E>;

// NOTE: struct to string serialization should be done by a more generic yaml library
//
// /// Serialize the given data structure as YAML into the IO stream.
// ///
// /// Serialization can fail if `T`'s implementation of `Serialize` decides to
// /// return an error.
// pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
// where
//     W: std::io::Write,
//     T: ?Sized + serde::ser::Serialize,
// {
//     let mut serializer = Serializer::new(writer);
//     value.serialize(&mut serializer)
// }
//
// /// Serialize the given data structure as a String of YAML.
// ///
// /// Serialization can fail if `T`'s implementation of `Serialize` decides to
// /// return an error.
// pub fn to_string<T>(value: &T) -> Result<String>
// where
//     T: ?Sized + serde::ser::Serialize,
// {
//     let mut vec = Vec::with_capacity(128);
//     to_writer(&mut vec, value)?;
//     String::from_utf8(vec).map_err(SerdeError::FromUtf8)
// }

pub mod value {
    use super::Result;
    use crate::error::SerdeError;
    use crate::tag::serde::{MaybeTag, check_for_tag};
    use crate::{Mapping, Number, Sequence, Spanned, Tag, TaggedValue, Value, to_value};

    impl serde::Serialize for Spanned<Value> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.as_ref().serialize(serializer)
        }
    }

    impl serde::Serialize for Value {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                Value::Null => serializer.serialize_unit(),
                Value::Bool(b) => serializer.serialize_bool(*b),
                Value::Number(n) => n.serialize(serializer),
                Value::String(s) => serializer.serialize_str(s),
                Value::Sequence(seq) => seq.serialize(serializer),
                Value::Mapping(mapping) => {
                    use serde::ser::SerializeMap;
                    let mut map = serializer.serialize_map(Some(mapping.len()))?;
                    for (k, v) in mapping {
                        map.serialize_entry(k.as_ref(), v.as_ref())?;
                    }
                    map.end()
                }
                Value::Tagged(tagged) => tagged.serialize(serializer),
            }
        }
    }

    /// Serializer whose output is a `Value`.
    ///
    /// This is the serializer that backs [`yaml_spanned::to_value`][crate::to_value].
    /// Unlike the main yaml_spanned serializer which goes from some serializable
    /// value of type `T` to YAML text, this one goes from `T` to
    /// `yaml_spanned::Value`.
    ///
    /// The `to_value` function is implementable as:
    ///
    /// ```
    /// use serde::Serialize;
    /// use yaml_spanned::Value;
    ///
    /// pub fn to_value<T>(input: T) -> Result<Value, yaml_spanned::error::SerdeError>
    /// where
    ///     T: Serialize,
    /// {
    ///     input.serialize(yaml_spanned::Serializer)
    /// }
    /// ```
    pub struct Serializer;

    impl serde::ser::Serializer for Serializer {
        type Ok = Value;
        type Error = SerdeError;

        type SerializeSeq = SerializeArray;
        type SerializeTuple = SerializeArray;
        type SerializeTupleStruct = SerializeArray;
        type SerializeTupleVariant = SerializeTupleVariant;
        type SerializeMap = SerializeMap;
        type SerializeStruct = SerializeStruct;
        type SerializeStructVariant = SerializeStructVariant;

        fn serialize_bool(self, v: bool) -> Result<Value> {
            Ok(Value::Bool(v))
        }

        fn serialize_i8(self, v: i8) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_i16(self, v: i16) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_i32(self, v: i32) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_i64(self, v: i64) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_i128(self, v: i128) -> Result<Value> {
            if let Ok(v) = u64::try_from(v) {
                self.serialize_u64(v)
            } else if let Ok(v) = i64::try_from(v) {
                self.serialize_i64(v)
            } else {
                Ok(Value::String(v.to_string()))
            }
        }

        fn serialize_u8(self, v: u8) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_u16(self, v: u16) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_u32(self, v: u32) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_u64(self, v: u64) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_u128(self, v: u128) -> Result<Value> {
            if let Ok(v) = u64::try_from(v) {
                self.serialize_u64(v)
            } else {
                Ok(Value::String(v.to_string()))
            }
        }

        fn serialize_f32(self, v: f32) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_f64(self, v: f64) -> Result<Value> {
            Ok(Value::Number(Number::from(v)))
        }

        fn serialize_char(self, value: char) -> Result<Value> {
            Ok(Value::String(value.to_string()))
        }

        fn serialize_str(self, value: &str) -> Result<Value> {
            Ok(Value::String(value.to_owned()))
        }

        fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
            let vec = value
                .iter()
                .map(|&b| Spanned::dummy(Value::Number(Number::from(b))))
                .collect();
            Ok(Value::Sequence(vec))
        }

        fn serialize_unit(self) -> Result<Value> {
            Ok(Value::Null)
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
            self.serialize_unit()
        }

        fn serialize_unit_variant(
            self,
            _name: &str,
            _variant_index: u32,
            variant: &str,
        ) -> Result<Value> {
            Ok(Value::String(variant.to_owned()))
        }

        fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
        where
            T: ?Sized + serde::Serialize,
        {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T>(
            self,
            _name: &str,
            _variant_index: u32,
            variant: &str,
            value: &T,
        ) -> Result<Value>
        where
            T: ?Sized + serde::Serialize,
        {
            if variant.is_empty() {
                return Err(SerdeError::EmptyTag);
            }
            Ok(Value::Tagged(Box::new(TaggedValue {
                tag: Tag::new(variant),
                value: Spanned::dummy(to_value(value)?),
            })))
        }

        fn serialize_none(self) -> Result<Value> {
            self.serialize_unit()
        }

        fn serialize_some<V>(self, value: &V) -> Result<Value>
        where
            V: ?Sized + serde::Serialize,
        {
            value.serialize(self)
        }

        fn serialize_seq(self, len: Option<usize>) -> Result<SerializeArray> {
            let sequence = match len {
                None => Sequence::new(),
                Some(len) => Sequence::with_capacity(len),
            };
            Ok(SerializeArray { sequence })
        }

        fn serialize_tuple(self, len: usize) -> Result<SerializeArray> {
            self.serialize_seq(Some(len))
        }

        fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SerializeArray> {
            self.serialize_seq(Some(len))
        }

        fn serialize_tuple_variant(
            self,
            _enum: &'static str,
            _idx: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<SerializeTupleVariant> {
            if variant.is_empty() {
                return Err(SerdeError::EmptyTag);
            }
            Ok(SerializeTupleVariant {
                tag: variant,
                sequence: Sequence::with_capacity(len),
            })
        }

        fn serialize_map(self, len: Option<usize>) -> Result<SerializeMap> {
            if len == Some(1) {
                Ok(SerializeMap::CheckForTag)
            } else {
                Ok(SerializeMap::Untagged {
                    mapping: Mapping::new(),
                    next_key: None,
                })
            }
        }

        fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<SerializeStruct> {
            Ok(SerializeStruct {
                mapping: Mapping::new(),
            })
        }

        fn serialize_struct_variant(
            self,
            _enum: &'static str,
            _idx: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<SerializeStructVariant> {
            if variant.is_empty() {
                return Err(SerdeError::EmptyTag);
            }
            Ok(SerializeStructVariant {
                tag: variant,
                mapping: Mapping::new(),
            })
        }
    }

    pub struct SerializeArray {
        sequence: Sequence,
    }

    impl serde::ser::SerializeSeq for SerializeArray {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
        where
            T: ?Sized + serde::Serialize,
        {
            self.sequence.push(Spanned::dummy(to_value(elem)?));
            Ok(())
        }

        fn end(self) -> Result<Value> {
            Ok(Value::Sequence(self.sequence))
        }
    }

    impl serde::ser::SerializeTuple for SerializeArray {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
        where
            T: ?Sized + serde::Serialize,
        {
            serde::ser::SerializeSeq::serialize_element(self, elem)
        }

        fn end(self) -> Result<Value> {
            serde::ser::SerializeSeq::end(self)
        }
    }

    impl serde::ser::SerializeTupleStruct for SerializeArray {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_field<V>(&mut self, value: &V) -> Result<()>
        where
            V: ?Sized + serde::Serialize,
        {
            serde::ser::SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Value> {
            serde::ser::SerializeSeq::end(self)
        }
    }

    pub struct SerializeTupleVariant {
        tag: &'static str,
        sequence: Sequence,
    }

    impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_field<V>(&mut self, v: &V) -> Result<()>
        where
            V: ?Sized + serde::Serialize,
        {
            self.sequence.push(Spanned::dummy(to_value(v)?));
            Ok(())
        }

        fn end(self) -> Result<Value> {
            Ok(Value::Tagged(Box::new(TaggedValue {
                tag: Tag::new(self.tag),
                value: Spanned::dummy(Value::Sequence(self.sequence)),
            })))
        }
    }

    pub enum SerializeMap {
        CheckForTag,
        Tagged(TaggedValue),
        Untagged {
            mapping: Mapping,
            next_key: Option<Value>,
        },
    }

    impl serde::ser::SerializeMap for SerializeMap {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_key<T>(&mut self, key: &T) -> Result<()>
        where
            T: ?Sized + serde::Serialize,
        {
            let key = Some(to_value(key)?);
            match self {
                SerializeMap::CheckForTag => {
                    *self = SerializeMap::Untagged {
                        mapping: Mapping::new(),
                        next_key: key,
                    };
                }
                SerializeMap::Tagged(tagged) => {
                    let mut mapping = Mapping::new();
                    mapping.insert(
                        Spanned::dummy(Value::String(tagged.tag.to_string())),
                        std::mem::take(&mut tagged.value),
                    );
                    *self = SerializeMap::Untagged {
                        mapping,
                        next_key: key,
                    };
                }
                SerializeMap::Untagged { next_key, .. } => *next_key = key,
            }
            Ok(())
        }

        fn serialize_value<T>(&mut self, value: &T) -> Result<()>
        where
            T: ?Sized + serde::Serialize,
        {
            let (mapping, key) = match self {
                SerializeMap::CheckForTag | SerializeMap::Tagged(_) => unreachable!(),
                SerializeMap::Untagged { mapping, next_key } => (mapping, next_key),
            };
            match key.take() {
                Some(key) => mapping.insert(Spanned::dummy(key), Spanned::dummy(to_value(value)?)),
                None => panic!("serialize_value called before serialize_key"),
            };
            Ok(())
        }

        fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
        where
            K: ?Sized + serde::Serialize,
            V: ?Sized + serde::Serialize,
        {
            struct CheckForTag;
            struct NotTag<T> {
                delegate: T,
            }

            impl serde::ser::Serializer for CheckForTag {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                type SerializeSeq = NotTag<SerializeArray>;
                type SerializeTuple = NotTag<SerializeArray>;
                type SerializeTupleStruct = NotTag<SerializeArray>;
                type SerializeTupleVariant = NotTag<SerializeTupleVariant>;
                type SerializeMap = NotTag<SerializeMap>;
                type SerializeStruct = NotTag<SerializeStruct>;
                type SerializeStructVariant = NotTag<SerializeStructVariant>;

                fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
                    Serializer.serialize_bool(v).map(MaybeTag::NotTag)
                }

                fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
                    Serializer.serialize_i8(v).map(MaybeTag::NotTag)
                }

                fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
                    Serializer.serialize_i16(v).map(MaybeTag::NotTag)
                }

                fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
                    Serializer.serialize_i32(v).map(MaybeTag::NotTag)
                }

                fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
                    Serializer.serialize_i64(v).map(MaybeTag::NotTag)
                }

                fn serialize_i128(self, v: i128) -> Result<Self::Ok> {
                    Serializer.serialize_i128(v).map(MaybeTag::NotTag)
                }

                fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
                    Serializer.serialize_u8(v).map(MaybeTag::NotTag)
                }

                fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
                    Serializer.serialize_u16(v).map(MaybeTag::NotTag)
                }

                fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
                    Serializer.serialize_u32(v).map(MaybeTag::NotTag)
                }

                fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
                    Serializer.serialize_u64(v).map(MaybeTag::NotTag)
                }

                fn serialize_u128(self, v: u128) -> Result<Self::Ok> {
                    Serializer.serialize_u128(v).map(MaybeTag::NotTag)
                }

                fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
                    Serializer.serialize_f32(v).map(MaybeTag::NotTag)
                }

                fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
                    Serializer.serialize_f64(v).map(MaybeTag::NotTag)
                }

                fn serialize_char(self, value: char) -> Result<Self::Ok> {
                    Serializer.serialize_char(value).map(MaybeTag::NotTag)
                }

                fn serialize_str(self, value: &str) -> Result<Self::Ok> {
                    Serializer.serialize_str(value).map(MaybeTag::NotTag)
                }

                fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
                    Serializer.serialize_bytes(value).map(MaybeTag::NotTag)
                }

                fn serialize_unit(self) -> Result<Self::Ok> {
                    Serializer.serialize_unit().map(MaybeTag::NotTag)
                }

                fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
                    Serializer.serialize_unit_struct(name).map(MaybeTag::NotTag)
                }

                fn serialize_unit_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                ) -> Result<Self::Ok> {
                    Serializer
                        .serialize_unit_variant(name, variant_index, variant)
                        .map(MaybeTag::NotTag)
                }

                fn serialize_newtype_struct<T>(
                    self,
                    name: &'static str,
                    value: &T,
                ) -> Result<Self::Ok>
                where
                    T: ?Sized + serde::Serialize,
                {
                    Serializer
                        .serialize_newtype_struct(name, value)
                        .map(MaybeTag::NotTag)
                }

                fn serialize_newtype_variant<T>(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    value: &T,
                ) -> Result<Self::Ok>
                where
                    T: ?Sized + serde::Serialize,
                {
                    Serializer
                        .serialize_newtype_variant(name, variant_index, variant, value)
                        .map(MaybeTag::NotTag)
                }

                fn serialize_none(self) -> Result<Self::Ok> {
                    Serializer.serialize_none().map(MaybeTag::NotTag)
                }

                fn serialize_some<V>(self, value: &V) -> Result<Self::Ok>
                where
                    V: ?Sized + serde::Serialize,
                {
                    Serializer.serialize_some(value).map(MaybeTag::NotTag)
                }

                fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_seq(len)?,
                    })
                }

                fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_tuple(len)?,
                    })
                }

                fn serialize_tuple_struct(
                    self,
                    name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleStruct> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_tuple_struct(name, len)?,
                    })
                }

                fn serialize_tuple_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleVariant> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_tuple_variant(
                            name,
                            variant_index,
                            variant,
                            len,
                        )?,
                    })
                }

                fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_map(len)?,
                    })
                }

                fn serialize_struct(
                    self,
                    name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeStruct> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_struct(name, len)?,
                    })
                }

                fn serialize_struct_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeStructVariant> {
                    Ok(NotTag {
                        delegate: Serializer.serialize_struct_variant(
                            name,
                            variant_index,
                            variant,
                            len,
                        )?,
                    })
                }

                fn collect_str<T>(self, value: &T) -> Result<Self::Ok>
                where
                    T: ?Sized + std::fmt::Display,
                {
                    Ok(match check_for_tag(value) {
                        MaybeTag::Tag(tag) => MaybeTag::Tag(tag),
                        MaybeTag::NotTag(string) => MaybeTag::NotTag(Value::String(string)),
                    })
                }
            }

            impl serde::ser::SerializeSeq for NotTag<SerializeArray> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
                where
                    T: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_element(elem)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            impl serde::ser::SerializeTuple for NotTag<SerializeArray> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
                where
                    T: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_element(elem)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            impl serde::ser::SerializeTupleStruct for NotTag<SerializeArray> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_field<V>(&mut self, value: &V) -> Result<()>
                where
                    V: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_field(value)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            impl serde::ser::SerializeTupleVariant for NotTag<SerializeTupleVariant> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_field<V>(&mut self, v: &V) -> Result<()>
                where
                    V: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_field(v)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            impl serde::ser::SerializeMap for NotTag<SerializeMap> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_key<T>(&mut self, key: &T) -> Result<()>
                where
                    T: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_key(key)
                }

                fn serialize_value<T>(&mut self, value: &T) -> Result<()>
                where
                    T: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_value(value)
                }

                fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
                where
                    K: ?Sized + serde::Serialize,
                    V: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_entry(key, value)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            impl serde::ser::SerializeStruct for NotTag<SerializeStruct> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<()>
                where
                    V: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_field(key, value)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            impl serde::ser::SerializeStructVariant for NotTag<SerializeStructVariant> {
                type Ok = MaybeTag<Value>;
                type Error = SerdeError;

                fn serialize_field<V>(&mut self, field: &'static str, v: &V) -> Result<()>
                where
                    V: ?Sized + serde::Serialize,
                {
                    self.delegate.serialize_field(field, v)
                }

                fn end(self) -> Result<Self::Ok> {
                    self.delegate.end().map(MaybeTag::NotTag)
                }
            }

            match self {
                SerializeMap::CheckForTag => {
                    let key = key.serialize(CheckForTag)?;
                    let mut mapping = Mapping::new();
                    *self = match key {
                        MaybeTag::Tag(string) => SerializeMap::Tagged(TaggedValue {
                            tag: Tag::new(string),
                            value: Spanned::dummy(to_value(value)?),
                        }),
                        MaybeTag::NotTag(key) => {
                            mapping.insert(Spanned::dummy(key), Spanned::dummy(to_value(value)?));
                            SerializeMap::Untagged {
                                mapping,
                                next_key: None,
                            }
                        }
                    };
                }
                SerializeMap::Tagged(tagged) => {
                    let mut mapping = Mapping::new();
                    mapping.insert(
                        Spanned::dummy(Value::String(tagged.tag.to_string())),
                        std::mem::take(&mut tagged.value),
                    );
                    mapping.insert(
                        Spanned::dummy(to_value(key)?),
                        Spanned::dummy(to_value(value)?),
                    );
                    *self = SerializeMap::Untagged {
                        mapping,
                        next_key: None,
                    };
                }
                SerializeMap::Untagged { mapping, .. } => {
                    mapping.insert(
                        Spanned::dummy(to_value(key)?),
                        Spanned::dummy(to_value(value)?),
                    );
                }
            }
            Ok(())
        }

        fn end(self) -> Result<Value> {
            Ok(match self {
                SerializeMap::CheckForTag => Value::Mapping(Mapping::new()),
                SerializeMap::Tagged(tagged) => Value::Tagged(Box::new(tagged)),
                SerializeMap::Untagged { mapping, .. } => Value::Mapping(mapping),
            })
        }
    }

    pub struct SerializeStruct {
        mapping: Mapping,
    }

    impl serde::ser::SerializeStruct for SerializeStruct {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<()>
        where
            V: ?Sized + serde::Serialize,
        {
            self.mapping.insert(
                Spanned::dummy(to_value(key)?),
                Spanned::dummy(to_value(value)?),
            );
            Ok(())
        }

        fn end(self) -> Result<Value> {
            Ok(Value::Mapping(self.mapping))
        }
    }

    pub struct SerializeStructVariant {
        tag: &'static str,
        mapping: Mapping,
    }

    impl serde::ser::SerializeStructVariant for SerializeStructVariant {
        type Ok = Value;
        type Error = SerdeError;

        fn serialize_field<V>(&mut self, field: &'static str, v: &V) -> Result<()>
        where
            V: ?Sized + serde::Serialize,
        {
            self.mapping.insert(
                Spanned::dummy(to_value(field)?),
                Spanned::dummy(to_value(v)?),
            );
            Ok(())
        }

        fn end(self) -> Result<Value> {
            Ok(Value::Tagged(Box::new(TaggedValue {
                tag: Tag::new(self.tag),
                value: Spanned::dummy(Value::Mapping(self.mapping)),
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mapping, Sequence, SpannedValue, TaggedValue, Value};
    use color_eyre::eyre;
    use indoc::indoc;
    use similar_asserts::assert_eq as sim_assert_eq;

    fn test_serde<T>(yaml: &str, value: Value) -> eyre::Result<()>
    where
        T: serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug,
    {
        let value_de = crate::from_str(yaml)?;
        dbg!(&value_de);
        let thing = crate::from_value::<T>(&value)?;
        dbg!(&thing);
        let value_ser = crate::to_value(&thing)?;
        dbg!(&value_ser);
        sim_assert_eq!(value_de, value);
        sim_assert_eq!(value_ser, value);
        Ok(())
    }

    #[test]
    fn test_default() {
        sim_assert_eq!(Value::default(), Value::Null);
    }

    #[test]
    fn test_int() -> eyre::Result<()> {
        crate::tests::init();
        test_serde::<i64>("256", Value::from(256))?;
        Ok(())
    }

    #[test]
    fn test_int_max_u64() -> eyre::Result<()> {
        crate::tests::init();
        test_serde::<u64>("18446744073709551615", Value::from(u64::MAX))?;
        Ok(())
    }

    #[test]
    fn test_int_min_i64() -> eyre::Result<()> {
        crate::tests::init();
        test_serde::<i64>("-9223372036854775808", Value::from(i64::MIN))?;
        Ok(())
    }

    #[test]
    fn test_int_max_i64() -> eyre::Result<()> {
        crate::tests::init();
        test_serde::<i64>("9223372036854775807", Value::from(i64::MAX))?;
        Ok(())
    }

    #[ignore = "cannot represent i128 yet"]
    #[test]
    fn test_i128_small() -> eyre::Result<()> {
        crate::tests::init();
        // test_serde::<i128>("-256", Value::from(-256i128))?;
        Ok(())
    }

    #[ignore = "cannot represent u128 yet"]
    #[test]
    fn test_u128_small() -> eyre::Result<()> {
        crate::tests::init();
        // test_serde::<u128>("256", Value::from(256u128))?;
        Ok(())
    }

    #[test]
    fn test_float() -> eyre::Result<()> {
        crate::tests::init();
        test_serde::<f64>("25.6", Value::from(25.6))?;

        test_serde::<f64>("25.", Value::from(25.))?;

        test_serde::<f64>(".inf", Value::from(f64::INFINITY))?;

        test_serde::<f64>("-.inf", Value::from(f64::NEG_INFINITY))?;

        test_serde::<f64>("-.inf", Value::from(f64::NEG_INFINITY))?;

        let value = crate::from_str(indoc! {"
            .nan
        "})?;
        let float: f64 = crate::from_value(&value)?;
        assert!(float.is_nan());
        Ok(())
    }

    #[test]
    fn test_float32() -> eyre::Result<()> {
        crate::tests::init();
        test_serde::<f32>("25.5", Value::from(25.5))?;
        test_serde::<f32>(".inf", Value::from(f32::INFINITY))?;
        test_serde::<f32>("-.inf", Value::from(f32::NEG_INFINITY))?;

        let value = crate::from_str(".nan")?;
        let single_float: f32 = crate::from_value(&value)?;
        assert!(single_float.is_nan());
        Ok(())
    }

    #[test]
    fn test_vec() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = indoc! {"
            - 1
            - 2
            - 3
        "};
        let value = [1, 2, 3].into_iter().map(Value::from).collect();
        test_serde::<Vec<usize>>(&yaml, value)?;
        Ok(())
    }

    #[test]
    fn test_map() -> eyre::Result<()> {
        crate::tests::init();
        use crate::SpannedValue;
        use std::collections::BTreeMap;
        let yaml = indoc! {"
            x: 1
            y: 2
        "};
        let value: crate::Mapping = [
            (SpannedValue::from("x"), SpannedValue::from(1)),
            (SpannedValue::from("y"), SpannedValue::from(2)),
        ]
        .into_iter()
        .collect();
        test_serde::<BTreeMap<String, usize>>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_basic_struct() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Basic {
            x: isize,
            y: String,
            z: bool,
        }
        // let thing = Basic {
        //     x: -4,
        //     y: "hi\tquoted".to_owned(),
        //     z: true,
        // };
        let yaml = indoc! {r#"
            x: -4
            y: "hi\tquoted"
            z: true
        "#};
        let value: Mapping = [
            (SpannedValue::from("x"), SpannedValue::from(-4)),
            (SpannedValue::from("y"), SpannedValue::from("hi\tquoted")),
            (SpannedValue::from("z"), SpannedValue::from(true)),
        ]
        .into_iter()
        .collect();
        test_serde::<Basic>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_string_escapes() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ascii
        "};
        test_serde::<String>(&yaml, Value::from("ascii"))?;

        let yaml = indoc! {r#"
            "\0\a\b\t\n\v\f\r\e\"\\\N\L\P"
        "#};
        test_serde::<String>(
            &yaml,
            Value::from("\0\u{7}\u{8}\t\n\u{b}\u{c}\r\u{1b}\"\\\u{85}\u{2028}\u{2029}"),
        )?;

        let yaml = indoc! {r#"
            "\x1F\uFEFF"
        "#};
        test_serde::<String>(&yaml, Value::from("\u{1f}\u{feff}"))?;

        let yaml = indoc! {"
            🎉
        "};
        test_serde::<String>(&yaml, Value::from("\u{1f389}"))?;
        Ok(())
    }

    #[test]
    fn test_multiline_string() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Struct {
            trailing_newline: String,
            no_trailing_newline: String,
        }
        // let thing = Struct {
        //     trailing_newline: "aaa\nbbb\n".to_owned(),
        //     no_trailing_newline: "aaa\nbbb".to_owned(),
        // };
        let yaml = indoc! {"
            trailing_newline: |
              aaa
              bbb
            no_trailing_newline: |-
              aaa
              bbb
        "};
        let value: Mapping = [
            (
                SpannedValue::from("trailing_newline"),
                SpannedValue::from("aaa\nbbb\n"),
            ),
            (
                SpannedValue::from("no_trailing_newline"),
                SpannedValue::from("aaa\nbbb"),
            ),
        ]
        .into_iter()
        .collect();
        test_serde::<Struct>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_strings_needing_quote() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Struct {
            boolean: String,
            integer: String,
            void: String,
            leading_zeros: String,
        }
        // let thing = Struct {
        //     boolean: "true".to_owned(),
        //     integer: "1".to_owned(),
        //     void: "null".to_owned(),
        //     leading_zeros: "007".to_owned(),
        // };
        let yaml = indoc! {"
            boolean: 'true'
            integer: '1'
            void: 'null'
            leading_zeros: '007'
        "};
        let value: Mapping = [
            (SpannedValue::from("boolean"), SpannedValue::from("true")),
            (SpannedValue::from("integer"), SpannedValue::from("1")),
            (SpannedValue::from("void"), SpannedValue::from("null")),
            (
                SpannedValue::from("leading_zeros"),
                SpannedValue::from("007"),
            ),
        ]
        .into_iter()
        .collect();
        test_serde::<Struct>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_nested_vec() -> eyre::Result<()> {
        crate::tests::init();
        // let thing = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let yaml = indoc! {"
            - - 1
              - 2
              - 3
            - - 4
              - 5
              - 6
        "};
        let value: Sequence = [
            Sequence::from(
                [1, 2, 3]
                    .into_iter()
                    .map(SpannedValue::from)
                    .collect::<Vec<_>>(),
            ),
            Sequence::from(
                [4, 5, 6]
                    .into_iter()
                    .map(SpannedValue::from)
                    .collect::<Vec<_>>(),
            ),
        ]
        .into_iter()
        .map(SpannedValue::from)
        .collect();
        test_serde::<Vec<Vec<usize>>>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_nested_struct() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Outer {
            inner: Inner,
        }
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Inner {
            v: u16,
        }
        // let thing = Outer {
        //     inner: Inner { v: 512 },
        // };
        let yaml = indoc! {"
            inner:
              v: 512
        "};
        let value = Mapping::from_iter([(
            SpannedValue::from("inner"),
            SpannedValue::from(Mapping::from_iter([(
                SpannedValue::from("v"),
                SpannedValue::from(512),
            )])),
        )]);
        test_serde::<Outer>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_nested_enum() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Outer {
            Inner(Inner),
        }
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Inner {
            Unit,
        }
        let yaml = indoc! {"
            !Inner Unit
        "};
        let value = TaggedValue::new("!Inner", "Unit");
        test_serde::<Outer>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_option() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = indoc! {"
            - 1
            - null
            - 3
        "};
        let value = Sequence::from_iter([
            SpannedValue::from(1),
            SpannedValue::from(Value::Null),
            SpannedValue::from(3),
        ]);
        test_serde::<Vec<Option<usize>>>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_unit() -> eyre::Result<()> {
        crate::tests::init();

        // let thing = vec![(), ()];
        let yaml = indoc! {"
            - null
            - null
        "};
        let value = Sequence::from_iter([
            SpannedValue::from(Value::Null),
            SpannedValue::from(Value::Null),
        ]);
        test_serde::<Vec<()>>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_unit_struct() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Foo;
        let yaml = indoc! {"
            null
        "};
        test_serde::<Foo>(&yaml, Value::Null)?;
        Ok(())
    }

    #[test]
    fn test_unit_variant() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Variant {
            First,
            Second,
        }
        let yaml = indoc! {"
            First
        "};
        let value = Value::from("First");
        test_serde::<Variant>(&yaml, value)?;
        Ok(())
    }

    #[test]
    fn test_newtype_struct() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct OriginalType {
            v: u16,
        }
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct NewType(OriginalType);

        // let thing = NewType(OriginalType { v: 1 });
        let yaml = indoc! {"
            v: 1
        "};
        let value = Mapping::from_iter([(SpannedValue::from("v"), SpannedValue::from(1))]);
        test_serde::<NewType>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_newtype_variant() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Variant {
            Size(usize),
        }
        // let thing = Variant::Size(127);
        let yaml = indoc! {"
            !Size 127
        "};
        let value = TaggedValue::new("!Size", 127);
        test_serde::<Variant>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_tuple_variant() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Variant {
            Rgb(u8, u8, u8),
        }
        // let thing = Variant::Rgb(32, 64, 96);
        let yaml = indoc! {"
            !Rgb
            - 32
            - 64
            - 96
        "};
        let value = TaggedValue::new(
            "!Rgb",
            Sequence::from_iter([
                SpannedValue::from(32),
                SpannedValue::from(64),
                SpannedValue::from(96),
            ]),
        );
        test_serde::<Variant>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_struct_variant() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Variant {
            Color { r: u8, g: u8, b: u8 },
        }
        let yaml = indoc! {"
            !Color
            r: 32
            g: 64
            b: 96
        "};
        let value = TaggedValue::new(
            "!Color",
            Mapping::from_iter([
                (SpannedValue::from("r"), SpannedValue::from(32)),
                (SpannedValue::from("g"), SpannedValue::from(64)),
                (SpannedValue::from("b"), SpannedValue::from(96)),
            ]),
        );
        test_serde::<Variant>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_tagged_map_value() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Bindings {
            profile: Profile,
        }
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        enum Profile {
            ClassValidator { class_name: String },
        }
        // let thing = Bindings {
        //     profile: Profile::ClassValidator {
        //         class_name: "ApplicationConfig".to_owned(),
        //     },
        // };
        let yaml = indoc! {"
            profile: !ClassValidator
                class_name: ApplicationConfig
        "};
        let value = Mapping::from_iter([(
            SpannedValue::from("profile"),
            SpannedValue::from(TaggedValue::new(
                "!ClassValidator",
                Mapping::from_iter([(
                    SpannedValue::from("class_name"),
                    SpannedValue::from("ApplicationConfig"),
                )]),
            )),
        )]);
        test_serde::<Bindings>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_value() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        pub struct GenericInstructions {
            #[serde(rename = "type")]
            pub typ: String,
            pub config: Value,
        }
        // let thing = GenericInstructions {
        //     typ: "primary".to_string(),
        //     config: Value::Sequence(vec![
        //         Value::Null,
        //         Value::Bool(true),
        //         Value::Number(Number::from(65535)),
        //         Value::Number(Number::from(0.54321)),
        //         Value::String("s".into()),
        //         Value::Mapping(Mapping::new()),
        //     ]),
        // };
        let yaml = indoc! {"
            type: primary
            config:
            - null
            - true
            - 65535
            - 0.54321
            - s
            - {}
        "};
        let value = Mapping::from_iter([
            (SpannedValue::from("type"), SpannedValue::from("primary")),
            (
                SpannedValue::from("config"),
                SpannedValue::from(Sequence::from_iter([
                    SpannedValue::from(Value::Null),
                    SpannedValue::from(true),
                    SpannedValue::from(65535),
                    SpannedValue::from(0.54321),
                    SpannedValue::from("s"),
                    SpannedValue::from(Mapping::new()),
                ])),
            ),
        ]);
        test_serde::<GenericInstructions>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_mapping() -> eyre::Result<()> {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Data {
            pub substructure: Mapping,
        }

        // let mut thing = Data {
        //     substructure: Mapping::new(),
        // };
        // thing.substructure.insert(
        //     Value::String("a".to_owned()),
        //     Value::String("foo".to_owned()),
        // );
        // thing.substructure.insert(
        //     Value::String("b".to_owned()),
        //     Value::String("bar".to_owned()),
        // );

        let yaml = indoc! {"
            substructure:
              a: foo
              b: bar
        "};

        let value = Mapping::from_iter([(
            SpannedValue::from("substructure"),
            SpannedValue::from(Mapping::from_iter([
                (SpannedValue::from("a"), SpannedValue::from("foo")),
                (SpannedValue::from("b"), SpannedValue::from("bar")),
            ])),
        )]);
        test_serde::<Data>(&yaml, Value::from(value))?;
        Ok(())
    }

    #[test]
    fn test_long_string() -> eyre::Result<()> {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Data {
            pub string: String,
        }

        // let thing = Data {
        //     string: std::iter::repeat(["word", " "])
        //         .flatten()
        //         .take(69)
        //         .collect(),
        // };

        let yaml = indoc! {"
        string: word word word word word word word word word word word word word word word word word word word word word word word word word word word word word word word word word word word
    "};

        let value = Mapping::from_iter([(
            SpannedValue::from("string"),
            SpannedValue::from(
                std::iter::repeat(["word", " "])
                    .flatten()
                    .take(69)
                    .collect::<String>(),
            ),
        )]);
        test_serde::<Data>(&yaml, Value::from(value))?;
        Ok(())
    }
}
