use crate::error::{
    DuplicateKeyError, ErrorSpan, InvalidKeyError, InvalidNumberErrorWithSpan, LimitExceeded,
    ParseError,
};
use crate::mapping::Mapping;
use crate::number::Number;
use crate::spanned::{Marker, Span, Spanned};
use crate::tag::{Tag, TaggedValue};

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::termcolor::StandardStream;
use codespan_reporting::term::{self, ColorArg};
use indexmap::IndexMap;
use itertools::Itertools;
use libyaml_safer::ScalarStyle;

#[inline]
fn parse_null(scalar: &[u8]) -> Option<()> {
    match scalar {
        b"null" | b"Null" | b"NULL" | b"~" | b"" => Some(()),
        _ => None,
    }
}

#[inline]
fn parse_bool(scalar: &str) -> Option<bool> {
    match scalar {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

pub type Sequence = Vec<Spanned<Value>>;

/// Represents any valid YAML value.
#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    /// Represents a YAML null value.
    Null,
    /// Represents a YAML boolean.
    Bool(bool),
    /// Represents a YAML numerical value, whether integer or floating point.
    Number(Number),
    /// Represents a YAML string.
    String(String),
    /// Represents a YAML sequence in which the elements are `yaml_spanned::SpannedValue`.
    Sequence(Sequence),
    /// Represents a YAML mapping in which the keys and values are both `yaml_spanned::SpannedValue`.
    Mapping(Mapping),
    /// A representation of YAML's `!Tag` syntax, used for enums.
    Tagged(Box<TaggedValue>),
}

/// Represents any valid YAML value kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    /// Represents a YAML null value.
    Null,
    /// Represents a YAML boolean.
    Bool,
    /// Represents a YAML numerical value, whether integer or floating point.
    Number,
    /// Represents a YAML string.
    String,
    /// Represents a YAML sequence in which the elements are `yaml_spanned::SpannedValue`.
    Sequence,
    /// Represents a YAML mapping in which the keys and values are both `yaml_spanned::SpannedValue`.
    Mapping,
    /// A representation of YAML's `!Tag` syntax, used for enums.
    Tagged,
}

/// The default value is `Value::Null`.
///
/// This is useful for handling omitted `Value` fields when deserializing.
///
/// # Examples
///
/// ```
/// use serde::Deserialize;
/// use yaml_spanned::{Value, SpannedValue};
///
/// #[derive(serde::Deserialize)]
/// struct Settings {
///     level: i32,
///     #[serde(default)]
///     extras: Value,
/// }
///
/// # fn main() -> Result<(), yaml_spanned::Error> {
/// let yaml = r#" { "level": 42 } "#;
/// let value: SpannedValue = yaml_spanned::from_str(&yaml)?;
/// let settings: Settings = yaml_spanned::from_value(&value)?;
///
/// assert_eq!(settings.level, 42);
/// assert_eq!(settings.extras, yaml_spanned::Value::Null);
/// #
/// #     Ok(())
/// # }
/// ```
impl Default for Value {
    fn default() -> Value {
        Value::Null
    }
}

impl Value {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Null => Kind::Null,
            Self::Number(_) => Kind::Number,
            Self::String(_) => Kind::String,
            Self::Sequence(_) => Kind::Sequence,
            Self::Mapping(_) => Kind::Mapping,
            Self::Tagged(_) => Kind::Tagged,
            Self::Bool(_) => Kind::Bool,
        }
    }

    pub fn cleared_spans(mut self) -> Self {
        self.clear_spans();
        self
    }

    pub fn clear_spans(&mut self) {
        match self {
            Value::Tagged(v) => v.value.clear_spans(),
            Value::Mapping(mapping) => {
                let old_mapping: Mapping = std::mem::take(mapping);
                *mapping = Mapping::from_iter(
                    old_mapping
                        .into_iter()
                        .map(|(k, v)| (k.into_inner().into(), v.into_inner().into())),
                );
            }
            Value::Sequence(seq) => {
                for v in seq.iter_mut() {
                    v.clear_spans();
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
}

impl Spanned<Value> {
    pub fn cleared_spans(mut self) -> Self {
        self.clear_spans();
        self
    }

    pub fn clear_spans(&mut self) {
        self.span.start = None;
        self.span.end = None;
        self.inner.clear_spans();
    }
}
impl Value {
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    #[inline]
    pub fn is_sequence(&self) -> bool {
        self.as_sequence().is_some()
    }

    #[inline]
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self {
            Self::Sequence(sequence) => Some(sequence),
            _ => None,
        }
    }

    #[inline]
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Self::Sequence(sequence) => Some(sequence),
            _ => None,
        }
    }

    #[inline]
    pub fn is_mapping(&self) -> bool {
        self.as_mapping().is_some()
    }

    #[inline]
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self {
            Self::Mapping(mapping) => Some(mapping),
            _ => None,
        }
    }

    #[inline]
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Self::Mapping(mapping) => Some(mapping),
            _ => None,
        }
    }

    #[inline]
    pub fn get_or_null<I: crate::mapping::Index>(&self, index: I) -> &Spanned<Value> {
        static null: Spanned<Value> = Spanned::dummy(Value::Null);
        self.as_mapping()
            .and_then(|map| map.get(index))
            .unwrap_or(&null)
    }

    #[inline]
    pub fn get<I: crate::mapping::Index>(&self, index: I) -> Option<&Spanned<Value>> {
        self.as_mapping().and_then(|map| map.get(index))
    }

    #[inline]
    pub fn get_mut<I: crate::mapping::Index>(&mut self, index: I) -> Option<&mut Spanned<Value>> {
        self.as_mapping_mut().and_then(|map| map.get_mut(index))
    }

    #[inline]
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    #[inline]
    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            Self::Bool(boolean) => Some(boolean),
            _ => None,
        }
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(boolean) => Some(*boolean),
            _ => None,
        }
    }

    #[inline]
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    #[inline]
    pub fn as_u64_mut(&mut self) -> Option<&mut u64> {
        match self {
            Self::Number(number) => number.as_u64_mut(),
            _ => None,
        }
    }

    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(number) => number.as_u64(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    #[inline]
    pub fn as_i64_mut(&mut self) -> Option<&mut i64> {
        match self {
            Self::Number(number) => number.as_i64_mut(),
            _ => None,
        }
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(number) => number.as_i64(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_f64(&self) -> bool {
        // NOTE: cannot use `as_f64().is_some()`, as_f64 will cast integers to floating points numbers
        match self {
            Self::Number(number) => number.is_f64(),
            _ => false,
        }
    }

    #[inline]
    pub fn as_f64_mut(&mut self) -> Option<&mut f64> {
        match self {
            Self::Number(number) => number.as_f64_mut(),
            _ => None,
        }
    }

    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => number.as_f64(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    #[inline]
    pub fn as_str_mut(&mut self) -> Option<&mut str> {
        match self {
            Self::String(scalar) => Some(scalar.as_mut_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(scalar) => Some(scalar.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn is_string(&self) -> bool {
        self.as_string().is_some()
    }

    #[inline]
    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::String(scalar) => Some(scalar),
            _ => None,
        }
    }

    #[inline]
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Self::String(scalar) => Some(scalar),
            _ => None,
        }
    }

    /// Performs merging of `<<` keys into the surrounding mapping.
    ///
    /// The intended use of this in YAML is described in
    /// <https://yaml.org/type/merge.html>.
    ///
    /// ```
    /// use yaml_spanned::SpannedValue;
    ///
    /// let config = "\
    /// tasks:
    ///   build: &webpack_shared
    ///     command: webpack
    ///     args: build
    ///     inputs:
    ///       - 'src/**/*'
    ///   start:
    ///     <<: *webpack_shared
    ///     args: start
    /// ";
    ///
    /// let mut value: SpannedValue = yaml_spanned::from_str(config).unwrap();
    /// value.apply_merge().unwrap();
    ///
    /// assert_eq!(value["tasks"]["start"]["command"], "webpack");
    /// assert_eq!(value["tasks"]["start"]["args"], "start");
    /// ```
    pub fn apply_merge(&mut self) -> Result<(), crate::error::MergeError> {
        let mut stack = Vec::new();
        stack.push(self);
        while let Some(node) = stack.pop() {
            match node {
                Value::Mapping(mapping) => {
                    match mapping.remove("<<").map(Spanned::into_inner) {
                        Some(Value::Mapping(merge)) => {
                            for (k, v) in merge {
                                mapping.entry(k).or_insert(v);
                            }
                        }
                        Some(Value::Sequence(sequence)) => {
                            for value in sequence {
                                match value.into_inner() {
                                    Value::Mapping(merge) => {
                                        for (k, v) in merge {
                                            mapping.entry(k).or_insert(v);
                                        }
                                    }
                                    Value::Sequence(_) => {
                                        return Err(
                                            crate::error::MergeError::SequenceInMergeElement,
                                        );
                                    }
                                    Value::Tagged(_) => {
                                        return Err(crate::error::MergeError::TaggedInMerge);
                                    }
                                    _unexpected => {
                                        return Err(crate::error::MergeError::ScalarInMergeElement);
                                    }
                                }
                            }
                        }
                        None => {}
                        Some(Value::Tagged(_)) => {
                            return Err(crate::error::MergeError::TaggedInMerge);
                        }
                        Some(_unexpected) => {
                            return Err(crate::error::MergeError::ScalarInMergeElement);
                        }
                    }
                    stack.extend(mapping.values_mut().map(Spanned::as_mut));
                }
                Value::Sequence(sequence) => stack.extend(sequence.iter_mut().map(Spanned::as_mut)),
                Value::Tagged(tagged) => stack.push(&mut tagged.value),
                _ => {}
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "NULL"),
            Self::Bool(boolean) => write!(f, "Bool({boolean})"),
            Self::String(value) => write!(f, "String({value:?})"),
            Self::Number(number) => write!(f, "Number({number:?})"),
            Self::Sequence(sequence) => f.debug_list().entries(sequence).finish(),
            Self::Mapping(mapping) => std::fmt::Debug::fmt(mapping, f),
            Self::Tagged(tagged) => std::fmt::Debug::fmt(tagged, f),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "NULL"),
            Self::Bool(boolean) => write!(f, "Bool({boolean})"),
            Self::String(value) => write!(f, "String({value})"),
            Self::Number(number) => write!(f, "Number({number})"),
            Self::Sequence(sequence) => f
                .debug_list()
                .entries(sequence.iter().map(|item| crate::fmt::Display(item)))
                .finish(),
            Self::Mapping(mapping) => std::fmt::Display::fmt(mapping, f),
            Self::Tagged(tagged) => std::fmt::Display::fmt(tagged, f),
        }
    }
}

pub struct StringValueRepr<'a>(&'a Value);

impl<'a> std::fmt::Debug for StringValueRepr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl<'a> std::fmt::Display for StringValueRepr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Value::Null => write!(f, "NULL"),
            Value::Bool(boolean) => write!(f, "{boolean}"),
            Value::String(value) => write!(f, "{value}"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Sequence(sequence) => f
                .debug_list()
                .entries(sequence.iter().map(|item| item.string_value_repr()))
                .finish(),
            Value::Mapping(mapping) => write!(f, "{}", mapping.string_value_repr()),
            Value::Tagged(tagged) => write!(f, "{}", tagged.value.string_value_repr()),
        }
    }
}

impl Value {
    pub fn string_value_repr(&self) -> StringValueRepr<'_> {
        StringValueRepr(self)
    }
}

impl Eq for Value {}

// NOTE: This impl must be kept consistent with HashLikeValue's Hash impl in
// mapping.rs in order for value[str] indexing to work.
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(v) => v.hash(state),
            Value::Number(v) => v.hash(state),
            Value::String(v) => v.hash(state),
            Value::Sequence(v) => v.hash(state),
            Value::Mapping(v) => v.hash(state),
            Value::Tagged(v) => v.hash(state),
        }
    }
}

impl TryFrom<libyaml_safer::Mark> for crate::spanned::Marker {
    type Error = std::num::TryFromIntError;
    fn try_from(value: libyaml_safer::Mark) -> Result<Self, Self::Error> {
        Ok(Self {
            byte_index: value.index.try_into()?,
            line: value.line.try_into()?,
            column: value.column.try_into()?,
        })
    }
}

impl TryFrom<(libyaml_safer::Mark, libyaml_safer::Mark)> for crate::spanned::Span {
    type Error = std::num::TryFromIntError;
    fn try_from(value: (libyaml_safer::Mark, libyaml_safer::Mark)) -> Result<Self, Self::Error> {
        Ok(Self {
            start: Some(value.0.try_into()?),
            end: Some(value.1.try_into()?),
        })
    }
}

trait ToValue {
    fn to_value<'doc>(
        &self,
        path: &str,
        document: &'doc libyaml_safer::Document,
        errors: &mut Vec<ParseError>,
        recursion_limit: usize,
        jump_limit: &mut usize,
    ) -> Result<Spanned<Value>, LimitExceeded>;
}

#[inline]
fn scalar_node_to_value_guess_type(
    node: &libyaml_safer::Node,
    path: &str,
    value: &str,
    style: &libyaml_safer::ScalarStyle,
    document: &libyaml_safer::Document,
    errors: &mut Vec<ParseError>,
) -> Value {
    match style {
        // ':'  => ScalarStyle::Plain,
        // '\'' => ScalarStyle::SingleQuoted,
        // '"'  => ScalarStyle::DoubleQuoted,
        // '|'  => ScalarStyle::Literal,
        // '>'  => ScalarStyle::Folded,
        libyaml_safer::ScalarStyle::Plain | libyaml_safer::ScalarStyle::Any => {
            if parse_null(value.as_bytes()).is_some() {
                return Value::Null;
            }
            if let Some(boolean) = parse_bool(value) {
                return Value::Bool(boolean);
            }
            if let Some(number) = crate::number::parse_number(value) {
                return Value::Number(number);
            }
            Value::String(value.to_string())
        }
        // libyaml_safer::ScalarStyle::Literal
        // | libyaml_safer::ScalarStyle::Folded
        // | libyaml_safer::ScalarStyle::SingleQuoted
        // | libyaml_safer::ScalarStyle::SingleQuoted
        _ => {
            // is string
            Value::String(value.to_string())
        }
    }
}

trait SpannedNode {
    fn span(&self) -> Span;
}

impl SpannedNode for &libyaml_safer::Node {
    fn span(&self) -> Span {
        Span {
            start: self.start_mark.try_into().ok(),
            end: self.end_mark.try_into().ok(),
        }
    }
}

impl SpannedNode for &libyaml_safer::Document {
    fn span(&self) -> Span {
        Span {
            start: self.start_mark.try_into().ok(),
            end: self.end_mark.try_into().ok(),
        }
    }
}

#[inline]
fn scalar_node_to_value(
    node: &libyaml_safer::Node,
    path: &str,
    value: &str,
    style: &libyaml_safer::ScalarStyle,
    document: &libyaml_safer::Document,
    errors: &mut Vec<ParseError>,
) -> Spanned<Value> {
    match node.tag.as_deref() {
        Some("tag:yaml.org,2002:int") => {
            if let Some(number) = crate::number::parse_number(value) {
                return Spanned::new(node.span(), Value::Number(number));
            }
        }
        _ => {
            // autodetect
        }
    }
    let value = scalar_node_to_value_guess_type(node, path, value, style, document, errors);
    Spanned::new(node.span(), value)
}

#[inline]
fn mapping_node_to_value(
    node: &libyaml_safer::Node,
    path: &str,
    pairs: &[libyaml_safer::NodePair],
    style: &libyaml_safer::MappingStyle,
    document: &libyaml_safer::Document,
    errors: &mut Vec<ParseError>,
    recursion_limit: usize,
    jump_limit: &mut usize,
) -> Result<Spanned<Value>, LimitExceeded> {
    let entries: Vec<_> = pairs
        .iter()
        .map(|pair| {
            // *jump_count += 1;
            if *jump_limit == 0 {
                return Err(LimitExceeded::RepetitionLimitExceeded);
            }
            *jump_limit = jump_limit.saturating_sub(1);
            let key = document.get_node(pair.key).unwrap().to_value(
                path,
                document,
                errors,
                recursion_limit.saturating_sub(1),
                jump_limit,
            )?;

            // *jump_count += 1;
            if *jump_limit == 0 {
                return Err(LimitExceeded::RepetitionLimitExceeded);
            }
            *jump_limit = jump_limit.saturating_sub(1);

            let value = document.get_node(pair.value).unwrap().to_value(
                &format!("{path}.{}", key.as_str().unwrap_or_default()),
                document,
                errors,
                recursion_limit.saturating_sub(1),
                jump_limit,
            )?;
            Ok::<_, LimitExceeded>((key, value))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let duplicate_keys = entries.iter().map(|(key, _)| key.as_ref()).duplicates();
    // .take(1); // only take the first entry?

    for duplicate_key in duplicate_keys {
        let occurrences = entries
            .iter()
            .filter(|(key, _)| key == duplicate_key)
            .map(|(key, _)| key.span().into())
            .collect::<Vec<ErrorSpan>>();
        errors.push(
            DuplicateKeyError {
                key: duplicate_key.string_value_repr().to_string(),
                path: path.to_string(),
                occurrences,
            }
            .into(),
        );
    }
    Ok(Spanned::new(
        node.span(),
        Value::Mapping(Mapping(IndexMap::from_iter(entries.into_iter()))),
    ))
}

#[inline]
fn sequence_node_to_value(
    node: &libyaml_safer::Node,
    path: &str,
    items: &[i32],
    style: &libyaml_safer::SequenceStyle,
    document: &libyaml_safer::Document,
    errors: &mut Vec<ParseError>,
    recursion_limit: usize,
    jump_limit: &mut usize,
) -> Result<Spanned<Value>, LimitExceeded> {
    let sequence = items
        .iter()
        .enumerate()
        .map(|(idx, node_idx)| {
            if *jump_limit == 0 {
                return Err(LimitExceeded::RepetitionLimitExceeded);
            }
            *jump_limit = jump_limit.saturating_sub(1);

            document.get_node(*node_idx).unwrap().to_value(
                &format!("{path}[{idx}]"),
                document,
                errors,
                recursion_limit.saturating_sub(1),
                jump_limit,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Spanned::new(node.span(), Value::Sequence(sequence)))
}

impl ToValue for libyaml_safer::Node {
    fn to_value<'doc>(
        &self,
        path: &str,
        document: &'doc libyaml_safer::Document,
        errors: &mut Vec<ParseError>,
        recursion_limit: usize,
        jump_limit: &mut usize,
    ) -> Result<Spanned<Value>, LimitExceeded> {
        if recursion_limit == 0 {
            return Err(LimitExceeded::RecursionLimitExceeded);
        }

        let value = match &self.data {
            libyaml_safer::NodeData::NoNode => Ok(Spanned::new(self.span(), Value::Null)),
            libyaml_safer::NodeData::Scalar { value, style } => Ok(scalar_node_to_value(
                self, path, value, style, document, errors,
            )),
            libyaml_safer::NodeData::Mapping { pairs, style } => mapping_node_to_value(
                self,
                path,
                pairs,
                style,
                document,
                errors,
                recursion_limit.saturating_sub(1),
                jump_limit,
            ),
            libyaml_safer::NodeData::Sequence { items, style } => sequence_node_to_value(
                self,
                path,
                items,
                style,
                document,
                errors,
                recursion_limit.saturating_sub(1),
                jump_limit,
            ),
        }?;
        // check if value is tagged using a non-YAML tag
        match self.tag.as_deref() {
            // None
            // | Some(
            //     "tag:yaml.org,2002:seq"
            //     | "tag:yaml.org,2002:map"
            //     | "tag:yaml.org,2002:str"
            //     | "tag:yaml.org,2002:int",
            // ) => value,
            None => Ok(value),
            Some(tag) if tag.starts_with("tag:yaml.org,2002:") => Ok(value),
            Some(tag) => Ok(Spanned::new(
                self.span(),
                Value::Tagged(Box::new(TaggedValue {
                    tag: Tag::new(tag.to_string()),
                    value,
                })),
            )),
        }
    }
}

/// Default maximum recursion depth during parsing.
///
/// The default value of 128 is adopted from `serde_yaml`.
pub const DEFAULT_MAX_RECURSION_DEPTH: usize = 128;

#[derive(Debug, Clone, Copy)]
pub struct Builder {
    /// Maximum recursion depth during parsing.
    pub max_recursion_depth: usize,
    /// Maximum jump count during parsing.
    pub jump_limit: Option<usize>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            max_recursion_depth: DEFAULT_MAX_RECURSION_DEPTH,
            jump_limit: None,
        }
    }
}

impl Builder {
    pub fn from_document(
        &self,
        document: &mut libyaml_safer::Document,
        errors: &mut Vec<ParseError>,
    ) -> Result<Spanned<Value>, LimitExceeded> {
        let Some(root_node) = document.nodes.get(0) else {
            return Ok(Spanned::new((&*document).span(), Value::Null));
        };

        let mut jump_limit = self.jump_limit.unwrap_or(document.nodes.len() * 100);
        let value = root_node.to_value(
            "",
            &*document,
            errors,
            self.max_recursion_depth,
            &mut jump_limit,
        )?;
        Ok(value)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::de::IntoDeserializer<'de, crate::error::SerdeError> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mapping, Sequence, Tag, TaggedValue, Value};
    use color_eyre::eyre;
    use indoc::indoc;
    use similar_asserts::assert_eq as sim_assert_eq;

    #[test]
    fn test_empty_string() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            empty:
            tilde: ~
        "};

        let value = crate::from_str(yaml)?;
        sim_assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([
                ("empty".into(), Value::Null.into()),
                ("tilde".into(), Value::Null.into()),
            ]))
        );

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, PartialEq, Debug)]
            struct Struct {
                empty: String,
                tilde: String,
            }

            let expected = Struct {
                empty: String::new(),
                tilde: "~".to_owned(),
            };
            // TODO: cannot deserialize to empty string from NULL
            // similar_asserts::assert_eq!(crate::from_value::<Struct>(value)?, expected);
        }
        Ok(())
    }

    #[test]
    fn test_enum_representations() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            - Unit
            - 'Unit'
            - !Unit
            - !Unit ~
            - !Unit null
            - !Tuple [0, 0]
            - !Tuple
              - 0
              - 0
            - !Struct {x: 0, y: 0}
            - !Struct
              x: 0
              y: 0
            - !String '...'
            - !String ...
            - !Number 0
        "};

        let value = crate::from_str(&yaml)?;
        similar_asserts::assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Sequence::from_iter([
                "Unit".into(),
                "Unit".into(),
                TaggedValue::new("Unit", Value::Null).into(),
                TaggedValue::new("Unit", Value::Null).into(),
                TaggedValue::new("Unit", Value::Null).into(),
                TaggedValue::new("Tuple", Sequence::from_iter([0.into(), 0.into()])).into(),
                TaggedValue::new("Tuple", Sequence::from_iter([0.into(), 0.into()])).into(),
                TaggedValue::new(
                    "Struct",
                    Mapping::from_iter([("x".into(), 0.into()), ("y".into(), 0.into())])
                )
                .into(),
                TaggedValue::new(
                    "Struct",
                    Mapping::from_iter([("x".into(), 0.into()), ("y".into(), 0.into())])
                )
                .into(),
                TaggedValue::new("String", "...").into(),
                TaggedValue::new("String", "...").into(),
                TaggedValue::new("Number", 0).into(),
            ]))
        );

        #[cfg(feature = "serde")]
        #[derive(serde::Deserialize, PartialEq, Debug)]
        enum Enum {
            Unit,
            Tuple(i32, i32),
            Struct { x: i32, y: i32 },
            String(String),
            Number(f64),
        }

        #[cfg(feature = "serde")]
        {
            let expected = vec![
                Enum::Unit,
                Enum::Unit,
                Enum::Unit,
                Enum::Unit,
                Enum::Unit,
                Enum::Tuple(0, 0),
                Enum::Tuple(0, 0),
                Enum::Struct { x: 0, y: 0 },
                Enum::Struct { x: 0, y: 0 },
                Enum::String("...".to_owned()),
                Enum::String("...".to_owned()),
                Enum::Number(0.0),
            ];

            sim_assert_eq!(crate::from_value::<Vec<Enum>>(&value)?, expected);
        }

        let yaml = indoc! {"
            - !String
        "};

        let value = crate::from_str(&yaml)?;
        sim_assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Sequence::from_iter([TaggedValue::new(
                "String",
                Value::Null
            )
            .into()]))
        );

        #[cfg(feature = "serde")]
        {
            let expected = vec![Enum::String(String::new())];
            // TODO: allow parsing empty string from Value::Null
            // sim_assert_eq!(crate::from_value::<Vec<Enum>>(value)?, expected);
        }

        Ok(())
    }

    #[test]
    fn test_enum_alias() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            aref:
              &aref
              A
            bref:
              &bref
              !B
                - 1
                - 2

            a: *aref
            b: *bref
        "};

        let value = crate::from_str(&yaml)?;
        let expected: Value = Mapping::from_iter([
            ("aref".into(), "A".into()),
            (
                "bref".into(),
                TaggedValue::new("B", Sequence::from_iter([1.into(), 2.into()])).into(),
            ),
            ("a".into(), "A".into()),
            (
                "b".into(),
                TaggedValue::new("B", Sequence::from_iter([1.into(), 2.into()])).into(),
            ),
        ])
        .into();
        sim_assert_eq!(value.clone().cleared_spans().into_inner(), expected);

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, PartialEq, Debug)]
            enum E {
                A,
                B(u8, u8),
            }
            #[derive(serde::Deserialize, PartialEq, Debug)]
            struct Data {
                a: E,
                b: E,
            }

            let expected = Data {
                a: E::A,
                b: E::B(1, 2),
            };
            sim_assert_eq!(crate::from_value::<Data>(&value)?, expected);
        }

        Ok(())
    }

    #[test]
    fn test_option_alias() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            none_f:
              &none_f
              ~
            none_s:
              &none_s
              ~
            none_b:
              &none_b
              ~

            some_f:
              &some_f
              1.0
            some_s:
              &some_s
              x
            some_b:
              &some_b
              true

            a: *none_f
            b: *none_s
            c: *none_b
            d: *some_f
            e: *some_s
            f: *some_b
        "};

        let value = crate::from_str(&yaml)?;
        let expected: Value = Mapping::from_iter([
            ("none_f".into(), Value::Null.into()),
            ("none_s".into(), Value::Null.into()),
            ("none_b".into(), Value::Null.into()),
            ("some_f".into(), 1.0.into()),
            ("some_s".into(), "x".into()),
            ("some_b".into(), true.into()),
            ("a".into(), Value::Null.into()),
            ("b".into(), Value::Null.into()),
            ("c".into(), Value::Null.into()),
            ("d".into(), 1.0.into()),
            ("e".into(), "x".into()),
            ("f".into(), true.into()),
        ])
        .into();
        sim_assert_eq!(value.clone().cleared_spans().into_inner(), expected);

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, PartialEq, Debug)]
            struct Data {
                a: Option<f64>,
                b: Option<String>,
                c: Option<bool>,
                d: Option<f64>,
                e: Option<String>,
                f: Option<bool>,
            }
            let expected = Data {
                a: None,
                b: None,
                c: None,
                d: Some(1.0),
                e: Some("x".to_owned()),
                f: Some(true),
            };
            sim_assert_eq!(crate::from_value::<Data>(&value)?, expected);
        }

        Ok(())
    }

    #[test]
    fn test_option() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            b:
            c: true
        "};

        let value = crate::from_str(&yaml)?;
        let expected: Value =
            Mapping::from_iter([("b".into(), Value::Null.into()), ("c".into(), true.into())])
                .into();
        sim_assert_eq!(value.clone().cleared_spans().into_inner(), expected);

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, PartialEq, Debug)]
            struct Data {
                a: Option<f64>,
                b: Option<String>,
                c: Option<bool>,
            }
            let expected = Data {
                a: None,
                b: None,
                c: Some(true),
            };
            sim_assert_eq!(crate::from_value::<Data>(&value)?, expected);
        }
        Ok(())
    }

    #[test]
    fn test_alias() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            first:
              &alias
              1
            second:
              *alias
            third: 3
        "};

        let value = crate::from_str(&yaml)?;
        let expected: Value = Mapping::from_iter([
            ("first".into(), 1.into()),
            ("second".into(), 1.into()),
            ("third".into(), 3.into()),
        ])
        .into();
        sim_assert_eq!(value.clone().cleared_spans().into_inner(), expected);

        #[cfg(feature = "serde")]
        {
            use std::collections::BTreeMap;

            let mut expected = BTreeMap::new();
            expected.insert("first".to_owned(), 1);
            expected.insert("second".to_owned(), 1);
            expected.insert("third".to_owned(), 3);

            sim_assert_eq!(crate::from_value::<BTreeMap<_, _>>(&value)?, expected);
        }

        Ok(())
    }

    #[test]
    fn test_borrowed() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            - plain nonàscii
            - 'single quoted'
            - \"double quoted\"
        "};

        let value = crate::from_str(&yaml)?;
        let expected: Value = Sequence::from_iter([
            "plain nonàscii".into(),
            "single quoted".into(),
            "double quoted".into(),
        ])
        .into();
        sim_assert_eq!(value.clone().cleared_spans().into_inner(), expected);

        #[cfg(feature = "serde")]
        {
            let expected = vec!["plain nonàscii", "single quoted", "double quoted"];
            // TODO: cannot deserialize borrowed as we first allocate value
            // sim_assert_eq!(crate::from_value::<Vec<&str>>(value)?, expected);
        }

        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_into_deserializer() -> eyre::Result<()> {
        crate::tests::init();

        use serde::{Deserialize, de::IntoDeserializer};

        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Test {
            first: String,
            second: u32,
        }

        let value = crate::from_str("xyz")?;
        dbg!(&value);
        let s = String::deserialize(value.into_deserializer())?;
        sim_assert_eq!(s, "xyz");

        let yaml = "- first\n- second\n- third";
        dbg!(&yaml);
        let value = crate::from_str(yaml)?;
        dbg!(&value);
        let arr = Vec::<String>::deserialize(value.into_deserializer())?;
        sim_assert_eq!(arr, &["first", "second", "third"]);

        let value = crate::from_str("first: abc\nsecond: 99")?;
        let test = Test::deserialize(value.into_deserializer())?;

        sim_assert_eq!(
            test,
            Test {
                first: "abc".to_string(),
                second: 99
            }
        );
        Ok(())
    }

    #[test]
    fn test_merge() -> eyre::Result<()> {
        crate::tests::init();

        // From https://yaml.org/type/merge.html.
        let yaml = indoc! {"
            ---
            - &CENTER { x: 1, y: 2 }
            - &LEFT { x: 0, y: 2 }
            - &BIG { r: 10 }
            - &SMALL { r: 1 }

            # All the following maps are equal:

            - # Explicit keys
              x: 1
              y: 2
              r: 10
              label: center/big

            - # Merge one map
              << : *CENTER
              r: 10
              label: center/big

            - # Merge multiple maps
              << : [ *CENTER, *BIG ]
              label: center/big

            - # Override
              << : [ *BIG, *LEFT, *SMALL ]
              x: 1
              label: center/big
        "};

        let mut value = crate::from_str(yaml)?;
        value.apply_merge()?;
        for i in 5..=7 {
            sim_assert_eq!(value[4], value[i]);
        }
        Ok(())
    }

    #[test]
    fn test_display() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            'Null': ~
            Bool: true
            Number: 1
            String: ...
            Sequence:
              - true
            EmptySequence: []
            EmptyMapping: {}
            Tagged: !tag true
        "};

        let value = crate::from_str(yaml)?;
        let display = format!("{value}");
        println!("{display}");

        let expected = indoc! {r#"
            Mapping {
                "Null": Null,
                "Bool": Bool(true),
                "Number": Number(1),
                "String": String("..."),
                "Sequence": Sequence [
                    Bool(true),
                ],
                "EmptySequence": Sequence [],
                "EmptyMapping": Mapping {},
                "Tagged": TaggedValue {
                    tag: !tag,
                    value: Bool(true),
                },
            }"#
        };
        let expected = r#"{"Null": NULL, "Bool": Bool(true), "Number": Number(1), "String": String(...), "Sequence": [Bool(true)], "EmptySequence": [], "EmptyMapping": {}, "Tagged": Bool(true)}"#;
        sim_assert_eq!(display, expected);
        Ok(())
    }

    #[test]
    fn test_debug() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            'Null': ~
            Bool: true
            Number: 1
            String: ...
            Sequence:
              - true
            EmptySequence: []
            EmptyMapping: {}
            Tagged: !tag true
        "};

        let value = crate::from_str(yaml)?;
        let debug = format!("{value:#?}");
        println!("{debug}");

        let expected = indoc! {r#"
            Spanned {
                span: L0:0 - L9:0,
                inner: Mapping(
                    {
                        Spanned {
                            span: L0:0 - L0:6,
                            inner: String("Null"),
                        }: Spanned {
                            span: L0:8 - L0:9,
                            inner: NULL,
                        },
                        Spanned {
                            span: L1:0 - L1:4,
                            inner: String("Bool"),
                        }: Spanned {
                            span: L1:6 - L1:10,
                            inner: Bool(true),
                        },
                        Spanned {
                            span: L2:0 - L2:6,
                            inner: String("Number"),
                        }: Spanned {
                            span: L2:8 - L2:9,
                            inner: Number(UnsignedInt(1)),
                        },
                        Spanned {
                            span: L3:0 - L3:6,
                            inner: String("String"),
                        }: Spanned {
                            span: L3:8 - L3:11,
                            inner: String("..."),
                        },
                        Spanned {
                            span: L4:0 - L4:8,
                            inner: String("Sequence"),
                        }: Spanned {
                            span: L5:2 - L6:0,
                            inner: [
                                Spanned {
                                    span: L5:4 - L5:8,
                                    inner: Bool(true),
                                },
                            ],
                        },
                        Spanned {
                            span: L6:0 - L6:13,
                            inner: String("EmptySequence"),
                        }: Spanned {
                            span: L6:15 - L6:17,
                            inner: [],
                        },
                        Spanned {
                            span: L7:0 - L7:12,
                            inner: String("EmptyMapping"),
                        }: Spanned {
                            span: L7:14 - L7:16,
                            inner: Mapping(
                                {},
                            ),
                        },
                        Spanned {
                            span: L8:0 - L8:6,
                            inner: String("Tagged"),
                        }: Spanned {
                            span: L8:8 - L8:17,
                            inner: TaggedValue {
                                tag: !tag,
                                value: Spanned {
                                    span: L8:8 - L8:17,
                                    inner: Bool(true),
                                },
                            },
                        },
                    },
                ),
            }"#
        };
        sim_assert_eq!(debug, expected);
        Ok(())
    }

    #[ignore = "serialization of struct to value not supported"]
    #[test]
    fn test_tagged() -> eyre::Result<()> {
        crate::tests::init();

        // #[derive(serde::Serialize)]
        // enum Enum {
        //     Variant(usize),
        // }
        //
        // let value = crate::to_value(&Enum::Variant(0))?;
        //
        // let deserialized: Value = serde_yaml::from_value(value.clone())?;
        // sim_assert_eq!(value, deserialized);
        //
        // let serialized = crate::to_value(&value)?;
        // sim_assert_eq!(value, serialized);
        Ok(())
    }

    #[test]
    fn test_two_documents() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            0
            ---
            1
        "};

        let values: Vec<Value> = crate::from_str_all(&yaml)?
            .into_iter()
            .map(|doc| doc.cleared_spans().into_inner())
            .collect();
        let expected: Vec<Value> = vec![0.into(), 1.into()];
        sim_assert_eq!(values, expected);
        Ok(())
    }
}
