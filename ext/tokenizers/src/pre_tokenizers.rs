use std::sync::{Arc, RwLock};

use magnus::typed_data::DataTypeBuilder;
use magnus::{
    function, memoize, method, Class, DataType, DataTypeFunctions, Module, Object,
    RArray, RClass, RModule, TypedData,
};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use tk::pre_tokenizers::bert::BertPreTokenizer;
use tk::pre_tokenizers::byte_level::ByteLevel;
use tk::pre_tokenizers::delimiter::CharDelimiterSplit;
use tk::pre_tokenizers::digits::Digits;
use tk::pre_tokenizers::metaspace::Metaspace;
use tk::pre_tokenizers::punctuation::Punctuation;
use tk::pre_tokenizers::split::Split;
use tk::pre_tokenizers::unicode_scripts::UnicodeScripts;
use tk::pre_tokenizers::whitespace::{Whitespace, WhitespaceSplit};
use tk::pre_tokenizers::PreTokenizerWrapper;
use tk::tokenizer::Offsets;
use tk::{PreTokenizedString, PreTokenizer};

use super::utils::*;
use super::{RbError, RbResult};

#[derive(DataTypeFunctions, Clone, Serialize, Deserialize)]
pub struct RbPreTokenizer {
    #[serde(flatten)]
    pub(crate) pretok: RbPreTokenizerTypeWrapper,
}

impl RbPreTokenizer {
    fn pre_tokenize_str(&self, s: String) -> RbResult<Vec<(String, Offsets)>> {
        let mut pretokenized = tk::tokenizer::PreTokenizedString::from(s);

        self.pretok.pre_tokenize(&mut pretokenized).map_err(RbError::from)?;

        Ok(pretokenized
            .get_splits(tk::OffsetReferential::Original, tk::OffsetType::Char)
            .into_iter()
            .map(|(s, o, _)| (s.to_owned(), o))
            .collect())
    }
}

macro_rules! getter {
    ($self: ident, $variant: ident, $($name: tt)+) => {{
        if let RbPreTokenizerTypeWrapper::Single(ref single) = &$self.pretok {
            if let RbPreTokenizerWrapper::Wrapped(PreTokenizerWrapper::$variant(ref pretok)) =
                *single.read().unwrap() {
                    pretok.$($name)+
                } else {
                    unreachable!()
                }
        } else {
            unreachable!()
        }
    }};
}

macro_rules! setter {
    ($self: ident, $variant: ident, $name: ident, $value: expr) => {{
        if let RbPreTokenizerTypeWrapper::Single(ref single) = &$self.pretok {
            if let RbPreTokenizerWrapper::Wrapped(PreTokenizerWrapper::$variant(ref mut pretok)) =
                *single.write().unwrap()
            {
                pretok.$name = $value;
            }
        }
    }};
    ($self: ident, $variant: ident, @$name: ident, $value: expr) => {{
        if let RbPreTokenizerTypeWrapper::Single(ref single) = &$self.pretok {
            if let RbPreTokenizerWrapper::Wrapped(PreTokenizerWrapper::$variant(ref mut pretok)) =
                *single.write().unwrap()
            {
                pretok.$name($value);
            }
        }
    }};
}

impl RbPreTokenizer {
    #[allow(dead_code)]
    pub(crate) fn new(pretok: RbPreTokenizerTypeWrapper) -> Self {
        RbPreTokenizer { pretok }
    }

    fn byte_level_add_prefix_space(&self) -> bool {
        getter!(self, ByteLevel, add_prefix_space)
    }

    fn byte_level_set_add_prefix_space(&self, add_prefix_space: bool) {
        setter!(self, ByteLevel, add_prefix_space, add_prefix_space);
    }

    fn byte_level_use_regex(&self) -> bool {
        getter!(self, ByteLevel, use_regex)
    }

    fn byte_level_set_use_regex(&self, use_regex: bool) {
        setter!(self, ByteLevel, use_regex, use_regex);
    }

    fn char_delimiter_split_delimiter(&self) -> String {
        getter!(self, Delimiter, delimiter.to_string())
    }

    fn char_delimiter_split_set_delimiter(&self, delimiter: char) {
        setter!(self, Delimiter, delimiter, delimiter);
    }

    fn digits_individual_digits(&self) -> bool {
        getter!(self, Digits, individual_digits)
    }

    fn digits_set_individual_digits(&self, individual_digits: bool) {
        setter!(self, Digits, individual_digits, individual_digits);
    }

    fn metaspace_add_prefix_space(&self) -> bool {
        getter!(self, Metaspace, add_prefix_space)
    }

    fn metaspace_set_add_prefix_space(&self, add_prefix_space: bool) {
        setter!(self, Metaspace, add_prefix_space, add_prefix_space);
    }

    fn metaspace_replacement(&self) -> String {
        getter!(self, Metaspace, get_replacement().to_string())
    }

    fn metaspace_set_replacement(&self, replacement: char) {
        setter!(self, Metaspace, @set_replacement, replacement);
    }
}

impl PreTokenizer for RbPreTokenizer {
    fn pre_tokenize(&self, normalized: &mut PreTokenizedString) -> tk::Result<()> {
        self.pretok.pre_tokenize(normalized)
    }
}

pub struct RbByteLevel {}

impl RbByteLevel {
    pub fn new(add_prefix_space: bool, use_regex: bool) -> RbPreTokenizer {
        ByteLevel::default()
            .add_prefix_space(add_prefix_space)
            .use_regex(use_regex)
            .into()
    }

    fn alphabet() -> Vec<String> {
        ByteLevel::alphabet()
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }
}

pub struct RbCharDelimiterSplit {}

impl RbCharDelimiterSplit {
    pub fn new(delimiter: char) -> RbPreTokenizer {
        CharDelimiterSplit::new(delimiter).into()
    }
}

pub struct RbDigits {}

impl RbDigits {
    fn new(individual_digits: bool) -> RbPreTokenizer {
        Digits::new(individual_digits).into()
    }
}

pub struct RbMetaspace {}

impl RbMetaspace {
    fn new(
        replacement: char,
        add_prefix_space: bool,
    ) -> RbPreTokenizer {
        Metaspace::new(replacement, add_prefix_space).into()
    }
}

pub struct RbPunctuation {}

impl RbPunctuation {
    pub fn new(behavior: RbSplitDelimiterBehavior) -> RbResult<RbPreTokenizer> {
        Ok(Punctuation::new(behavior.into()).into())
    }
}

pub struct RbSplit {}

impl RbSplit {
    pub fn new(pattern: RbPattern, behavior: RbSplitDelimiterBehavior, invert: bool) -> RbResult<RbPreTokenizer> {
        Split::new(pattern, behavior.into(), invert).map(|v| v.into()).map_err(RbError::from)
    }
}

pub struct RbUnicodeScripts {}

impl RbUnicodeScripts {
    pub fn new() -> RbPreTokenizer {
        UnicodeScripts::new().into()
    }
}

pub struct RbWhitespace {}

impl RbWhitespace {
    pub fn new() -> RbPreTokenizer {
        Whitespace::default().into()
    }
}

pub struct RbWhitespaceSplit {}

impl RbWhitespaceSplit {
    pub fn new() -> RbPreTokenizer {
        WhitespaceSplit.into()
    }
}

pub struct RbBertPreTokenizer {}

impl RbBertPreTokenizer {
    pub fn new() -> RbPreTokenizer {
        BertPreTokenizer.into()
    }
}

pub struct RbSequence {}

impl RbSequence {
    fn new(pre_tokenizers: RArray) -> RbResult<RbPreTokenizer> {
        let mut sequence = Vec::with_capacity(pre_tokenizers.len());
        for n in pre_tokenizers.each() {
            let pretokenizer: &RbPreTokenizer = n?.try_convert()?;
            match &pretokenizer.pretok {
                RbPreTokenizerTypeWrapper::Sequence(inner) => {
                    sequence.extend(inner.iter().cloned())
                }
                RbPreTokenizerTypeWrapper::Single(inner) => sequence.push(inner.clone()),
            }
        }
        Ok(RbPreTokenizer::new(RbPreTokenizerTypeWrapper::Sequence(sequence)))
    }
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RbPreTokenizerWrapper {
    // Custom(CustomPreTokenizer),
    Wrapped(PreTokenizerWrapper),
}

impl Serialize for RbPreTokenizerWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        match self {
            RbPreTokenizerWrapper::Wrapped(inner) => inner.serialize(serializer),
            // RbPreTokenizerWrapper::Custom(inner) => inner.serialize(serializer),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RbPreTokenizerTypeWrapper {
    Sequence(Vec<Arc<RwLock<RbPreTokenizerWrapper>>>),
    Single(Arc<RwLock<RbPreTokenizerWrapper>>),
}

impl Serialize for RbPreTokenizerTypeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            RbPreTokenizerTypeWrapper::Sequence(seq) => {
                let mut ser = serializer.serialize_struct("Sequence", 2)?;
                ser.serialize_field("type", "Sequence")?;
                ser.serialize_field("pretokenizers", seq)?;
                ser.end()
            }
            RbPreTokenizerTypeWrapper::Single(inner) => inner.serialize(serializer),
        }
    }
}

impl<I> From<I> for RbPreTokenizerWrapper
where
    I: Into<PreTokenizerWrapper>,
{
    fn from(pretok: I) -> Self {
        RbPreTokenizerWrapper::Wrapped(pretok.into())
    }
}

impl<I> From<I> for RbPreTokenizerTypeWrapper
where
    I: Into<RbPreTokenizerWrapper>,
{
    fn from(pretok: I) -> Self {
        RbPreTokenizerTypeWrapper::Single(Arc::new(RwLock::new(pretok.into())))
    }
}

impl<I> From<I> for RbPreTokenizer
where
    I: Into<PreTokenizerWrapper>,
{
    fn from(pretok: I) -> Self {
        RbPreTokenizer {
            pretok: pretok.into().into(),
        }
    }
}

impl PreTokenizer for RbPreTokenizerTypeWrapper {
    fn pre_tokenize(&self, pretok: &mut PreTokenizedString) -> tk::Result<()> {
        match self {
            RbPreTokenizerTypeWrapper::Single(inner) => inner.read().unwrap().pre_tokenize(pretok),
            RbPreTokenizerTypeWrapper::Sequence(inner) => inner
                .iter()
                .try_for_each(|n| n.read().unwrap().pre_tokenize(pretok)),
        }
    }
}

impl PreTokenizer for RbPreTokenizerWrapper {
    fn pre_tokenize(&self, pretok: &mut PreTokenizedString) -> tk::Result<()> {
        match self {
            RbPreTokenizerWrapper::Wrapped(inner) => inner.pre_tokenize(pretok),
            // RbPreTokenizerWrapper::Custom(inner) => inner.pre_tokenize(pretok),
        }
    }
}

unsafe impl TypedData for RbPreTokenizer {
    fn class() -> RClass {
        *memoize!(RClass: {
          let class: RClass = crate::pre_tokenizers().const_get("PreTokenizer").unwrap();
          class.undef_alloc_func();
          class
        })
    }

    fn data_type() -> &'static DataType {
        memoize!(DataType: DataTypeBuilder::<RbPreTokenizer>::new("Tokenizers::PreTokenizers::PreTokenizer").build())
    }

    fn class_for(value: &Self) -> RClass {
        match &value.pretok {
            RbPreTokenizerTypeWrapper::Sequence(_seq) => *memoize!(RClass: {
                let class: RClass = crate::pre_tokenizers().const_get("Sequence").unwrap();
                class.undef_alloc_func();
                class
            }),
            RbPreTokenizerTypeWrapper::Single(inner) => match &*inner.read().unwrap() {
                RbPreTokenizerWrapper::Wrapped(wrapped) => match &wrapped {
                    PreTokenizerWrapper::BertPreTokenizer(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("BertPreTokenizer").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::ByteLevel(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("ByteLevel").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::Delimiter(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("CharDelimiterSplit").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::Digits(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("Digits").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::Metaspace(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("Metaspace").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::Punctuation(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("Punctuation").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::Split(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("Split").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::UnicodeScripts(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("UnicodeScripts").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::Whitespace(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("Whitespace").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::WhitespaceSplit(_) => *memoize!(RClass: {
                        let class: RClass = crate::pre_tokenizers().const_get("WhitespaceSplit").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    _ => todo!(),
                },
            },
        }
    }
}

pub fn pre_tokenizers(module: &RModule) -> RbResult<()> {
    let pre_tokenizer = module.define_class("PreTokenizer", Default::default())?;
    pre_tokenizer.define_method("pre_tokenize_str", method!(RbPreTokenizer::pre_tokenize_str, 1))?;

    let class = module.define_class("Sequence", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbSequence::new, 1))?;

    let class = module.define_class("BertPreTokenizer", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbBertPreTokenizer::new, 0))?;

    let class = module.define_class("ByteLevel", pre_tokenizer)?;
    class.define_singleton_method("_new", function!(RbByteLevel::new, 2))?;
    class.define_singleton_method("alphabet", function!(RbByteLevel::alphabet, 0))?;
    class.define_method("add_prefix_space", method!(RbPreTokenizer::byte_level_add_prefix_space, 0))?;
    class.define_method("add_prefix_space=", method!(RbPreTokenizer::byte_level_set_add_prefix_space, 1))?;
    class.define_method("use_regex", method!(RbPreTokenizer::byte_level_use_regex, 0))?;
    class.define_method("use_regex=", method!(RbPreTokenizer::byte_level_set_use_regex, 1))?;

    let class = module.define_class("CharDelimiterSplit", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbCharDelimiterSplit::new, 1))?;
    class.define_method("delimiter", method!(RbPreTokenizer::char_delimiter_split_delimiter, 0))?;
    class.define_method("delimiter=", method!(RbPreTokenizer::char_delimiter_split_set_delimiter, 1))?;

    let class = module.define_class("Digits", pre_tokenizer)?;
    class.define_singleton_method("_new", function!(RbDigits::new, 1))?;
    class.define_method("individual_digits", method!(RbPreTokenizer::digits_individual_digits, 0))?;
    class.define_method("individual_digits=", method!(RbPreTokenizer::digits_set_individual_digits, 1))?;

    let class = module.define_class("Metaspace", pre_tokenizer)?;
    class.define_singleton_method("_new", function!(RbMetaspace::new, 2))?;
    class.define_method("add_prefix_space", method!(RbPreTokenizer::metaspace_add_prefix_space, 0))?;
    class.define_method("add_prefix_space=", method!(RbPreTokenizer::metaspace_set_add_prefix_space, 1))?;
    class.define_method("replacement", method!(RbPreTokenizer::metaspace_replacement, 0))?;
    class.define_method("replacement=", method!(RbPreTokenizer::metaspace_set_replacement, 1))?;

    let class = module.define_class("Punctuation", pre_tokenizer)?;
    class.define_singleton_method("_new", function!(RbPunctuation::new, 1))?;

    let class = module.define_class("Split", pre_tokenizer)?;
    class.define_singleton_method("_new", function!(RbSplit::new, 3))?;

    let class = module.define_class("UnicodeScripts", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbUnicodeScripts::new, 0))?;

    let class = module.define_class("Whitespace", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbWhitespace::new, 0))?;

    let class = module.define_class("WhitespaceSplit", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbWhitespaceSplit::new, 0))?;

    Ok(())
}
