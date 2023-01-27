use std::sync::{Arc, RwLock};

use magnus::typed_data::DataTypeBuilder;
use magnus::{
    function, memoize, Class, DataType, DataTypeFunctions, Module, Object, RClass, RModule,
    TypedData,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use tk::pre_tokenizers::bert::BertPreTokenizer;
use tk::pre_tokenizers::whitespace::Whitespace;
use tk::pre_tokenizers::PreTokenizerWrapper;
use tk::{PreTokenizedString, PreTokenizer};

use super::{module, RbResult};

#[derive(DataTypeFunctions, Clone, Serialize, Deserialize)]
pub struct RbPreTokenizer {
    #[serde(flatten)]
    pub(crate) pretok: RbPreTokenizerTypeWrapper,
}

impl RbPreTokenizer {
    #[allow(dead_code)]
    pub(crate) fn new(pretok: RbPreTokenizerTypeWrapper) -> Self {
        RbPreTokenizer { pretok }
    }
}

impl PreTokenizer for RbPreTokenizer {
    fn pre_tokenize(&self, normalized: &mut PreTokenizedString) -> tk::Result<()> {
        self.pretok.pre_tokenize(normalized)
    }
}

pub struct RbWhitespace {}

impl RbWhitespace {
    pub fn new() -> RbPreTokenizer {
        Whitespace::default().into()
    }
}

pub struct RbBertPreTokenizer {}

impl RbBertPreTokenizer {
    pub fn new() -> RbPreTokenizer {
        BertPreTokenizer.into()
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
          let class: RClass = module().const_get("PreTokenizer").unwrap();
          class.undef_alloc_func();
          class
        })
    }

    fn data_type() -> &'static DataType {
        memoize!(DataType: DataTypeBuilder::<RbPreTokenizer>::new("Tokenizers::PreTokenizer").build())
    }

    fn class_for(value: &Self) -> RClass {
        match &value.pretok {
            RbPreTokenizerTypeWrapper::Sequence(_seq) => todo!(),
            RbPreTokenizerTypeWrapper::Single(inner) => match &*inner.read().unwrap() {
                RbPreTokenizerWrapper::Wrapped(wrapped) => match &wrapped {
                    PreTokenizerWrapper::Whitespace(_) => *memoize!(RClass: {
                        let class: RClass = module().const_get("Whitespace").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    PreTokenizerWrapper::BertPreTokenizer(_) => *memoize!(RClass: {
                        let class: RClass = module().const_get("BertPreTokenizer").unwrap();
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

    let class = module.define_class("BertPreTokenizer", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbBertPreTokenizer::new, 0))?;

    let class = module.define_class("Whitespace", pre_tokenizer)?;
    class.define_singleton_method("new", function!(RbWhitespace::new, 0))?;

    Ok(())
}
