use minicbor::{
    data::{IanaTag, Tag, Type},
    decode::Error,
    Decode, Encode,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, str::FromStr};
use std::{
    collections::HashMap,
    fmt,
    hash::Hash as StdHash,
    ops::{Deref, DerefMut},
};

static TAG_SET: u64 = 258;

/// Utility for skipping parts of the CBOR payload, use only for debugging
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct SkipCbor<const N: usize> {}

impl<'b, C, const N: usize> minicbor::Decode<'b, C> for SkipCbor<N> {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        {
            let probe = d.probe();
            println!("skipped cbor value {N}: {:?}", probe.datatype()?);
        }

        d.skip()?;
        Ok(SkipCbor {})
    }
}

impl<C, const N: usize> minicbor::Encode<C> for SkipCbor<N> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        _e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        todo!()
    }
}

/// Custom collection to ensure ordered pairs of values
///
/// Since the ordering of the entries requires a particular order to maintain
/// canonicalization for isomorphic decoding / encoding operators, we use a Vec
/// as the underlaying struct for storage of the items (as opposed to a BTreeMap
/// or HashMap).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(from = "Vec::<(K, V)>", into = "Vec::<(K, V)>")]
pub enum KeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    Def(Vec<(K, V)>),
    Indef(Vec<(K, V)>),
}

impl<K, V> KeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    pub fn to_vec(self) -> Vec<(K, V)> {
        self.into()
    }
}

impl<K, V> FromIterator<(K, V)> for KeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        KeyValuePairs::Def(Vec::from_iter(iter))
    }
}

impl<K, V> From<KeyValuePairs<K, V>> for Vec<(K, V)>
where
    K: Clone,
    V: Clone,
{
    fn from(other: KeyValuePairs<K, V>) -> Self {
        match other {
            KeyValuePairs::Def(x) => x,
            KeyValuePairs::Indef(x) => x,
        }
    }
}

impl<K, V> From<Vec<(K, V)>> for KeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    fn from(other: Vec<(K, V)>) -> Self {
        KeyValuePairs::Def(other)
    }
}

impl<K, V> From<KeyValuePairs<K, V>> for HashMap<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    fn from(other: KeyValuePairs<K, V>) -> Self {
        match other {
            KeyValuePairs::Def(x) => x.into_iter().collect(),
            KeyValuePairs::Indef(x) => x.into_iter().collect(),
        }
    }
}
impl<K, V> From<HashMap<K, V>> for KeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    fn from(other: HashMap<K, V>) -> Self {
        KeyValuePairs::Def(other.into_iter().collect())
    }
}

impl<K, V> Deref for KeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    type Target = Vec<(K, V)>;

    fn deref(&self) -> &Self::Target {
        match self {
            KeyValuePairs::Def(x) => x,
            KeyValuePairs::Indef(x) => x,
        }
    }
}

impl<'b, C, K, V> minicbor::decode::Decode<'b, C> for KeyValuePairs<K, V>
where
    K: Decode<'b, C> + Clone,
    V: Decode<'b, C> + Clone,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        let items: Result<Vec<_>, _> = d.map_iter_with::<C, K, V>(ctx)?.collect();
        let items = items?;

        match datatype {
            minicbor::data::Type::Map => Ok(KeyValuePairs::Def(items)),
            minicbor::data::Type::MapIndef => Ok(KeyValuePairs::Indef(items)),
            _ => Err(minicbor::decode::Error::message(
                "invalid data type for keyvaluepairs",
            )),
        }
    }
}

impl<C, K, V> minicbor::encode::Encode<C> for KeyValuePairs<K, V>
where
    K: Encode<C> + Clone,
    V: Encode<C> + Clone,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            KeyValuePairs::Def(x) => {
                e.map(x.len() as u64)?;

                for (k, v) in x.iter() {
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }
            }
            KeyValuePairs::Indef(x) => {
                e.begin_map()?;

                for (k, v) in x.iter() {
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }

                e.end()?;
            }
        }

        Ok(())
    }
}

/// Custom collection to ensure ordered pairs of values (non-empty)
///
/// Since the ordering of the entries requires a particular order to maintain
/// canonicalization for isomorphic decoding / encoding operators, we use a Vec
/// as the underlaying struct for storage of the items (as opposed to a BTreeMap
/// or HashMap).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "Vec::<(K, V)>", into = "Vec::<(K, V)>")]
pub enum NonEmptyKeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    Def(Vec<(K, V)>),
    Indef(Vec<(K, V)>),
}

impl<K, V> IntoIterator for NonEmptyKeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            NonEmptyKeyValuePairs::Def(pairs) => pairs.into_iter(),
            NonEmptyKeyValuePairs::Indef(pairs) => pairs.into_iter(),
        }
    }
}

impl<K, V> NonEmptyKeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    pub fn to_vec(self) -> Vec<(K, V)> {
        self.into()
    }

    pub fn from_vec(x: Vec<(K, V)>) -> Option<Self> {
        if x.is_empty() {
            None
        } else {
            Some(NonEmptyKeyValuePairs::Def(x))
        }
    }
}

impl<K, V> From<NonEmptyKeyValuePairs<K, V>> for Vec<(K, V)>
where
    K: Clone,
    V: Clone,
{
    fn from(other: NonEmptyKeyValuePairs<K, V>) -> Self {
        match other {
            NonEmptyKeyValuePairs::Def(x) => x,
            NonEmptyKeyValuePairs::Indef(x) => x,
        }
    }
}

impl<K, V> TryFrom<Vec<(K, V)>> for NonEmptyKeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    type Error = String;

    fn try_from(value: Vec<(K, V)>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err("NonEmptyKeyValuePairs must contain at least one element".into())
        } else {
            Ok(NonEmptyKeyValuePairs::Def(value))
        }
    }
}

impl<K, V> TryFrom<KeyValuePairs<K, V>> for NonEmptyKeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    type Error = String;

    fn try_from(value: KeyValuePairs<K, V>) -> Result<Self, Self::Error> {
        match value {
            KeyValuePairs::Def(x) => {
                if x.is_empty() {
                    Err("NonEmptyKeyValuePairs must contain at least one element".into())
                } else {
                    Ok(NonEmptyKeyValuePairs::Def(x))
                }
            }
            KeyValuePairs::Indef(x) => {
                if x.is_empty() {
                    Err("NonEmptyKeyValuePairs must contain at least one element".into())
                } else {
                    Ok(NonEmptyKeyValuePairs::Indef(x))
                }
            }
        }
    }
}

impl<K, V> Deref for NonEmptyKeyValuePairs<K, V>
where
    K: Clone,
    V: Clone,
{
    type Target = Vec<(K, V)>;

    fn deref(&self) -> &Self::Target {
        match self {
            NonEmptyKeyValuePairs::Def(x) => x,
            NonEmptyKeyValuePairs::Indef(x) => x,
        }
    }
}

impl<'b, C, K, V> minicbor::decode::Decode<'b, C> for NonEmptyKeyValuePairs<K, V>
where
    K: Decode<'b, C> + Clone,
    V: Decode<'b, C> + Clone,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        let items: Result<Vec<_>, _> = d.map_iter_with::<C, K, V>(ctx)?.collect();
        let items = items?;

        // if items.is_empty() {
        //     return Err(Error::message(
        //         "decoding empty map as NonEmptyKeyValuePairs",
        //     ));
        // }

        match datatype {
            minicbor::data::Type::Map => Ok(NonEmptyKeyValuePairs::Def(items)),
            minicbor::data::Type::MapIndef => Ok(NonEmptyKeyValuePairs::Indef(items)),
            _ => Err(minicbor::decode::Error::message(
                "invalid data type for nonemptykeyvaluepairs",
            )),
        }
    }
}

impl<C, K, V> minicbor::encode::Encode<C> for NonEmptyKeyValuePairs<K, V>
where
    K: Encode<C> + Clone,
    V: Encode<C> + Clone,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            NonEmptyKeyValuePairs::Def(x) => {
                e.map(x.len() as u64)?;

                for (k, v) in x.iter() {
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }
            }
            NonEmptyKeyValuePairs::Indef(x) => {
                e.begin_map()?;

                for (k, v) in x.iter() {
                    k.encode(e, ctx)?;
                    v.encode(e, ctx)?;
                }

                e.end()?;
            }
        }

        Ok(())
    }
}

/// A struct that maintains a reference to whether a cbor array was indef or not
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum MaybeIndefArray<A> {
    Def(Vec<A>),
    Indef(Vec<A>),
}

impl<A> MaybeIndefArray<A> {
    pub fn to_vec(self) -> Vec<A> {
        self.into()
    }
}

impl<A> Deref for MaybeIndefArray<A> {
    type Target = Vec<A>;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeIndefArray::Def(x) => x,
            MaybeIndefArray::Indef(x) => x,
        }
    }
}

impl<A> From<MaybeIndefArray<A>> for Vec<A> {
    fn from(other: MaybeIndefArray<A>) -> Self {
        match other {
            MaybeIndefArray::Def(x) => x,
            MaybeIndefArray::Indef(x) => x,
        }
    }
}

impl<'b, C, A> minicbor::decode::Decode<'b, C> for MaybeIndefArray<A>
where
    A: minicbor::decode::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let datatype = d.datatype()?;

        match datatype {
            minicbor::data::Type::Array => Ok(Self::Def(d.decode_with(ctx)?)),
            minicbor::data::Type::ArrayIndef => Ok(Self::Indef(d.decode_with(ctx)?)),
            _ => Err(minicbor::decode::Error::message(
                "unknown data type of maybe indef array",
            )),
        }
    }
}

impl<C, A> minicbor::encode::Encode<C> for MaybeIndefArray<A>
where
    A: minicbor::encode::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            MaybeIndefArray::Def(x) => {
                e.encode_with(x, ctx)?;
            }
            // TODO: this seemed necesary on alonzo, but breaks on byron. We need to double check.
            //MaybeIndefArray::Indef(x) if x.is_empty() => {
            //    e.encode(x)?;
            //}
            MaybeIndefArray::Indef(x) => {
                e.begin_array()?;

                for v in x.iter() {
                    e.encode_with(v, ctx)?;
                }

                e.end()?;
            }
        };

        Ok(())
    }
}

/// Order-preserving set of attributes
///
/// There's no guarantee that the entries on a Cardano cbor entity that uses
/// maps for its representation will follow the canonical order specified by the
/// standard. To implement an isomorphic codec, we need a way of preserving the
/// original order in which the entries were encoded. To acomplish this, we
/// transform key-value structures into an orderer vec of `properties`, where
/// each entry represents a a cbor-encodable variant of an attribute of the
/// struct.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd)]
pub struct OrderPreservingProperties<P>(Vec<P>);

impl<P> Deref for OrderPreservingProperties<P> {
    type Target = Vec<P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> From<Vec<P>> for OrderPreservingProperties<P> {
    fn from(value: Vec<P>) -> Self {
        OrderPreservingProperties(value)
    }
}

impl<'b, C, P> minicbor::decode::Decode<'b, C> for OrderPreservingProperties<P>
where
    P: Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let len = d.map()?.unwrap_or_default();

        let components: Result<_, _> = (0..len).map(|_| d.decode_with(ctx)).collect();

        Ok(Self(components?))
    }
}

impl<C, P> minicbor::encode::Encode<C> for OrderPreservingProperties<P>
where
    P: Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(self.0.len() as u64)?;
        for component in &self.0 {
            e.encode_with(component, ctx)?;
        }

        Ok(())
    }
}

/// Wraps a struct so that it is encoded/decoded as a cbor bytes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, StdHash)]
#[serde(transparent)]
pub struct CborWrap<T>(pub T);

impl<T> CborWrap<T> {
    pub fn unwrap(self) -> T {
        self.0
    }
}

impl<'b, C, T> minicbor::Decode<'b, C> for CborWrap<T>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;
        let cbor = d.bytes()?;
        let wrapped = minicbor::decode_with(cbor, ctx)?;

        Ok(CborWrap(wrapped))
    }
}

impl<C, T> minicbor::Encode<C> for CborWrap<T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let buf = minicbor::to_vec_with(&self.0, ctx).map_err(|_| {
            minicbor::encode::Error::message("error encoding cbor-wrapped structure")
        })?;

        e.tag(IanaTag::Cbor)?;
        e.bytes(&buf)?;

        Ok(())
    }
}

impl<T> Deref for CborWrap<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagWrap<I, const T: u64>(pub I);

impl<I, const T: u64> TagWrap<I, T> {
    pub fn new(inner: I) -> Self {
        TagWrap(inner)
    }
}

impl<I, const T: u64> From<I> for TagWrap<I, T> {
    fn from(inner: I) -> Self {
        TagWrap(inner)
    }
}

impl<'b, C, I, const T: u64> minicbor::Decode<'b, C> for TagWrap<I, T>
where
    I: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.tag()?;

        Ok(TagWrap(d.decode_with(ctx)?))
    }
}

impl<C, I, const T: u64> minicbor::Encode<C> for TagWrap<I, T>
where
    I: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::new(T))?;
        e.encode_with(&self.0, ctx)?;

        Ok(())
    }
}

impl<I, const T: u64> Deref for TagWrap<I, T> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An empty map
///
/// don't ask me why, that's what the CDDL asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyMap;

impl<'b, C> minicbor::decode::Decode<'b, C> for EmptyMap {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.skip()?;
        Ok(EmptyMap)
    }
}

impl<C> minicbor::encode::Encode<C> for EmptyMap {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(0)?;

        Ok(())
    }
}

/// An array with zero or one elements
///
/// A common pattern seen in the CDDL is to represent optional values as an
/// array containing zero or more items. This structure reflects that pattern
/// while providing semantic meaning.
#[derive(Debug, Clone)]
pub struct ZeroOrOneArray<T>(Option<T>);

impl<T> Deref for ZeroOrOneArray<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'b, C, T> minicbor::decode::Decode<'b, C> for ZeroOrOneArray<T>
where
    T: Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let len = d.array()?;

        match len {
            Some(0) => Ok(ZeroOrOneArray(None)),
            Some(1) => Ok(ZeroOrOneArray(Some(d.decode_with(ctx)?))),
            Some(_) => Err(minicbor::decode::Error::message(
                "found invalid len for zero-or-one pattern",
            )),
            None => Err(minicbor::decode::Error::message(
                "found invalid indefinite len array for zero-or-one pattern",
            )),
        }
    }
}

impl<C, T> minicbor::encode::Encode<C> for ZeroOrOneArray<T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match &self.0 {
            Some(x) => {
                e.array(1)?;
                e.encode_with(x, ctx)?;
            }
            None => {
                e.array(0)?;
            }
        }

        Ok(())
    }
}

/// Set
///
/// Optional 258 tag (until era after Conway, at which point is it required)
/// with a vec of items which should contain no duplicates
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Serialize, Deserialize)]
pub struct Set<T>(Vec<T>);

impl<T> Set<T> {
    pub fn to_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T> Deref for Set<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<Vec<T>> for Set<T> {
    fn from(value: Vec<T>) -> Self {
        Set(value)
    }
}

impl<T> From<Set<KeepRaw<'_, T>>> for Set<T> {
    fn from(value: Set<KeepRaw<'_, T>>) -> Self {
        let inner = value.0.into_iter().map(|x| x.unwrap()).collect();
        Self(inner)
    }
}

impl<'a, T> IntoIterator for &'a Set<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'b, C, T> minicbor::decode::Decode<'b, C> for Set<T>
where
    T: Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        // decode optional set tag (this will be required in era following Conway)
        if d.datatype()? == Type::Tag {
            let found_tag = d.tag()?;

            if found_tag != Tag::new(TAG_SET) {
                return Err(Error::message(format!("Unrecognised tag: {found_tag:?}")));
            }
        }

        Ok(Self(d.decode_with(ctx)?))
    }
}

impl<C, T> minicbor::encode::Encode<C> for Set<T>
where
    T: Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::new(TAG_SET))?;
        e.encode_with(&self.0, ctx)?;

        Ok(())
    }
}

/// Non-empty Set
///
/// Optional 258 tag (until era after Conway, at which point is it required)
/// with a vec of items which should contain no duplicates
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Serialize, Deserialize)]
pub struct NonEmptySet<T>(Vec<T>);

impl<T> NonEmptySet<T> {
    pub fn to_vec(self) -> Vec<T> {
        self.0
    }

    pub fn from_vec(x: Vec<T>) -> Option<Self> {
        if x.is_empty() {
            None
        } else {
            Some(Self(x))
        }
    }
}

impl<T> Deref for NonEmptySet<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> TryFrom<Vec<T>> for NonEmptySet<T> {
    type Error = Vec<T>;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(value)
        } else {
            Ok(NonEmptySet(value))
        }
    }
}

impl<T> From<NonEmptySet<KeepRaw<'_, T>>> for NonEmptySet<T> {
    fn from(value: NonEmptySet<KeepRaw<'_, T>>) -> Self {
        let inner = value.0.into_iter().map(|x| x.unwrap()).collect();
        Self(inner)
    }
}

impl<'a, T> IntoIterator for &'a NonEmptySet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'b, C, T> minicbor::decode::Decode<'b, C> for NonEmptySet<T>
where
    T: Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        // decode optional set tag (this will be required in era following Conway)
        if d.datatype()? == Type::Tag {
            let found_tag = d.tag()?;

            if found_tag != Tag::new(TAG_SET) {
                return Err(Error::message(format!("Unrecognised tag: {found_tag:?}")));
            }
        }

        let inner: Vec<T> = d.decode_with(ctx)?;

        // if inner.is_empty() {
        //     return Err(Error::message("decoding empty set as NonEmptySet"));
        // }

        Ok(Self(inner))
    }
}

impl<C, T> minicbor::encode::Encode<C> for NonEmptySet<T>
where
    T: Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(Tag::new(TAG_SET))?;
        e.encode_with(&self.0, ctx)?;

        Ok(())
    }
}

/// A uint structure that preserves original int length
#[derive(Debug, PartialEq, Copy, Clone, PartialOrd, Eq, Ord, Hash)]
pub enum AnyUInt {
    MajorByte(u8),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl<'b, C> minicbor::decode::Decode<'b, C> for AnyUInt {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let data_type = d.datatype()?;

        use minicbor::data::Type::*;
        match data_type {
            U8 => match d.u8()? {
                x @ 0..=0x17 => Ok(AnyUInt::MajorByte(x)),
                x @ 0x18..=0xff => Ok(AnyUInt::U8(x)),
            },
            U16 => Ok(AnyUInt::U16(d.u16()?)),
            U32 => Ok(AnyUInt::U32(d.u32()?)),
            U64 => Ok(AnyUInt::U64(d.u64()?)),
            _ => Err(minicbor::decode::Error::message(format!(
                "invalid data type for AnyUInt at position {}: {}",
                d.position(),
                data_type
            ))),
        }
    }
}

impl<C> minicbor::encode::Encode<C> for AnyUInt {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            AnyUInt::MajorByte(x) => {
                let b = &x.to_be_bytes()[..];

                e.writer_mut()
                    .write_all(b)
                    .map_err(minicbor::encode::Error::write)?;

                Ok(())
            }
            AnyUInt::U8(x) => {
                let x = x.to_be_bytes();
                let b = &[[24u8], x].concat()[..];

                e.writer_mut()
                    .write_all(b)
                    .map_err(minicbor::encode::Error::write)?;

                Ok(())
            }
            AnyUInt::U16(x) => {
                let x = &x.to_be_bytes()[..];
                let b = &[&[25u8], x].concat()[..];

                e.writer_mut()
                    .write_all(b)
                    .map_err(minicbor::encode::Error::write)?;

                Ok(())
            }
            AnyUInt::U32(x) => {
                let x = &x.to_be_bytes()[..];
                let b = &[&[26u8], x].concat()[..];

                e.writer_mut()
                    .write_all(b)
                    .map_err(minicbor::encode::Error::write)?;

                Ok(())
            }
            AnyUInt::U64(x) => {
                let x = &x.to_be_bytes()[..];
                let b = &[&[27u8], x].concat()[..];

                e.writer_mut()
                    .write_all(b)
                    .map_err(minicbor::encode::Error::write)?;

                Ok(())
            }
        }
    }
}

impl From<AnyUInt> for u64 {
    fn from(x: AnyUInt) -> Self {
        match x {
            AnyUInt::MajorByte(x) => x as u64,
            AnyUInt::U8(x) => x as u64,
            AnyUInt::U16(x) => x as u64,
            AnyUInt::U32(x) => x as u64,
            AnyUInt::U64(x) => x,
        }
    }
}

impl From<&AnyUInt> for u64 {
    fn from(x: &AnyUInt) -> Self {
        u64::from(*x)
    }
}

/// Introduced in Conway
/// positive_coin = 1 .. 18446744073709551615
#[derive(
    Encode, Decode, Debug, PartialEq, Copy, Clone, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct PositiveCoin(u64);

impl TryFrom<u64> for PositiveCoin {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err(value);
        }

        Ok(Self(value))
    }
}

impl From<PositiveCoin> for u64 {
    fn from(value: PositiveCoin) -> Self {
        value.0
    }
}

impl From<&PositiveCoin> for u64 {
    fn from(x: &PositiveCoin) -> Self {
        u64::from(*x)
    }
}

/// Introduced in Conway
/// negInt64 = -9223372036854775808 .. -1
/// posInt64 = 1 .. 9223372036854775807
/// nonZeroInt64 = negInt64 / posInt64 ; this is the same as the current int64
/// definition but without zero
#[derive(Debug, PartialEq, Copy, Clone, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NonZeroInt(i64);

impl TryFrom<i64> for NonZeroInt {
    type Error = i64;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err(value);
        }

        Ok(Self(value))
    }
}

impl From<NonZeroInt> for i64 {
    fn from(value: NonZeroInt) -> Self {
        value.0
    }
}

impl From<&NonZeroInt> for i64 {
    fn from(x: &NonZeroInt) -> Self {
        i64::from(*x)
    }
}

impl<'b, C> minicbor::decode::Decode<'b, C> for NonZeroInt {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let n = d.decode_with(ctx)?;

        if n == 0 {
            return Err(Error::message("decoding 0 as NonZeroInt"));
        }

        Ok(Self(n))
    }
}

impl<C> minicbor::encode::Encode<C> for NonZeroInt {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(self.0)?;

        Ok(())
    }
}

/// Decodes a struct while preserving original CBOR
///
/// # Examples
///
/// ```
/// use pallas_codec::utils::KeepRaw;
///
/// let a = (123u16, (456u16, 789u16), 123u16);
/// let data = minicbor::to_vec(a).unwrap();
///
/// let (_, keeper, _): (u16, KeepRaw<(u16, u16)>, u16) = minicbor::decode(&data).unwrap();
/// let confirm: (u16, u16) = minicbor::decode(keeper.raw_cbor()).unwrap();
/// assert_eq!(confirm, (456u16, 789u16));
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct KeepRaw<'b, T> {
    raw: Cow<'b, [u8]>,
    inner: T,
}

impl<T> KeepRaw<'_, T> {
    pub fn raw_cbor(&self) -> &[u8] {
        &self.raw
    }

    pub fn unwrap(self) -> T {
        self.inner
    }

    pub fn clear_raw(&mut self) {
        self.raw = Cow::from(vec![]);
    }

    pub fn to_owned(self) -> KeepRaw<'static, T> {
        KeepRaw {
            raw: Cow::Owned(self.raw.into_owned()),
            inner: self.inner,
        }
    }
}

impl<T> Deref for KeepRaw<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for KeepRaw<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // If the inner value is mutated, we need to clear the raw bytes to
        // avoid returning stale data.
        self.clear_raw();

        &mut self.inner
    }
}

impl<T> From<T> for KeepRaw<'static, T> {
    /// Note that the `KeepRaw` value obtained from this implementation does
    /// **not** include a valid CBOR representation.
    fn from(val: T) -> Self {
        Self {
            raw: Cow::from(vec![]),
            inner: val,
        }
    }
}

impl<'b, T, C> minicbor::Decode<'b, C> for KeepRaw<'b, T>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let all = d.input();
        let start = d.position();
        let inner: T = d.decode_with(ctx)?;
        let end = d.position();

        Ok(Self {
            inner,
            raw: Cow::Borrowed(&all[start..end]),
        })
    }
}

impl<C, T> minicbor::Encode<C> for KeepRaw<'_, T>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        if self.raw_cbor().is_empty() {
            e.encode_with(&self.inner, ctx)?;
            Ok(())
        } else {
            e.writer_mut()
                .write_all(self.raw_cbor())
                .map_err(minicbor::encode::Error::write)
        }
    }
}

impl<T: Serialize> Serialize for KeepRaw<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.deref().serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for KeepRaw<'_, T> {
    /// Note that the `KeepRaw` value obtained from this implementation does
    /// **not** include a valid CBOR representation.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner: T = T::deserialize(deserializer)?;

        Ok(Self {
            inner,
            raw: Cow::from(vec![]),
        })
    }
}

/// Struct to hold arbitrary CBOR to be processed independently
///
/// # Examples
///
/// ```
/// use pallas_codec::utils::AnyCbor;
///
/// let a = (123u16, (456u16, 789u16), 123u16);
/// let data = minicbor::to_vec(a).unwrap();
///
/// let (_, any, _): (u16, AnyCbor, u16) = minicbor::decode(&data).unwrap();
/// let confirm: (u16, u16) = any.into_decode().unwrap();
/// assert_eq!(confirm, (456u16, 789u16));
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AnyCbor {
    inner: Vec<u8>,
}

impl AnyCbor {
    pub fn raw_bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn unwrap(self) -> Vec<u8> {
        self.inner
    }

    pub fn from_encode<T>(other: T) -> Self
    where
        T: Encode<()>,
    {
        let inner = minicbor::to_vec(other).unwrap();
        Self { inner }
    }

    pub fn into_decode<T>(self) -> Result<T, minicbor::decode::Error>
    where
        for<'b> T: Decode<'b, ()>,
    {
        minicbor::decode(&self.inner)
    }
}

impl Deref for AnyCbor {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'b, C> minicbor::Decode<'b, C> for AnyCbor {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let all = d.input();
        let start = d.position();
        d.skip()?;
        let end = d.position();

        Ok(Self {
            inner: Vec::from(&all[start..end]),
        })
    }
}

impl<C> minicbor::Encode<C> for AnyCbor {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.writer_mut()
            .write_all(self.raw_bytes())
            .map_err(minicbor::encode::Error::write)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(from = "Option::<T>", into = "Option::<T>")]
pub enum Nullable<T>
where
    T: std::clone::Clone,
{
    Some(T),
    Null,
    Undefined,
}

impl<T> Nullable<T>
where
    T: std::clone::Clone,
{
    pub fn map<F, O>(self, f: F) -> Nullable<O>
    where
        O: std::clone::Clone,
        F: Fn(T) -> O,
    {
        match self {
            Nullable::Some(x) => Nullable::Some(f(x)),
            Nullable::Null => Nullable::Null,
            Nullable::Undefined => Nullable::Undefined,
        }
    }

    pub fn as_ref(&self) -> Nullable<&T> {
        match self {
            Nullable::Some(x) => Nullable::Some(x),
            Nullable::Null => Nullable::Null,
            Nullable::Undefined => Nullable::Undefined,
        }
    }
}

impl<'b, C, T> minicbor::Decode<'b, C> for Nullable<T>
where
    T: minicbor::Decode<'b, C> + std::clone::Clone,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Null => {
                d.null()?;
                Ok(Self::Null)
            }
            minicbor::data::Type::Undefined => {
                d.undefined()?;
                Ok(Self::Undefined)
            }
            _ => {
                let x = d.decode_with(ctx)?;
                Ok(Self::Some(x))
            }
        }
    }
}

impl<C, T> minicbor::Encode<C> for Nullable<T>
where
    T: minicbor::Encode<C> + std::clone::Clone,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Nullable::Some(x) => {
                e.encode_with(x, ctx)?;
                Ok(())
            }
            Nullable::Null => {
                e.null()?;
                Ok(())
            }
            Nullable::Undefined => {
                e.undefined()?;
                Ok(())
            }
        }
    }
}

impl<T> From<Option<T>> for Nullable<T>
where
    T: std::clone::Clone,
{
    fn from(x: Option<T>) -> Self {
        match x {
            Some(x) => Nullable::Some(x),
            None => Nullable::Null,
        }
    }
}

impl<T> From<Nullable<T>> for Option<T>
where
    T: std::clone::Clone,
{
    fn from(other: Nullable<T>) -> Self {
        match other {
            Nullable::Some(x) => Some(x),
            _ => None,
        }
    }
}

#[derive(
    Serialize, Deserialize, Clone, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[cbor(transparent)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct Bytes(#[n(0)] minicbor::bytes::ByteVec);

impl From<Vec<u8>> for Bytes {
    fn from(xs: Vec<u8>) -> Self {
        Bytes(minicbor::bytes::ByteVec::from(xs))
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(b: Bytes) -> Self {
        b.0.into()
    }
}

impl Deref for Bytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<const N: usize> TryFrom<&Bytes> for [u8; N] {
    type Error = core::array::TryFromSliceError;

    fn try_from(value: &Bytes) -> Result<Self, Self::Error> {
        value.0.as_slice().try_into()
    }
}

impl TryFrom<String> for Bytes {
    type Error = hex::FromHexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let v = hex::decode(value)?;
        Ok(Bytes(minicbor::bytes::ByteVec::from(v)))
    }
}

impl FromStr for Bytes {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = hex::decode(s)?;
        Ok(Bytes(minicbor::bytes::ByteVec::from(v)))
    }
}

impl From<Bytes> for String {
    fn from(b: Bytes) -> Self {
        hex::encode(b.deref())
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes: Vec<u8> = self.clone().into();

        f.write_str(&hex::encode(bytes))
    }
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Encode, Decode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[cbor(transparent)]
#[serde(into = "i128")]
#[serde(try_from = "i128")]
pub struct Int(#[n(0)] pub minicbor::data::Int);

impl Deref for Int {
    type Target = minicbor::data::Int;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Int> for i128 {
    fn from(value: Int) -> Self {
        i128::from(value.0)
    }
}

impl From<i32> for Int {
    fn from(x: i32) -> Self {
        let inner = minicbor::data::Int::from(x);
        Self(inner)
    }
}

impl From<i64> for Int {
    fn from(x: i64) -> Self {
        let inner = minicbor::data::Int::from(x);
        Self(inner)
    }
}

impl TryFrom<i128> for Int {
    type Error = minicbor::data::TryFromIntError;

    fn try_from(value: i128) -> Result<Self, Self::Error> {
        let inner = minicbor::data::Int::try_from(value)?;
        Ok(Self(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_raw_retains_original() {
        // Indef array info is lost when decoded. By using KeepRaw, we can retain the
        // original bytes. This test makes sure KeepRaw is working by making use of this
        // well-known CBOR nuance.

        let raw = hex::decode("9F0102FF").unwrap();
        let subject: KeepRaw<'_, Vec<u32>> = minicbor::decode(&raw).unwrap();
        assert_eq!(subject.inner, vec![1, 2]);
        assert_eq!(subject.raw_cbor(), raw);
    }

    #[test]
    fn keep_raw_fallbacks_to_encode() {
        // By using the From trait we can encode the inner value directly without any
        // information about the original cbor bytes. By attempting to encode this
        // structure we ensure that KeepRaw is falling back to the expected encode
        // behavior.

        let subject = KeepRaw::from(vec![1, 2]);
        let encoded = minicbor::to_vec(&subject).unwrap();

        assert_eq!(encoded, hex::decode("820102").unwrap());
    }

    #[test]
    fn keep_raw_clears_original_when_mutated() {
        // If the inner value is mutated, we need to clear the raw bytes to
        // avoid returning stale data. This test starts from raw bytes, mutates the
        // value and then asserts that the returned cbor matches the updates.

        let raw = hex::decode("9F0102FF").unwrap();
        let mut subject: KeepRaw<'_, Vec<u32>> = minicbor::decode(&raw).unwrap();

        let inner = subject.deref_mut();
        inner.push(3);

        let encoded = minicbor::to_vec(&subject).unwrap();

        assert_eq!(subject.inner, vec![1, 2, 3]);
        assert_eq!(encoded, hex::decode("83010203").unwrap());
    }
}
