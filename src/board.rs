use boolinator::Boolinator;
use itertools::{izip, Itertools};
use std::{
    convert::{TryFrom, TryInto},
    iter::{self, FromIterator},
    mem::size_of_val,
    num::TryFromIntError,
};

use crate::u32set::FixedBitSet;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

impl Position {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
    pub fn from((x, y): (u8, u8)) -> Self {
        Position { x, y }
    }
}
pub fn pos(x: u8, y: u8) -> Position {
    Position::new(x, y)
}
// impl From<(u8, u8)> for Position {
//     fn from((x, y): (u8, u8)) -> Self {
//         Position { x, y }
//     }
// }
impl<T, U> From<Position> for (T, U)
where
    T: From<u8>,
    U: From<u8>,
{
    fn from(p: Position) -> Self {
        (p.x.into(), p.y.into())
    }
}
impl<T, U> TryFrom<(T, U)> for Position
where
    T: TryInto<u8, Error = TryFromIntError>,
    U: TryInto<u8, Error = TryFromIntError>,
{
    type Error = TryFromIntError;

    fn try_from((x, y): (T, U)) -> Result<Self, TryFromIntError> {
        Ok(Position {
            x: x.try_into()?,
            y: y.try_into()?,
        })
    }
}

pub type BitSetType = u64;
pub type BitSet = FixedBitSet<BitSetType>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Board<const SIZE: u8> {
    dots: BitSet,
}

impl<const SIZE: u8> Board<SIZE> {
    // Initialize a board with an empty spot at `e`
    pub fn full() -> Self {
        Board {
            dots: Self::iter_all()
                .map(|p| Self::get_idx(p).expect("Inbounds by construction"))
                .collect(),
        }
    }
    pub fn start(e: Position) -> Self {
        Board {
            dots: Self::iter_all()
                .filter(|&p: &Position| p != e)
                .map(|p| Self::get_idx(p).expect("Inbounds by construction"))
                .collect(),
        }
    }
    pub fn row_len(y: u8) -> u8 {
        SIZE - y
    }
    pub fn at(&self, p: Position) -> Option<bool> {
        Some(
            self.at_index(Self::get_idx(p)?)
                .expect("Index already bounds checked"),
        )
    }
    fn valid_pos(p: Position) -> bool {
        p.y < SIZE && p.x < Self::row_len(p.y)
    }
    // Round up to power two so that mul becomes shl and mod becomes bitand, uses more memory
    const ROW_OFFSET: usize = 1 << (size_of_val(&SIZE) * 8 - SIZE.leading_zeros() as usize);
    fn get_idx(p: Position) -> Option<usize> {
        Self::valid_pos(p).as_some((p.x as usize) | ((p.y as usize) * Self::ROW_OFFSET))
    }
    fn get_pos(i: impl TryInto<u16>) -> Option<Position> {
        let i = i.try_into().ok()?;
        let r: u16 = Self::ROW_OFFSET.try_into().expect("Statically known");

        let p = Position::try_from((i % r, i / r)).ok()?;
        Self::valid_pos(p).as_some(p)
    }
    pub fn row(&self, y: u8) -> Option<BitSet> {
        (y < SIZE).as_some(()).and_then(|()| {
            Some(
                self.dots
                    .get(Self::get_idx(pos(0, y)).expect("Inbounds by construction")..)
                    .expect("Bounds checked 1")
                    .get(..Self::row_len(y) as usize)
                    .expect("Bounds checked 2"),
            )
        })
    }
    fn at_index(&self, i: usize) -> Option<bool> {
        self.dots.get(i)
    }

    fn neighbours(p: Position, dist: u8) -> impl Iterator<Item = Option<Position>> {
        lazy_fixed_iter![
            move || Some((p.x.checked_add(dist)?, p.y)),
            Some((p.x, p.y.checked_add(dist)?)),
            Some((p.x.checked_sub(dist)?, p.y.checked_add(dist)?)),
            Some((p.x.checked_sub(dist)?, p.y)),
            Some((p.x, p.y.checked_sub(dist)?)),
            Some((p.x.checked_add(dist)?, p.y.checked_sub(dist)?))
        ]
        .map(|p| p.map(Position::from))
    }

    pub fn valid_moves(
        &self,
        p: Position,
    ) -> Option<impl Iterator<Item = (Position, Position)> + '_> {
        self.at(p)?.as_some_from(move || {
            izip!(Self::neighbours(p, 1), Self::neighbours(p, 2))
                .filter_map(|(e, t)| e.zip(t))
                .filter(move |&(e, t)| !self.at(t).unwrap_or(true) && self.at(e).unwrap_or(false))
        })
    }

    // from, over, to
    pub fn all_valid_moves(&self) -> impl Iterator<Item = (Position, Position, Position)> + '_ {
        self.iter_stones().flat_map(move |pos| {
            self.valid_moves(pos)
                .expect("pos must have a stone as it is from iter_stones")
                .map(move |(e, t)| (pos, e, t))
        })
    }

    /// Some(Position) means this is a valid move that would eliminate the returned stone
    /// None means invalid move for some reason
    pub fn valid_move(&self, fr: Position, to: Position) -> Option<Position> {
        if !self.at(fr)? {
            // Stone at the origin
            None
        } else if self.at(to)? {
            // Space at the ending
            None
        } else if let Some((i, _)) = Self::neighbours(fr, 2)
            .into_iter()
            .find_position(|&x| x.map(|x| x == to).unwrap_or(false))
        {
            // Stone to jump over
            // TODO: check that this optimizes well
            let p = Self::neighbours(fr, 1)
                .nth(i)
                .expect("This index is from an Iterator::find_position")
                .expect("If the 2 away position is valid then this should be valid");
            self.at(p).and_then(|b| b.as_some(p))
        } else {
            None
        }
    }

    pub fn iter_all() -> impl Iterator<Item = Position> {
        (0..SIZE).flat_map(|y| (0..Self::row_len(y)).map(move |x| pos(x, y)))
    }
    pub fn iter_stones(&self) -> impl Iterator<Item = Position> {
        self.dots.iter_pos().map(Self::get_pos).flatten()
        // .map(|x| x.expect("Must be a valid position"))
    }
    pub fn iter(&self) -> impl Iterator<Item = (Position, bool)> + '_ {
        Self::iter_all().map(move |p| (p, self.at(p).expect("Must be valid")))
    }

    pub fn map(&self, map: impl FnMut(Position) -> Position) -> Option<Board<SIZE>> {
        self.iter_stones().map(map).collect()
    }
    pub fn filter_map(&self, map: impl FnMut(Position) -> Option<Position>) -> Option<Board<SIZE>> {
        self.iter_stones().filter_map(map).collect()
    }
    pub fn filter(&self, p: impl FnMut(&Position) -> bool) -> Board<SIZE> {
        self.iter_stones().filter(p).collect()
    }
    fn flip(&self) -> Self {
        // self.iter_pos().map(|p| pos(SIZE - 1 - p.x, p.y)).collect()
        let mut dots: BitSetType = 0;
        for row in 0..SIZE {
            let rowbits = self.row(row as u8).expect("By definition");
            dots |= rowbits.revn(Self::row_len(row as u8).into()).val
                << Self::get_idx(pos(0, row as u8)).expect("Inbounds by construction");
        }
        Board {
            dots: BitSet::new(dots),
        }
    }
    fn rotate_right(&self) -> Self {
        self.iter_stones()
            .map(|p| pos(p.y, SIZE - 1 - p.x - p.y))
            .collect()
    }
    fn rotate_left(&self) -> Self {
        self.iter_stones()
            .map(|p| pos(SIZE - 1 - p.y - p.x, p.x))
            .collect()
    }
    fn rotate_right_flip(&self) -> Self {
        self.iter_stones()
            .map(|p| pos(p.y, SIZE - 1 - p.y))
            .map(|p| pos(SIZE - 1 - p.x, p.y))
            .collect()
    }
    fn rotate_left_flip(&self) -> Self {
        self.iter_stones()
            .map(|p| pos(SIZE - 1 - p.y, p.x))
            .map(|p| pos(SIZE - 1 - p.x, p.y))
            .collect()
    }
    fn all_variants(&self) -> impl Iterator<Item = Board<SIZE>> {
        let base = *self;
        let r = self.rotate_right();
        let l = self.rotate_left();
        lazy_fixed_iter![move || base, base.flip(), r, l, r.flip(), l.flip()]
    }
    pub fn canonicalize(&self) -> Self {
        // The chose of canonical value is kinda arbitrary, maybe a more relevant method would be better
        // But this should be pretty fast
        self.all_variants()
            .min_by_key(|board| board.dots.val)
            .expect("Array is non empty")
    }

    pub fn apply_move(&self, from: Position, to: Position) -> Option<Self> {
        let elim = self.valid_move(from, to)?;
        Some(
            self.filter_map(|old| {
                if old == from {
                    Some(to)
                } else if old == elim {
                    None
                } else {
                    Some(old)
                }
            })
            .expect("Valid board because both pickup and stone are valid"),
        )
    }

    pub fn count(&self) -> usize {
        self.dots.len()
    }
}

impl<const SIZE: u8> FromIterator<Position> for Board<SIZE> {
    fn from_iter<T: IntoIterator<Item = Position>>(iter: T) -> Self {
        Option::<Board<SIZE>>::from_iter(iter).unwrap()
    }
}

impl<const SIZE: u8> FromIterator<Position> for Option<Board<SIZE>> {
    fn from_iter<T: IntoIterator<Item = Position>>(iter: T) -> Self {
        Some(Board {
            dots: iter
                .into_iter()
                .map(Board::<SIZE>::get_idx)
                .collect::<Option<_>>()?,
        })
    }
}
