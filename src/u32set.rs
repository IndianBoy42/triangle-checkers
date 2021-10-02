use boolinator::Boolinator;
use core::iter::FromIterator;
use itertools::unfold;
use num_traits::Bounded;
use std::{
    fmt::{Binary, Debug},
    mem::{self, size_of},
    ops::{
        BitAnd, BitAndAssign, BitOr, BitOrAssign, Range, RangeFrom, RangeTo, Shl, Shr, ShrAssign,
    },
};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedBitSet<T> {
    pub val: T,
}
impl<T> Debug for FixedBitSet<T>
where
    T: Binary,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:b}", self.val)
    }
}
pub type U32Set = FixedBitSet<u32>;
pub type U64Set = FixedBitSet<u64>;
impl<T> FixedBitSet<T> {
    pub fn new(val: T) -> FixedBitSet<T> {
        FixedBitSet { val }
    }
    pub fn get<I>(&self, i: I) -> Option<<I as SliceIndexBitSet<T>>::Output>
    where
        I: SliceIndexBitSet<T>,
    {
        i.get(self)
    }
    pub fn intersect_with(&mut self, other: FixedBitSet<T>)
    where
        T: BitAndAssign<T>,
    {
        self.val &= other.val;
    }
    pub fn intersect(self, other: FixedBitSet<T>) -> FixedBitSet<T>
    where
        T: BitAnd<T, Output = T>,
    {
        FixedBitSet {
            val: self.val & other.val,
        }
    }
    pub fn union_with(&mut self, other: FixedBitSet<T>)
    where
        T: BitOrAssign<T>,
    {
        self.val |= other.val;
    }
    pub fn union(self, other: FixedBitSet<T>) -> FixedBitSet<T>
    where
        T: BitOr<T, Output = T>,
    {
        FixedBitSet {
            val: self.val | other.val,
        }
    }
    pub fn len(self) -> usize
    where
        T: CountOnes,
    {
        self.val.count_ones() as _
    }
    pub fn is_empty(self) -> bool
    where
        T: CountOnes,
    {
        self.len() == 0
    }
    /// Reverse the first `i` bits
    pub fn revn(self, i: usize) -> FixedBitSet<T>
    where
        T: ReverseBits,
        T: Shr<usize, Output = T>,
    {
        Self::new(self.val.reverse_bits() >> (32 - i))
    }
    pub fn rev(self) -> FixedBitSet<T>
    where
        T: ReverseBits,
    {
        Self::new(self.val.reverse_bits())
    }

    /// Infinite iterator of bits (after the last bit keeps returning false)
    pub fn iter_bits(self) -> impl Iterator<Item = bool>
    where
        for<'a> &'a T: BitAnd<&'a T, Output = T>,
        T: From<u8>,
        T: ShrAssign<T>,
        T: PartialEq,
    {
        unfold(self.val, |st| {
            let bit = (&*st & &0b1.into()) != 0.into();
            *st >>= 1.into();
            Some(bit)
        })
    }
    pub fn iter_pos(self) -> impl Iterator<Item = usize>
    where
        for<'a> &'a T: BitAnd<&'a T, Output = T>,
        T: From<u8>,
        T: ShrAssign<T>,
        T: PartialEq,
    {
        self.iter_bits()
            .take(mem::size_of::<T>() * 8)
            .enumerate()
            .filter_map(|(i, bit)| if bit { Some(i) } else { None })
    }
}

impl<T> FromIterator<usize> for FixedBitSet<T>
where
    bool: Into<T>,
    u8: Into<T>,
    T: BitOr<T, Output = T>,
    T: Shl<usize, Output = T>,
{
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        FixedBitSet {
            val: iter
                .into_iter()
                .fold(0.into(), |acc, bit| acc | (1.into() << bit)),
        }
    }
}

impl<T> FromIterator<bool> for FixedBitSet<T>
where
    // bool: Into<T>,
    // i8: Into<T>,
    // T: Shl<Output = T> + BitOr<Output = T>,
    bool: Into<T>,
    u8: Into<T>,
    T: BitOr<T, Output = T>,
    T: Shl<usize, Output = T>,
{
    fn from_iter<I: IntoIterator<Item = bool>>(iter: I) -> Self {
        // Convert to iterator of positions
        iter.into_iter()
            .enumerate()
            .filter(|&(_, x)| x)
            .map(|(i, _)| i)
            .collect()
        // USet {
        //     val: iter
        //         .into_iter()
        //         .fold(0.into(), |acc, bit| (acc << 1.into()) | bit.into()),
        // }
    }
}

pub trait CountOnes {
    fn count_ones(&self) -> usize;
}
macro_rules! count_ones_impl {
    ($T:ty) => {
        impl CountOnes for $T {
            fn count_ones(&self) -> usize {
                <$T>::count_ones(*self) as _
            }
        }
    };
}
count_ones_impl!(u32);
count_ones_impl!(u16);
count_ones_impl!(u8);
count_ones_impl!(u64);
count_ones_impl!(usize);

pub trait ReverseBits {
    fn reverse_bits(&self) -> Self;
}
macro_rules! reverse_bits_impl {
    ($T:ty) => {
        impl ReverseBits for $T {
            fn reverse_bits(&self) -> $T {
                <$T>::reverse_bits(*self) as _
            }
        }
    };
}
reverse_bits_impl!(u32);
reverse_bits_impl!(u16);
reverse_bits_impl!(u8);
reverse_bits_impl!(u64);
reverse_bits_impl!(usize);

pub trait SliceIndexBitSet<T> {
    type Output;
    fn get(self, slice: &FixedBitSet<T>) -> Option<Self::Output>;
    unsafe fn get_unchecked(self, slice: *const FixedBitSet<T>) -> Self::Output;
    fn index(self, slice: &FixedBitSet<T>) -> Self::Output;
}
impl<T> SliceIndexBitSet<T> for usize
where
    T: BitAnd<T, Output = T> + From<u8> + PartialEq + Copy + Shl<usize, Output = T>,
{
    type Output = bool;

    fn get(self, slice: &FixedBitSet<T>) -> Option<bool> {
        (self <= size_of::<T>() * 8)
            .as_some_from(|| unsafe { self.get_unchecked(slice as *const _) })
    }

    unsafe fn get_unchecked(self, slice: *const FixedBitSet<T>) -> bool {
        ((*slice).val & (T::from(0b1) << self)) != 0.into()
    }

    fn index(self, slice: &FixedBitSet<T>) -> bool {
        self.get(slice).unwrap()
    }
}
impl<T> SliceIndexBitSet<T> for Range<usize>
where
    T: BitAnd<T, Output = T> + Shr<usize, Output = T> + From<u8> + Copy + Bounded,
{
    type Output = FixedBitSet<T>;
    fn get(self, slice: &FixedBitSet<T>) -> Option<FixedBitSet<T>> {
        (self.end <= size_of::<T>() * 8 && self.start <= self.end)
            .as_some_from(|| unsafe { self.get_unchecked(slice as *const _) })
    }

    unsafe fn get_unchecked(self, slice: *const FixedBitSet<T>) -> FixedBitSet<T> {
        (self.start..).get_unchecked(&(..self.end).get_unchecked(slice) as *const _)
        // let mask: T = if self.end >= size_of::<T>() * 8 {
        //     (!0).into()
        // } else {
        //     T::from(1) << self.end - 1
        // };
        // FixedBitSet {
        //     val: ((*slice).val & mask) >> self.start,
        // }
    }

    fn index(self, slice: &FixedBitSet<T>) -> FixedBitSet<T> {
        self.get(slice).unwrap()
    }
}

impl<T> SliceIndexBitSet<T> for RangeFrom<usize>
where
    T: BitAnd<T, Output = T> + Shr<usize, Output = T>,
    T: From<u8>,
    T: Copy,
{
    type Output = FixedBitSet<T>;
    fn get(self, slice: &FixedBitSet<T>) -> Option<FixedBitSet<T>> {
        unsafe { Some(self.get_unchecked(slice as *const _)) }
    }

    unsafe fn get_unchecked(self, slice: *const FixedBitSet<T>) -> FixedBitSet<T> {
        FixedBitSet {
            val: (*slice).val >> self.start,
        }
    }

    fn index(self, slice: &FixedBitSet<T>) -> FixedBitSet<T> {
        self.get(slice).unwrap()
    }
}
impl<T> SliceIndexBitSet<T> for RangeTo<usize>
where
    T: BitAnd<T, Output = T> + From<u8> + std::ops::Shr<usize, Output = T> + Copy + Bounded,
{
    type Output = FixedBitSet<T>;
    fn get(self, slice: &FixedBitSet<T>) -> Option<FixedBitSet<T>> {
        (self.end <= size_of::<T>() * 8)
            .as_some_from(|| unsafe { self.get_unchecked(slice as *const _) })
    }

    unsafe fn get_unchecked(self, slice: *const FixedBitSet<T>) -> FixedBitSet<T> {
        let mask = T::max_value() >> (size_of::<T>() * 8 - self.end);
        // let mask: T = if self.end >= size_of::<T>() * 8 {
        //     (!0).into()
        // } else {
        //     (T::from(1) << self.end) - 1
        // };
        FixedBitSet {
            val: (*slice).val & mask,
        }
    }

    fn index(self, slice: &FixedBitSet<T>) -> FixedBitSet<T> {
        self.get(slice).unwrap()
    }
}
// TODO: other Range types
