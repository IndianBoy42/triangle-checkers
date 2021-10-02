use std::{collections::VecDeque, default::default};

use crate::board::{Board, Position};
use boolinator::Boolinator;
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTreeNode<const SIZE: u8> {
    // State of the board
    pub board: Board<SIZE>,
    /// Index of parent state
    /// TODO: make into a SmallVec
    pub parent: usize,
    /// Children states,
    pub children: SmallVec<[usize; 2]>,
}

impl<const SIZE: u8> GameTreeNode<SIZE> {
    pub fn parent<'a>(&'a self, p: &'a GameTree<SIZE>) -> Option<&'a GameTreeNode<SIZE>> {
        p.states.get(self.parent)
    }
    pub fn children<'a>(
        &'a self,
        p: &'a GameTree<SIZE>,
    ) -> impl Iterator<Item = &'a GameTreeNode<SIZE>> + 'a {
        self.children.iter().filter_map(move |&i| p.states.get(i))
    }
    pub fn solvable<'a>(&'a self, p: &'a GameTree<SIZE>) -> impl Iterator<Item = usize> + 'a {
        self.children
            .iter()
            .copied()
            .filter(move |&ch| {
                let ch = p.get(ch).expect("Child exists");
                // ch.board.count() == 1 || ch.solvable(p).next().is_some()
                ch.solvable(p).next().is_some()
            })
            .chain((self.board.count() == 1).as_some(usize::MAX))
    }
    pub fn num_solutions<'a>(&'a self, p: &'a GameTree<SIZE>) -> usize {
        if self.board.count() == 1 {
            1
        } else {
            self.children
                .iter()
                .copied()
                .map(move |ch| {
                    let ch = p.get(ch).expect("Child exists");
                    // ch.board.count() == 1 || ch.solvable(p).next().is_some()
                    ch.num_solutions(p)
                })
                .sum::<usize>()
        }
    }
}

#[derive(Debug)]
pub struct GameTree<const SIZE: u8> {
    /// Tree of game states (maybe a DAG in the future)
    states: Vec<GameTreeNode<SIZE>>,
}

impl<const SIZE: u8> Default for GameTree<SIZE> {
    fn default() -> Self {
        Self { states: Vec::new() }
    }
}

impl<const SIZE: u8> GameTree<SIZE> {
    // pub type Board = board::Board<SIZE>;
    pub fn start(board: Board<SIZE>) -> GameTree<SIZE> {
        Self {
            states: vec![GameTreeNode {
                board,
                parent: usize::MAX,
                children: smallvec![],
            }],
        }
    }
    pub fn push(&mut self, after: usize, board: Board<SIZE>) -> (bool, usize) {
        // TODO: canonicalize here?
        let children = self.states[after].children.iter();
        if let Some((existingch, node)) = children
            .copied()
            .find_position(|&child| self.states[child].board == board)
        {
            self.states[node].parent = after;
            let ch = &mut self.states[after].children;
            let last = ch.len() - 1;
            ch.swap(existingch, last);
            (true, node)
        } else {
            let (ex, idx) = if let Some((idx, node)) = self
                .states
                .iter_mut()
                .find_position(|search| search.board == board)
            {
                node.parent = after;
                (true, idx)
            } else {
                self.states.push(GameTreeNode {
                    board,
                    parent: after,
                    children: default(),
                });
                (false, self.states.len() - 1)
            };
            self.states[after].children.push(idx);
            (ex, idx)
        }
    }
    pub fn get(&self, i: usize) -> Option<&GameTreeNode<SIZE>> {
        self.states.get(i)
    }
    pub fn get_mut(&mut self, i: usize) -> Option<&mut GameTreeNode<SIZE>> {
        self.states.get_mut(i)
    }
    pub fn parent(&self, i: usize) -> Option<(usize, &GameTreeNode<SIZE>)> {
        let ch = self.states.get(i)?;
        ch.parent(self).map(|x| (ch.parent, x))
    }
    pub fn children_indices(&self, i: usize) -> Option<&SmallVec<[usize; 2]>> {
        Some(&self.states.get(i)?.children)
    }
    pub fn children(&self, i: usize) -> Option<impl Iterator<Item = (usize, &GameTreeNode<SIZE>)>> {
        let ch = self.states.get(i)?;
        Some(ch.children.iter().copied().zip(ch.children(self)))
    }

    pub fn apply_move(
        &mut self,
        boardi: usize,
        from: Position,
        to: Position,
    ) -> Option<(bool, usize, Board<SIZE>)> {
        let board = self.get(boardi)?.board.apply_move(from, to)?;
        let (new, idx) = self.push(boardi, board);
        Some((new, idx, board))
    }

    pub fn explore(&mut self, from: usize) -> Vec<usize> {
        let mut qu = VecDeque::with_capacity(1024);
        let mut succ = Vec::with_capacity(256);
        qu.push_back(from);

        while let Some(brd_idx) = qu.pop_back() {
            let board = self.get(brd_idx).unwrap().board;

            if board.count() == 1 {
                succ.push(brd_idx);
            }

            for (from, over, to) in board.all_valid_moves() {
                // TODO: optimization: no need to recheck validity
                let (existing, idx, board) = self
                    .apply_move(brd_idx, from, to)
                    .expect("Valid by construction");
                if !existing {
                    qu.push_back(idx);
                }
            }
        }

        succ
    }

    pub fn check_solvable(&self, from: usize) -> Option<impl Iterator<Item = usize> + '_> {
        Some(self.get(from)?.solvable(self))
    }
}
