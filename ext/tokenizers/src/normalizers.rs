use std::sync::{Arc, RwLock};

use magnus::typed_data::DataTypeBuilder;
use magnus::{
    function, memoize, method, Class, DataType, DataTypeFunctions, Module, Object, RArray, RClass, RModule,
    TypedData,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use tk::normalizers::{
    BertNormalizer, Lowercase, Nmt, NormalizerWrapper, Replace, Prepend, Strip, StripAccents,
    NFC, NFD, NFKC, NFKD,
};
use tk::{NormalizedString, Normalizer};

use super::utils::*;
use super::{RbError, RbResult};

#[derive(DataTypeFunctions, Clone, Serialize, Deserialize)]
pub struct RbNormalizer {
    #[serde(flatten)]
    pub(crate) normalizer: RbNormalizerTypeWrapper,
}

impl RbNormalizer {
    pub(crate) fn new(normalizer: RbNormalizerTypeWrapper) -> Self {
        RbNormalizer { normalizer }
    }

    pub fn normalize_str(&self, sequence: String) -> RbResult<String> {
        let mut normalized = NormalizedString::from(sequence);
        self.normalizer.normalize(&mut normalized).map_err(RbError::from)?;
        Ok(normalized.get().to_owned())
    }
}

impl Normalizer for RbNormalizer {
    fn normalize(&self, normalized: &mut NormalizedString) -> tk::Result<()> {
        self.normalizer.normalize(normalized)
    }
}

macro_rules! getter {
    ($self: ident, $variant: ident, $name: ident) => {{
        if let RbNormalizerTypeWrapper::Single(ref norm) = &$self.normalizer {
            let wrapper = norm.read().unwrap();
            if let RbNormalizerWrapper::Wrapped(NormalizerWrapper::$variant(o)) = (*wrapper).clone() {
                o.$name
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
        if let RbNormalizerTypeWrapper::Single(ref norm) = &$self.normalizer {
            let mut wrapper = norm.write().unwrap();
            if let RbNormalizerWrapper::Wrapped(NormalizerWrapper::$variant(ref mut o)) = *wrapper {
                o.$name = $value;
            }
        }
    }};
}

impl RbNormalizer {

    fn bert_clean_text(&self) -> bool {
        getter!(self, BertNormalizer, clean_text)
    }

    fn bert_set_clean_text(&self, clean_text: bool) {
        setter!(self, BertNormalizer, clean_text, clean_text);
    }

    fn bert_handle_chinese_chars(&self) -> bool {
        getter!(self, BertNormalizer, handle_chinese_chars)
    }

    fn bert_set_handle_chinese_chars(&self, handle_chinese_chars: bool) {
        setter!(
            self,
            BertNormalizer,
            handle_chinese_chars,
            handle_chinese_chars
        );
    }

    fn bert_strip_accents(&self) -> Option<bool> {
        getter!(self, BertNormalizer, strip_accents)
    }

    fn bert_set_strip_accents(&self, strip_accents: Option<bool>) {
        setter!(self, BertNormalizer, strip_accents, strip_accents);
    }

    fn bert_lowercase(&self) -> bool {
        getter!(self, BertNormalizer, lowercase)
    }

    fn bert_set_lowercase(&self, lowercase: bool) {
        setter!(self, BertNormalizer, lowercase, lowercase)
    }

    fn prepend_prepend(&self) -> String {
        getter!(self, Prepend, prepend)
    }

    fn prepend_set_prepend(&self, prepend: String) {
        setter!(self, Prepend, prepend, prepend)
    }

    fn strip_left(&self) -> bool {
        getter!(self, StripNormalizer, strip_left)
    }

    fn strip_set_left(&self, left: bool) {
        setter!(self, StripNormalizer, strip_left, left)
    }

    fn strip_right(&self) -> bool {
        getter!(self, StripNormalizer, strip_right)
    }

    fn strip_set_right(&self, right: bool) {
        setter!(self, StripNormalizer, strip_right, right)
    }
}

pub struct RbBertNormalizer {}

impl RbBertNormalizer {
    pub fn new(clean_text: bool, handle_chinese_chars: bool, strip_accents: Option<bool>, lowercase: bool) -> RbNormalizer {
        BertNormalizer::new(clean_text, handle_chinese_chars, strip_accents, lowercase).into()
    }
}

pub struct RbLowercase {}

impl RbLowercase {
    pub fn new() -> RbNormalizer {
        Lowercase.into()
    }
}

pub struct RbNFC {}

impl RbNFC {
    pub fn new() -> RbNormalizer {
        NFC.into()
    }
}

pub struct RbNFD {}

impl RbNFD {
    pub fn new() -> RbNormalizer {
        NFD.into()
    }
}

pub struct RbNFKC {}

impl RbNFKC {
    pub fn new() -> RbNormalizer {
        NFKC.into()
    }
}

pub struct RbNFKD {}

impl RbNFKD {
    pub fn new() -> RbNormalizer {
        NFKD.into()
    }
}

pub struct RbNmt {}

impl RbNmt {
    pub fn new() -> RbNormalizer {
        Nmt.into()
    }
}

pub struct RbReplace {}

impl RbReplace {
    pub fn new(pattern: RbPattern, content: String) -> RbResult<RbNormalizer> {
        Replace::new(pattern, content).map(|v| v.into()).map_err(RbError::from)
    }
}

pub struct RbPrepend {}

impl RbPrepend {
    pub fn new(prepend: String) -> RbNormalizer {
        Prepend::new(prepend).into()
    }
}

pub struct RbStrip {}

impl RbStrip {
    pub fn new(left: bool, right: bool) -> RbNormalizer {
        Strip::new(left, right).into()
    }
}

pub struct RbStripAccents {}

impl RbStripAccents {
    pub fn new() -> RbNormalizer {
        StripAccents.into()
    }
}

pub struct RbSequence {}

impl RbSequence {
    fn new(normalizers: RArray) -> RbResult<RbNormalizer> {
        let mut sequence = Vec::with_capacity(normalizers.len());
        for n in normalizers.each() {
            let normalizer: &RbNormalizer = n?.try_convert()?;
            match &normalizer.normalizer {
                RbNormalizerTypeWrapper::Sequence(inner) => sequence.extend(inner.iter().cloned()),
                RbNormalizerTypeWrapper::Single(inner) => sequence.push(inner.clone()),
            }
        }
        Ok(RbNormalizer::new(RbNormalizerTypeWrapper::Sequence(sequence)))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RbNormalizerWrapper {
    // Custom(CustomNormalizer),
    Wrapped(NormalizerWrapper),
}

impl Serialize for RbNormalizerWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        match self {
            RbNormalizerWrapper::Wrapped(inner) => inner.serialize(serializer),
            // RbNormalizerWrapper::Custom(inner) => inner.serialize(serializer),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RbNormalizerTypeWrapper {
    Sequence(Vec<Arc<RwLock<RbNormalizerWrapper>>>),
    Single(Arc<RwLock<RbNormalizerWrapper>>),
}

impl Serialize for RbNormalizerTypeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            RbNormalizerTypeWrapper::Sequence(seq) => {
                let mut ser = serializer.serialize_struct("Sequence", 2)?;
                ser.serialize_field("type", "Sequence")?;
                ser.serialize_field("normalizers", seq)?;
                ser.end()
            }
            RbNormalizerTypeWrapper::Single(inner) => inner.serialize(serializer),
        }
    }
}

impl<I> From<I> for RbNormalizerWrapper
where
    I: Into<NormalizerWrapper>,
{
    fn from(norm: I) -> Self {
        RbNormalizerWrapper::Wrapped(norm.into())
    }
}

impl<I> From<I> for RbNormalizerTypeWrapper
where
    I: Into<RbNormalizerWrapper>,
{
    fn from(norm: I) -> Self {
        RbNormalizerTypeWrapper::Single(Arc::new(RwLock::new(norm.into())))
    }
}

impl<I> From<I> for RbNormalizer
where
    I: Into<NormalizerWrapper>,
{
    fn from(norm: I) -> Self {
        RbNormalizer {
            normalizer: norm.into().into(),
        }
    }
}

impl Normalizer for RbNormalizerTypeWrapper {
    fn normalize(&self, normalized: &mut NormalizedString) -> tk::Result<()> {
        match self {
            RbNormalizerTypeWrapper::Single(inner) => inner.read().unwrap().normalize(normalized),
            RbNormalizerTypeWrapper::Sequence(inner) => inner
                .iter()
                .try_for_each(|n| n.read().unwrap().normalize(normalized)),
        }
    }
}

impl Normalizer for RbNormalizerWrapper {
    fn normalize(&self, normalized: &mut NormalizedString) -> tk::Result<()> {
        match self {
            RbNormalizerWrapper::Wrapped(inner) => inner.normalize(normalized),
            // RbNormalizerWrapper::Custom(inner) => inner.normalize(normalized),
        }
    }
}

unsafe impl TypedData for RbNormalizer {
    fn class() -> RClass {
        *memoize!(RClass: {
          let class: RClass = crate::normalizers().const_get("Normalizer").unwrap();
          class.undef_alloc_func();
          class
        })
    }

    fn data_type() -> &'static DataType {
        memoize!(DataType: DataTypeBuilder::<RbNormalizer>::new("Tokenizers::Normalizers::Normalizer").build())
    }

    fn class_for(value: &Self) -> RClass {
        match &value.normalizer {
            RbNormalizerTypeWrapper::Sequence(_seq) => *memoize!(RClass: {
                let class: RClass = crate::normalizers().const_get("Sequence").unwrap();
                class.undef_alloc_func();
                class
            }),
            RbNormalizerTypeWrapper::Single(inner) => match &*inner.read().unwrap() {
                RbNormalizerWrapper::Wrapped(wrapped) => match &wrapped {
                    NormalizerWrapper::BertNormalizer(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("BertNormalizer").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::Lowercase(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("Lowercase").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::NFD(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("NFD").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::NFC(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("NFC").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::NFKC(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("NFKC").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::NFKD(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("NFKD").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::Nmt(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("Nmt").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::Replace(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("Replace").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::Prepend(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("Prepend").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::StripNormalizer(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("Strip").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    NormalizerWrapper::StripAccents(_) => *memoize!(RClass: {
                        let class: RClass = crate::normalizers().const_get("StripAccents").unwrap();
                        class.undef_alloc_func();
                        class
                    }),
                    _ => todo!(),
                },
            },
        }
    }
}

pub fn normalizers(module: &RModule) -> RbResult<()> {
    let normalizer = module.define_class("Normalizer", Default::default())?;
    normalizer.define_method("normalize_str", method!(RbNormalizer::normalize_str, 1))?;

    let class = module.define_class("Sequence", normalizer)?;
    class.define_singleton_method("new", function!(RbSequence::new, 1))?;

    let class = module.define_class("BertNormalizer", normalizer)?;
    class.define_singleton_method("_new", function!(RbBertNormalizer::new, 4))?;
    class.define_method("clean_text", method!(RbNormalizer::bert_clean_text, 0))?;
    class.define_method("clean_text=", method!(RbNormalizer::bert_set_clean_text, 1))?;
    class.define_method("handle_chinese_chars", method!(RbNormalizer::bert_handle_chinese_chars, 0))?;
    class.define_method("handle_chinese_chars=", method!(RbNormalizer::bert_set_handle_chinese_chars, 1))?;
    class.define_method("strip_accents", method!(RbNormalizer::bert_strip_accents, 0))?;
    class.define_method("strip_accents=", method!(RbNormalizer::bert_set_strip_accents, 1))?;
    class.define_method("lowercase", method!(RbNormalizer::bert_lowercase, 0))?;
    class.define_method("lowercase=", method!(RbNormalizer::bert_set_lowercase, 1))?;

    let class = module.define_class("Lowercase", normalizer)?;
    class.define_singleton_method("new", function!(RbLowercase::new, 0))?;

    let class = module.define_class("NFC", normalizer)?;
    class.define_singleton_method("new", function!(RbNFC::new, 0))?;

    let class = module.define_class("NFD", normalizer)?;
    class.define_singleton_method("new", function!(RbNFD::new, 0))?;

    let class = module.define_class("NFKC", normalizer)?;
    class.define_singleton_method("new", function!(RbNFKC::new, 0))?;

    let class = module.define_class("NFKD", normalizer)?;
    class.define_singleton_method("new", function!(RbNFKD::new, 0))?;

    let class = module.define_class("Nmt", normalizer)?;
    class.define_singleton_method("new", function!(RbNmt::new, 0))?;

    let class = module.define_class("Replace", normalizer)?;
    class.define_singleton_method("new", function!(RbReplace::new, 2))?;

    let class = module.define_class("Prepend", normalizer)?;
    class.define_singleton_method("_new", function!(RbPrepend::new, 1))?;
    class.define_method("prepend", method!(RbNormalizer::prepend_prepend, 0))?;
    class.define_method("prepend=", method!(RbNormalizer::prepend_set_prepend, 1))?;

    let class = module.define_class("Strip", normalizer)?;
    class.define_singleton_method("_new", function!(RbStrip::new, 2))?;
    class.define_method("left", method!(RbNormalizer::strip_left, 0))?;
    class.define_method("left=", method!(RbNormalizer::strip_set_left, 1))?;
    class.define_method("right", method!(RbNormalizer::strip_right, 0))?;
    class.define_method("right=", method!(RbNormalizer::strip_set_right, 1))?;

    let class = module.define_class("StripAccents", normalizer)?;
    class.define_singleton_method("new", function!(RbStripAccents::new, 0))?;

    Ok(())
}
