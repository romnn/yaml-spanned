#![allow(warnings)]

#[cfg(feature = "serde")]
pub mod de;
pub mod error;
pub mod fmt;
pub mod from;
pub mod index;
pub mod mapping;
pub mod number;
pub mod partial_eq;
#[cfg(feature = "serde")]
pub mod ser;
pub mod spanned;
pub mod tag;
pub mod value;

pub use error::{Error, ParseError};
pub use mapping::Mapping;
pub use number::Number;
#[cfg(feature = "serde")]
pub use ser::value::Serializer;
pub use spanned::Spanned;
pub use tag::{Tag, TaggedValue};
pub use value::{Builder, Sequence, Value};

pub type SpannedValue = Spanned<Value>;

struct LossyDocumentDeserializer<'a> {
    builder: Builder,
    parser: libyaml_safer::Parser<'a>,
}

impl<'a> LossyDocumentDeserializer<'a> {
    pub fn new(value: &'a mut &[u8]) -> Self {
        let mut parser = libyaml_safer::Parser::new();
        parser.set_input_string(value);

        let builder = Builder::default();
        let mut this = Self { parser, builder };
        this
    }
}

impl<'a> std::iter::Iterator for LossyDocumentDeserializer<'a> {
    type Item = Result<(Spanned<Value>, Vec<ParseError>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut errors = vec![];

        match libyaml_safer::Document::load(&mut self.parser) {
            Ok(document) if document.nodes.is_empty() => None,
            Ok(mut document) => Some(
                self.builder
                    .from_document(&mut document, &mut errors)
                    .map(|value| (value, errors))
                    .map_err(Error::from),
            ),
            Err(err) => Some(Err(err.into())),
        }
    }
}

pub fn from_str_lossy_iter<'a>(value: &'a mut &[u8]) -> LossyDocumentDeserializer<'a> {
    LossyDocumentDeserializer::new(value)
}

pub fn from_str_lossy_all(value: &str) -> Result<Vec<(Spanned<Value>, Vec<ParseError>)>, Error> {
    let mut parser = libyaml_safer::Parser::new();
    let mut bytes = value.as_bytes();

    parser.set_input_string(&mut bytes);

    let builder = Builder::default();
    let mut documents = vec![];
    loop {
        let mut errors = vec![];

        let mut document = match libyaml_safer::Document::load(&mut parser) {
            Ok(document) if document.nodes.is_empty() => break,
            Ok(document) => document,
            Err(err) => return Err(err.into()),
        };
        let value: Spanned<Value> = builder.from_document(&mut document, &mut errors)?;
        documents.push((value, errors))
    }

    Ok(documents)
}

pub fn from_str_lossy(value: &str) -> Result<(Spanned<Value>, Vec<ParseError>), Error> {
    let mut parser = libyaml_safer::Parser::new();
    let mut bytes = value.as_bytes();
    parser.set_input_string(&mut bytes);

    let mut document = libyaml_safer::Document::load(&mut parser)?;
    let mut errors = vec![];
    let builder = Builder::default();
    let value: Spanned<Value> = builder.from_document(&mut document, &mut errors)?;

    Ok((value, errors))
}

pub fn from_str(value: &str) -> Result<Spanned<Value>, Error> {
    let (value, errors) = from_str_lossy(value)?;
    if errors.is_empty() {
        Ok(value)
    } else {
        Err(Error::Parse(errors))
    }
}

pub fn from_str_all(value: &str) -> Result<Vec<Spanned<Value>>, Error> {
    let documents = from_str_lossy_all(value)?;
    let (values, errors): (Vec<_>, Vec<_>) = documents.into_iter().unzip();
    let errors: Vec<_> = errors.into_iter().flatten().collect();
    if errors.is_empty() {
        Ok(values)
    } else {
        Err(Error::Parse(errors))
    }
}

/// Interpret a `yaml_spanned::Value` as an instance of type `T`.
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a YAML map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the YAML map or some number is too big to fit in the expected primitive
/// type.
///
/// ```
/// # use yaml_spanned::Value;
/// let val = Value::String("foo".to_owned());
/// let s: String = yaml_spanned::from_value(&val).unwrap();
/// assert_eq!("foo", s);
/// ```
#[cfg(feature = "serde")]
pub fn from_value<T>(value: &Value) -> Result<T, error::SerdeError>
where
    T: serde::de::DeserializeOwned,
{
    serde::Deserialize::deserialize(value)
}

/// Convert a `T` into `yaml_spanned::Value` which is an enum that can represent
/// any valid YAML data.
///
/// This conversion can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
///
/// ```
/// # use serde_yaml::Value;
/// let val = serde_yaml::to_value("s").unwrap();
/// assert_eq!(val, Value::String("s".to_owned()));
/// ```
#[cfg(feature = "serde")]
pub fn to_value<T>(value: T) -> Result<Value, error::SerdeError>
where
    T: serde::Serialize,
{
    value.serialize(Serializer)
}

// use serde::ser::SerializeMap;
//check_for_tag
// use serde::ser::SerializeMap;

// Prevent downstream code from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl Sealed for crate::value::Value {}
    impl Sealed for crate::spanned::Spanned<crate::value::Value> {}
    impl<'a, T> Sealed for &'a T where T: ?Sized + Sealed {}
}

#[cfg(test)]
mod tests {
    static INIT: std::sync::Once = std::sync::Once::new();

    /// Initialize test
    ///
    /// This ensures `color_eyre` is setup once.
    pub fn init() {
        INIT.call_once(|| {
            color_eyre::install().ok();
        });
    }
}
