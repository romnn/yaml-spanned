use crate::value::Value;
use codespan_reporting::diagnostic::{Diagnostic, Label};

pub type ErrorSpan = std::ops::Range<usize>;

pub trait ToDiagnostics {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>>;
}

#[derive(thiserror::Error, Debug)]
#[error("invalid key {path:?}, expected string but got {value:?}")]
pub struct InvalidKeyError {
    pub value: Value,
    pub span: ErrorSpan,
    pub path: String,
}

impl ToDiagnostics for InvalidKeyError {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>> {
        vec![
            Diagnostic::error()
                // .with_code("E0308")
                .with_message("invalid key")
                .with_labels(vec![
                    Label::primary(file_id, self.span.clone())
                        .with_message(format!("Expected `String`, found: `{:?}`", self.value)),
                ]),
        ]
    }
}

#[derive(thiserror::Error, Debug)]
#[error("duplicate key `{path}.{key}`")]
pub struct DuplicateKeyError {
    pub key: String,
    pub path: String,
    pub occurrences: Vec<ErrorSpan>,
}

impl ToDiagnostics for DuplicateKeyError {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>> {
        assert!(
            self.occurrences.len() >= 2,
            "duplicated key must have at least two occurrences"
        );

        let span = &self.occurrences[self.occurrences.len() - 2];
        let secondary_label = Label::secondary(file_id, span.clone()).with_message(format!(
            "first use of key {}.{}.{}",
            self.path,
            self.key,
            if self.occurrences.len() > 2 {
                format!(" (duplicated {} more time)", self.occurrences.len() - 2)
            } else {
                String::new()
            },
        ));

        let span = &self.occurrences[self.occurrences.len() - 2];
        let primary_label =
            Label::primary(file_id, span.clone()).with_message("cannot set the same key twice");

        vec![
            Diagnostic::error()
                // .with_code("E0384")
                .with_message(format!("duplicate key `{}.{}`", self.path, self.key))
                .with_labels(vec![secondary_label, primary_label]),
        ]
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LimitExceeded {
    #[error("recursion limit exceeded")]
    RecursionLimitExceeded,

    #[error("repetition limit exceeded")]
    RepetitionLimitExceeded,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    InvalidKey(#[from] InvalidKeyError),
    #[error(transparent)]
    DuplicateKey(#[from] DuplicateKeyError),
}

impl ToDiagnostics for ParseError {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>> {
        match self {
            Self::InvalidKey(err) => err.to_diagnostics(file_id),
            Self::DuplicateKey(err) => err.to_diagnostics(file_id),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    YAML(#[from] libyaml_safer::Error),

    #[cfg(feature = "serde")]
    #[error(transparent)]
    Serde(#[from] crate::error::SerdeError),

    #[error(transparent)]
    LimitExceeded(#[from] LimitExceeded),

    #[error("parse error: {0:?}")]
    Parse(Vec<ParseError>),
}

impl ToDiagnostics for Error {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>> {
        match self {
            Self::YAML(err) => err.to_diagnostics(file_id),
            Self::LimitExceeded(_) => vec![],
            #[cfg(feature = "serde")]
            Self::Serde(_) => vec![],
            Self::Parse(errs) => errs
                .iter()
                .flat_map(|err| err.to_diagnostics(file_id))
                .collect(),
        }
    }
}

// impl Error {
//     // pub fn into_iter(self) -> impl Iterator<Item = dyn std::error::Error> {
//     pub fn iter_boxed(self) -> Box<dyn Iterator<Item = Box<dyn std::error::Error>>> {
//         match self {
//             Self::YAML(err) => Box::new([Box::<dyn std::error::Error>::from(err)].into_iter()),
//             Self::RecursionLimit(err) => {
//                 Box::new([Box::<dyn std::error::Error>::from(err)].into_iter())
//             }
//             Self::Parse(errors) => Box::new(
//                 errors
//                     .into_iter()
//                     .map(|err| Box::<dyn std::error::Error>::from(err)),
//             ),
//         }
//     }
//
//     pub fn iter_boxed(self) -> Box<dyn Iterator<Item = Box<dyn std::error::Error>>> {
//         match self {
//             Self::YAML(err) => Box::new([Box::<dyn std::error::Error>::from(err)].into_iter()),
//             Self::RecursionLimit(err) => {
//                 Box::new([Box::<dyn std::error::Error>::from(err)].into_iter())
//             }
//             Self::Parse(errors) => Box::new(
//                 errors
//                     .into_iter()
//                     .map(|err| Box::<dyn std::error::Error>::from(err)),
//             ),
//         }
//     }
// }

// impl IntoIterator for Error {
//     type IntoIter = dyn Iterator<Item = Self::Item>;
//     type Item = dyn std::error::Error;
//
//     fn into_iter(self) -> Self::IntoIter {}
// }

impl ToDiagnostics for libyaml_safer::Error {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>> {
        let mut labels = vec![];

        if let (Some(context), Some(index)) = (
            self.context(),
            self.context_mark()
                .and_then(|mark| mark.index.try_into().ok()),
        ) {
            labels.push(Label::secondary(file_id, index..index).with_message(context));
        }
        if let Some(index) = self
            .problem_mark()
            .and_then(|mark| mark.index.try_into().ok())
        {
            labels.push(Label::primary(file_id, index..index).with_message(self.problem()));
        }

        vec![
            Diagnostic::error()
                .with_message(self.problem())
                .with_labels(labels),
        ]
    }
}

#[cfg(feature = "serde")]
#[derive(thiserror::Error, Debug)]
pub enum SerdeError {
    #[error("{0}")]
    Custom(String, Option<()>),
    #[error("invalid number {0}")]
    InvalidNumber(InvalidNumberError),
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("empty tag")]
    EmptyTag,
}

#[cfg(feature = "serde")]
impl serde::ser::Error for SerdeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string(), None)
    }
}

#[cfg(feature = "serde")]
impl serde::de::Error for SerdeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string(), None)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum InvalidNumberError {
    #[error("unknown number format {0:?}")]
    UnknownFormat(String),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
}

#[derive(thiserror::Error, Debug)]
pub enum MergeError {
    #[error("cannot merge sequence in element")]
    SequenceInMergeElement,
    #[error("cannot merge taggged value")]
    TaggedInMerge,
    #[error("cannot merge scalar in element")]
    ScalarInMergeElement,
}

#[derive(thiserror::Error, Debug)]
pub struct InvalidNumberErrorWithSpan {
    pub path: String,
    pub span: ErrorSpan,
    #[source]
    pub source: InvalidNumberError,
}

impl std::fmt::Display for InvalidNumberErrorWithSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.source, f)
    }
}

impl ToDiagnostics for InvalidNumberErrorWithSpan {
    fn to_diagnostics<F: Copy + PartialEq>(&self, file_id: F) -> Vec<Diagnostic<F>> {
        let labels = vec![
            Label::primary(file_id, self.span.clone())
                .with_message(format!("failed to parse: {:?}", self.source)),
        ];
        vec![
            Diagnostic::error()
                // .with_code("E0308")
                .with_message("invalid number")
                .with_labels(labels),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mapping, Value};
    use color_eyre::eyre;
    use indoc::indoc;
    use similar_asserts::assert_eq as sim_assert_eq;

    #[test]
    fn test_incorrect_type() -> eyre::Result<()> {
        crate::tests::init();
        let yaml = indoc! {"
            ---
            str
        "};
        let value = crate::from_str(yaml)?;
        let expected = r#"invalid type: string "str", expected i16"#; // at line 2 column 1";

        #[cfg(feature = "serde")]
        {
            sim_assert_eq!(
                have: crate::from_value::<i16>(&value).unwrap_err().to_string(),
                want: expected
            );
        }
        Ok(())
    }

    #[test]
    fn test_incorrect_nested_type() -> eyre::Result<()> {
        crate::tests::init();

        // spellcheck:ignore-block
        let yaml = indoc! {"
            b:
              - !C
                d: fase
        "};

        let value = crate::from_str(yaml)?;

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, Debug)]
            pub struct A {
                #[allow(dead_code)]
                pub b: Vec<B>,
            }
            #[derive(serde::Deserialize, Debug)]
            pub enum B {
                C(#[allow(dead_code)] C),
            }
            #[derive(serde::Deserialize, Debug)]
            pub struct C {
                #[allow(dead_code)]
                pub d: bool,
            }

            // spellcheck:ignore-next-line
            let expected = r#"invalid type: string "fase", expected a boolean"#;

            sim_assert_eq!(
                crate::from_value::<A>(&value).unwrap_err().to_string(),
                expected
            );
            sim_assert_eq!(
                have: serde_yaml::from_value::<A>(serde_yaml::from_str(yaml)?)
                    .unwrap_err()
                    .to_string(),
                want: expected
            );
        }
        Ok(())
    }

    #[ignore = "empty files (EOF) are parsed as NULL"]
    #[test]
    fn test_empty() -> eyre::Result<()> {
        crate::tests::init();
        let expected = "EOF while parsing a value";
        sim_assert_eq!(have: crate::from_str("").unwrap_err().to_string(), want: expected);
        Ok(())
    }

    #[test]
    fn test_missing_field() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            v: true
        "};

        let value = crate::from_str(yaml)?;

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, Debug)]
            pub struct Basic {
                #[allow(dead_code)]
                pub v: bool,
                #[allow(dead_code)]
                pub w: bool,
            }

            let expected = r"missing field `w`";
            sim_assert_eq!(
                have: crate::from_value::<Basic>(&value).unwrap_err().to_string(),
                want: expected
            );
        }
        Ok(())
    }

    #[test]
    fn test_unknown_anchor() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            *some
        "};

        let expected = "Composer error: line 2 column 1: found undefined alias";
        sim_assert_eq!(have: crate::from_str(yaml).unwrap_err().to_string(), want: expected);
        Ok(())
    }

    #[test]
    fn test_ignored_unknown_anchor() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            b: [*a]
            c: ~
        "};
        dbg!(&yaml);

        let expected = "Composer error: line 1 column 5: found undefined alias";
        sim_assert_eq!(have: crate::from_str(yaml).unwrap_err().to_string(), want: expected);
        Ok(())
    }

    #[test]
    fn test_bytes() -> eyre::Result<()> {
        crate::tests::init();

        let expected = "Parser error: line 1 column 1: did not find expected node content while parsing a block node (line 1 column 1)";
        sim_assert_eq!(have: crate::from_str("...").unwrap_err().to_string(), want: expected);

        // sim_assert_eq!(
        //     crate::from_value::<Vec<u8>>(&value)
        //         .unwrap_err()
        //         .to_string(),
        //     expected
        // );
        Ok(())
    }

    #[test]
    fn test_second_document_syntax_error() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            0
            ---
            ]
        "};

        let mut test = yaml.as_bytes();
        let mut documents = crate::from_str_lossy_iter(&mut test);

        // first document
        let (value, _errors) = documents.next().unwrap().unwrap();
        let expected: Value = 0.into();
        sim_assert_eq!(value.cleared_spans().into_inner(), expected);

        // second document
        let second_document = documents.next().unwrap();
        // let expected = "did not find expected node content at line 4 column 1, while parsing a block node";
        let expected = r"Parser error: line 4 column 1: did not find expected node content while parsing a block node (line 4 column 1)";
        sim_assert_eq!(have: second_document.unwrap_err().to_string(), want: expected);

        Ok(())
    }

    #[test]
    fn test_missing_enum_tag() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {r#"
            "V": 16
            "other": 32
        "#};

        let value = crate::from_str(yaml)?;
        sim_assert_eq!(
            value.clone().cleared_spans().into_inner(),
            Value::from(Mapping::from_iter([
                ("V".into(), 16.into()),
                ("other".into(), 32.into()),
            ]))
        );

        #[cfg(feature = "serde")]
        {
            #[derive(serde::Deserialize, Debug)]
            pub enum E {
                V(#[allow(dead_code)] usize),
            }
            // let expected = "invalid type: map, expected a YAML tag starting with '!'";
            let expected = "invalid type: map, expected a Value::Tagged enum";
            sim_assert_eq!(
                crate::from_value::<E>(&value).unwrap_err().to_string(),
                expected,
            );
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_deserialize_nested_enum() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Deserialize, Debug)]
        pub enum Outer {
            Inner(#[allow(dead_code)] Inner),
        }
        #[derive(serde::Deserialize, Debug)]
        pub enum Inner {
            Variant(#[allow(dead_code)] Vec<usize>),
        }

        let yaml = indoc! {"
            ---
            !Inner []
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<Outer>(&value).unwrap_err();
        // let expected = "deserializing nested enum in Outer::Inner from YAML is not supported yet at line 2 column 1";
        let expected = "invalid type: sequence, expected a Value::Tagged enum";
        sim_assert_eq!(error.to_string(), expected);

        let yaml = indoc! {"
            ---
            !Variant []
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<Outer>(&value).unwrap_err();
        let expected = "unknown variant `Variant`, expected `Inner`";
        sim_assert_eq!(error.to_string(), expected);

        let yaml = indoc! {"
            ---
            !Inner !Variant []
        "};
        // let expected = "deserializing nested enum in Outer::Inner from YAML is not supported yet at line 2 column 1";
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<Outer>(&value).unwrap_err();
        let expected = "invalid type: unit value, expected a Value::Tagged enum";
        sim_assert_eq!(error.to_string(), expected);

        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_variant_not_a_seq() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Deserialize, Debug)]
        pub enum E {
            V(#[allow(dead_code)] usize),
        }
        let yaml = indoc! {"
            ---
            !V
            value: 0
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<E>(&value).unwrap_err();
        let expected = "invalid type: map, expected usize";
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_struct_from_sequence() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Deserialize, Debug)]
        pub struct Struct {
            #[allow(dead_code)]
            pub x: usize,
            #[allow(dead_code)]
            pub y: usize,
        }
        let yaml = indoc! {"
            [0, 0]
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<Struct>(&value).unwrap_err();
        let expected = "invalid type: sequence, expected struct Struct";
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_bad_bool() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            !!bool str
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<bool>(&value).unwrap_err();
        let expected = r#"invalid type: string "str", expected a boolean"#;
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_bad_int() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            !!int str
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<i64>(&value).unwrap_err();
        let expected = r#"invalid type: string "str", expected i64"#;
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_bad_float() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            !!float str
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<f64>(&value).unwrap_err();
        let expected = r#"invalid type: string "str", expected f64"#;
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_bad_null() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            !!null str
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<()>(&value).unwrap_err();
        let expected = r#"invalid type: string "str", expected unit"#;
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_short_tuple() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            [0, 0]
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<(u8, u8, u8)>(&value).unwrap_err();
        let expected = "invalid length 2, expected a tuple of size 3";
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_long_tuple() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            [0, 0, 0]
        "};
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<(u8, u8)>(&value).unwrap_err();
        let expected = "invalid length 3, expected fewer elements in sequence";
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_invalid_scalar_type() -> eyre::Result<()> {
        crate::tests::init();

        #[derive(serde::Deserialize, Debug)]
        pub struct S {
            #[allow(dead_code)]
            pub x: [i32; 1],
        }

        let yaml = "x: ''\n";
        let value = crate::from_str(yaml)?;
        let error = crate::from_value::<S>(&value).unwrap_err();
        let expected = r#"invalid type: string "", expected an array of length 1"#;
        sim_assert_eq!(error.to_string(), expected);
        Ok(())
    }

    // #[cfg(feature = "serde")]
    // #[cfg(not(miri))]
    #[test]
    fn test_infinite_recursion_objects() -> eyre::Result<()> {
        crate::tests::init();

        // #[derive(serde::Deserialize, Debug)]
        // pub struct S {
        //     #[allow(dead_code)]
        //     pub x: Option<Box<S>>,
        // }

        let yaml = "&a {'x': *a}";
        let error = crate::from_str(yaml).unwrap_err();
        let expected = "recursion limit exceeded";
        sim_assert_eq!(error.to_string(), expected);
        // test_error::<S>(yaml, expected);
        Ok(())
    }

    // #[cfg(feature = "serde")]
    // #[cfg(not(miri))]
    #[test]
    fn test_infinite_recursion_arrays() -> eyre::Result<()> {
        crate::tests::init();

        // #[derive(serde::Deserialize, Debug)]
        // pub struct S(
        //     #[allow(dead_code)] pub usize,
        //     #[allow(dead_code)] pub Option<Box<S>>,
        // );

        let yaml = "&a [0, *a]";
        let error = crate::from_str(yaml).unwrap_err();
        let expected = "recursion limit exceeded";
        sim_assert_eq!(error.to_string(), expected);
        // test_error::<S>(yaml, expected);
        Ok(())
    }

    // #[cfg(feature = "serde")]
    // #[cfg(not(miri))]
    #[test]
    fn test_infinite_recursion_newtype() -> eyre::Result<()> {
        crate::tests::init();

        // #[derive(serde::Deserialize, Debug)]
        // pub struct S(#[allow(dead_code)] pub Option<Box<S>>);

        let yaml = "&a [*a]";
        let error = crate::from_str(yaml).unwrap_err();
        let expected = "recursion limit exceeded";
        sim_assert_eq!(error.to_string(), expected);
        // test_error::<S>(yaml, expected);
        Ok(())
    }

    // #[cfg(feature = "serde")]
    // #[cfg(not(miri))]
    #[test]
    fn test_finite_recursion_objects() -> eyre::Result<()> {
        crate::tests::init();

        // #[derive(serde::Deserialize, Debug)]
        // pub struct S {
        //     #[allow(dead_code)]
        //     pub x: Option<Box<S>>,
        // }

        let yaml = "{'x':".repeat(1_000) + &"}".repeat(1_000);
        let error = crate::from_str(&yaml).unwrap_err();
        let expected = "recursion limit exceeded";
        sim_assert_eq!(error.to_string(), expected);
        // test_error::<S>(&yaml, expected);
        Ok(())
    }

    // #[cfg(feature = "serde")]
    // #[cfg(not(miri))]
    #[test]
    fn test_finite_recursion_arrays() -> eyre::Result<()> {
        crate::tests::init();

        // #[derive(serde::Deserialize, Debug)]
        // pub struct S(
        //     #[allow(dead_code)] pub usize,
        //     #[allow(dead_code)] pub Option<Box<S>>,
        // );

        let yaml = "[0, ".repeat(1_000) + &"]".repeat(1_000);
        let error = crate::from_str(&yaml).unwrap_err();
        let expected = "recursion limit exceeded";
        sim_assert_eq!(error.to_string(), expected);
        // test_error::<S>(&yaml, expected);
        Ok(())
    }

    // #[cfg(not(miri))]
    #[test]
    fn test_billion_laughs() {
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
        "};
        let error = crate::from_str(yaml).unwrap_err();
        let expected = "repetition limit exceeded";
        sim_assert_eq!(error.to_string(), expected);

        // #[cfg(feature = "serde")]
        // {
        //     #[derive(Debug)]
        //     struct X;
        //
        //     impl<'de> serde::de::Visitor<'de> for X {
        //         type Value = X;
        //
        //         fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        //             formatter.write_str("exponential blowup")
        //         }
        //
        //         fn visit_unit<E>(self) -> Result<X, E> {
        //             Ok(X)
        //         }
        //
        //         fn visit_seq<S>(self, mut seq: S) -> Result<X, S::Error>
        //         where
        //             S: serde::de::SeqAccess<'de>,
        //         {
        //             while let Some(X) = seq.next_element()? {}
        //             Ok(X)
        //         }
        //     }
        //
        //     impl<'de> serde::de::Deserialize<'de> for X {
        //         fn deserialize<D>(deserializer: D) -> Result<X, D::Error>
        //         where
        //             D: serde::Deserializer<'de>,
        //         {
        //             deserializer.deserialize_any(X)
        //         }
        //     }
        //
        //     test_error::<BTreeMap<String, X>>(yaml, expected);
        // }
    }

    impl crate::error::Error {
        #[must_use]
        pub fn errors(&self) -> Vec<String> {
            match self {
                Self::YAML(err) => vec![err.to_string()],
                #[cfg(feature = "serde")]
                Self::Serde(err) => vec![err.to_string()],
                Self::LimitExceeded(err) => vec![err.to_string()],
                Self::Parse(errors) => errors
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            }
        }
    }

    #[test]
    fn test_duplicate_keys() -> eyre::Result<()> {
        crate::tests::init();

        let yaml = indoc! {"
            ---
            thing: true
            thing: false
        "};
        let errors = crate::from_str(yaml).unwrap_err().errors();
        let expected = r"duplicate key `.thing`";
        sim_assert_eq!(errors, vec![expected]);

        let yaml = indoc! {"
            ---
            null: true
            ~: false
        "};
        let errors = crate::from_str(yaml).unwrap_err().errors();
        let expected = "duplicate key `.NULL`";
        sim_assert_eq!(errors, vec![expected]);

        let yaml = indoc! {"
            ---
            99: true
            99: false
        "};
        let errors = crate::from_str(yaml).unwrap_err().errors();
        let expected = "duplicate key `.99`";
        sim_assert_eq!(errors, vec![expected]);

        let yaml = indoc! {"
            ---
            {}: true
            {}: false
        "};
        let errors = crate::from_str(yaml).unwrap_err().errors();
        let expected = "duplicate key `.{}`";
        sim_assert_eq!(errors, vec![expected]);
        Ok(())
    }
}
