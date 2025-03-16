use crate::Value;

/// A spanned type.
#[derive(Clone, Default, Debug)]
pub struct Spanned<T> {
    pub span: Span,
    pub inner: T,
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Spanned<T> {
    fn deserialize<D>(deserializer: D) -> Result<Spanned<T>, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Spanned::dummy(value))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::de::IntoDeserializer<'de, crate::error::SerdeError> for Spanned<Value> {
    type Deserializer = Value;

    fn into_deserializer(self) -> Self::Deserializer {
        self.inner
    }
}

impl Into<Value> for Spanned<Value> {
    fn into(self: Spanned<Value>) -> Value {
        self.into_inner()
    }
}

impl<T> From<T> for Spanned<T> {
    fn from(value: T) -> Self {
        Spanned::dummy(value)
    }
}

impl<T> std::fmt::Display for Spanned<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

impl<T> Spanned<T> {
    /// Wrap an instance of something with the given span
    pub const fn new(span: Span, inner: T) -> Self {
        Self { span, inner }
    }

    /// Wrap an instance of something with the given span
    pub const fn dummy(inner: T) -> Self {
        Self {
            span: Span::EMPTY,
            inner,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    /// The span associated with this value
    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl<T> AsMut<T> for Spanned<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> AsRef<T> for Spanned<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::borrow::BorrowMut<T> for Spanned<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> std::borrow::Borrow<T> for Spanned<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Ord for Spanned<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.inner, &other.inner)
    }
}

impl<T> PartialOrd for Spanned<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self.inner, &other.inner)
    }
}

impl<T> PartialOrd<T> for Spanned<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(&self.inner, &other)
    }
}

impl<T> PartialEq for Spanned<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> PartialEq<T> for Spanned<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        (&self.inner as &dyn PartialEq<T>).eq(other)
    }
}

impl<T> PartialEq<&T> for Spanned<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &&T) -> bool {
        (&self.inner as &dyn PartialEq<T>).eq(*other)
    }
}

impl<T> Eq for Spanned<T> where T: Eq {}

impl<T> std::hash::Hash for Spanned<T>
where
    T: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl std::borrow::Borrow<str> for Spanned<String> {
    fn borrow(&self) -> &str {
        self.inner.borrow()
    }
}

impl std::borrow::Borrow<str> for Spanned<&'_ str> {
    fn borrow(&self) -> &str {
        self.inner
    }
}

/// A displayable marker for a YAML node
///
/// While `Marker` can be `Display`'d it doesn't understand what its source
/// means.  This struct is the result of asking a Marker to render itself.
pub struct RenderedMarker<D> {
    source: D,
    line: usize,
    column: usize,
}

impl<D> std::fmt::Display for RenderedMarker<D>
where
    D: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.source.fmt(f)?;
        write!(f, ":{}:{}", self.line, self.column)
    }
}

/// A marker for a YAML node.
///
/// Stores the byte index (zero-indexed) of where a node starts or ends.
pub type ByteIndex = usize;

/// A marker for a YAML node.
///
/// Stores the line and column index (zero-indexed) of where a node starts or ends.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Marker {
    // pub source: usize,
    pub byte_index: ByteIndex,
    pub line: usize,   // u32 for performance
    pub column: usize, // u32 for performance
}

impl std::fmt::Display for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// The span for a YAML marked node
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: Option<Marker>,
    pub end: Option<Marker>,
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(start) = self.start {
            write!(f, "L{}:{}", start.line, start.column)?;
        } else {
            write!(f, "?")?;
        }
        write!(f, " - ")?;
        if let Some(end) = self.end {
            write!(f, "L{}:{}", end.line, end.column)
        } else {
            write!(f, "?")
        }
    }
}

impl From<&Span> for std::ops::Range<usize> {
    fn from(value: &Span) -> Self {
        let start = value.start.unwrap_or_default().byte_index;
        let end = value.end.unwrap_or_default().byte_index;
        std::ops::Range { start, end }
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(value: Span) -> Self {
        Self::from(&value)
    }
}

impl Span {
    pub const EMPTY: Span = Span {
        start: None,
        end: None,
    };
}
