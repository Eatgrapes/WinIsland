/// A borrowed translation key-value pair.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TranslationPairV1 {
    pub key: crate::Utf8SliceV1,
    pub value: crate::Utf8SliceV1,
}
