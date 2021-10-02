#![feature(default_free_fn)]
#![feature(const_size_of_val)]

#[macro_use]
mod lazy_fixed_iter;
mod board;
mod gamestate;
mod u32set;

use std::f32::consts::PI;

use board::Position;
use boolinator::Boolinator;

use itertools::Itertools;
use speedy2d::{
    color::Color,
    dimen::Vector2,
    window::{self, KeyScancode, MouseButton, VirtualKeyCode, WindowHandler, WindowHelper},
    Graphics2D, Window,
};

const BOARD_SIZE: u8 = 5;
type GameTree = gamestate::GameTree<BOARD_SIZE>;
type Board = board::Board<BOARD_SIZE>;

const MOUSE_LEFT_KEY: char = 'a';
const MOUSE_MIDDLE_KEY: char = 's';
const MOUSE_RIGHT_KEY: char = 'd';
const ESC_KEY: char = 'q';

const STONE_RADIUS: f32 = 30.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum State {
    SelectStart,
    Idle,
    PickUpStone(Position),
}
const INITIAL: State = State::SelectStart;

#[derive(Debug)]
struct MyWindowHandler {
    mouse: Vector2<f32>,
    keys: String,
    mods: Option<window::ModifiersState>,
    state: State,

    tree: GameTree,
    current: usize,
}

impl MyWindowHandler {
    fn new() -> Self {
        Self {
            mouse: Vector2::<f32>::ZERO,
            keys: String::new(),
            mods: Option::None,
            // tree: GameTree::start(Board::start(Position::new(0, 0))),
            tree: GameTree::start(Board::full()),
            current: 0,
            state: INITIAL,
        }
    }
}

impl WindowHandler for MyWindowHandler {
    fn on_start(&mut self, helper: &mut WindowHelper<()>, info: window::WindowStartupInfo) {}

    fn on_user_event(&mut self, helper: &mut WindowHelper<()>, user_event: ()) {}

    fn on_resize(&mut self, helper: &mut WindowHelper<()>, size_pixels: Vector2<u32>) {}

    fn on_scale_factor_changed(&mut self, helper: &mut WindowHelper<()>, scale_factor: f64) {}

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        let mods = self.mods.as_ref();
        let ctrl = mods.map(|m| m.ctrl()).unwrap_or(false);
        let alt = mods.map(|m| m.alt()).unwrap_or(false);
        let shift = mods.map(|m| m.shift()).unwrap_or(false);

        let node = self.tree.get(self.current).unwrap();
        // let prev = self.tree.get(board.parent);
        // let nexts = board.children.iter().flat_mmap(||)
        let board = node.board;

        let hover = board.iter().find(|&(pos, _)| {
            (self.mouse - stone_pos(pos)).magnitude_squared() < STONE_RADIUS * STONE_RADIUS
        });
        let hover_stone = hover.and_then(|(hover, at)| at.as_some(hover));

        // let solutions = self
        //     .tree
        //     .check_solvable(self.current)
        //     .map(|it| it.collect_vec())
        //     .unwrap_or(Vec::new());
        // let solvable = solutions.len() > 0;
        let num_solutions = node.num_solutions(&self.tree);
        let solvable = num_solutions > 0;
        graphics.clear_screen(if solvable { Color::WHITE } else { Color::GRAY });

        {
            // Process keypresses
            for key in self.keys.drain(..) {
                match key {
                    MOUSE_LEFT_KEY | MOUSE_RIGHT_KEY => {
                        match self.state {
                            State::SelectStart => {
                                if let Some(stone) = hover_stone {
                                    let new = board.filter(|&old| old != stone);
                                    // let new = new.canonicalize();
                                    self.current = self.tree.push(self.current, new).1;
                                    self.state = State::Idle;
                                }
                            }
                            State::Idle => {
                                if let Some(stone) = hover_stone {
                                    self.state = State::PickUpStone(stone)
                                }
                            }
                            State::PickUpStone(pickup) => {
                                if let Some((stone, _)) = hover {
                                    if stone == pickup {
                                        self.state = State::Idle;
                                    } else {
                                        // if let Some(new) = self.tree.apply_move(board, pickup, stone) {
                                        // self.current = new;
                                        if let Some(new) = board.apply_move(pickup, stone) {
                                            // if let Some(elim) = board.valid_move(pickup, stone) {
                                            //     let new = board
                                            //     .filter_map(|old| if old == pickup { Some(stone) } else if old == elim { None } else { Some(old) })
                                            //     .expect("Valid board because both pickup and stone are valid");

                                            // let new = new.canonicalize();
                                            self.current = self.tree.push(self.current, new).1;
                                            if key == MOUSE_LEFT_KEY {
                                                self.state = State::Idle;
                                            } else {
                                                self.state = State::PickUpStone(stone)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MOUSE_MIDDLE_KEY => {}
                    ESC_KEY => match self.state {
                        State::Idle => {}
                        State::PickUpStone(_) => self.state = State::Idle,
                        State::SelectStart => {}
                    },
                    // Undo
                    'u' => match self.state {
                        State::PickUpStone(_) => {
                            self.state = State::Idle;
                        }
                        State::Idle => {
                            if let Some((idx, _)) = self.tree.parent(self.current) {
                                {
                                    let refresh = self.tree.push(
                                        idx,
                                        self.tree.get(self.current).expect("Exists").board,
                                    );
                                    assert_eq!(refresh, (true, self.current)); // Push this state to the "top of the redo stack"
                                }
                                self.current = idx;
                                if self.current == 0 {
                                    self.state = State::SelectStart;
                                }
                            } // else already at origin probably
                        }
                        State::SelectStart => {}
                    },
                    // Redo
                    'r' => match self.state {
                        State::PickUpStone(_) => {
                            self.state = State::Idle;
                        }
                        State::SelectStart | State::Idle => {
                            if let Some(children) = self.tree.children(self.current) {
                                if let Some((idx, _)) = children.last() {
                                    self.current = idx;
                                    if self.current != 0 {
                                        self.state = State::Idle;
                                    }
                                }
                            } // else already at origin probably
                        }
                    },
                    // Sidedo
                    'o' => match self.state {
                        State::PickUpStone(_) => {
                            self.state = State::Idle;
                        }
                        State::SelectStart => {}
                        State::Idle => {
                            let tree = &self.tree;
                            let i = self.current;
                            if let Some(idx) = (|| {
                                let ch = tree.children_indices(tree.parent(i)?.0)?;
                                let (i, _) = ch.iter().copied().find_position(|&ch| ch == i)?;
                                ch.get((i + 1) % ch.len()).copied()
                            })() {
                                self.current = idx;
                            }
                        }
                    },
                    'm' => match self.state {
                        State::PickUpStone(_) => {
                            self.state = State::Idle;
                        }
                        State::SelectStart => {}
                        State::Idle => {
                            let succs = self.tree.explore(self.current);
                            if let Some(&fin) = succs.first() {
                                self.current = fin;
                            }
                        }
                    },
                    'c' => println!("{}", num_solutions),
                    _ => {}
                }
            }
        }

        for pos in Board::iter_all() {
            let stone = stone_pos(pos);
            let hover = hover.map(|(s, _)| s == pos).unwrap_or(false);
            let empty = !board.at(pos).expect("Must be valid position");
            let (pickup, drop) = match self.state {
                State::PickUpStone(pickup) => {
                    (pickup == pos, board.valid_move(pickup, pos).is_some())
                }
                _ => (false, false),
            };
            graphics.draw_circle(
                stone,
                STONE_RADIUS,
                if pickup {
                    Color::YELLOW
                } else if hover {
                    if drop {
                        Color::GREEN
                    } else if !empty {
                        match self.state {
                            State::SelectStart | State::Idle => Color::GREEN,
                            State::PickUpStone(_) => Color::RED,
                        }
                    } else {
                        Color::from_rgb(0.8, 0.9, 1.0)
                    }
                } else if empty {
                    Color::from_rgb(0.8, 0.9, 1.0)
                } else {
                    if pos.x % 2 == 0 && pos.y % 2 == 0 {
                        Color::from_rgb(1.0, 0.4023, 0.7)
                    } else {
                        Color::BLUE
                    }
                },
            );
        }

        // Helpful mode
        for (from, over, to) in board.all_valid_moves() {
            if let State::PickUpStone(pickup) = self.state {
                if pickup != from {
                    continue;
                }
            }
            let fromp = stone_pos(from);
            let overp = stone_pos(over);
            let top = stone_pos(to);
            graphics.draw_circle(fromp, STONE_RADIUS * 3. / 5., Color::YELLOW);
            graphics.draw_circle(fromp, STONE_RADIUS / 3., Color::TRANSPARENT);
            graphics.draw_circle(overp, STONE_RADIUS / 3., Color::CYAN);
            graphics.draw_circle(top, STONE_RADIUS / 2., Color::GREEN);
        }

        graphics.draw_circle(
            self.mouse,
            10.,
            ctrl.as_some(Color::from_rgb(0.8, 0.9, 1.0))
                .unwrap_or(Color::BLUE),
        );

        helper.request_redraw();
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper<()>, position: Vector2<f32>) {
        self.mouse = position;

        helper.request_redraw();
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper<()>, button: MouseButton) {
        match button {
            MouseButton::Left => self.keys.push(MOUSE_LEFT_KEY),
            MouseButton::Middle => self.keys.push(MOUSE_MIDDLE_KEY),
            MouseButton::Right => self.keys.push(MOUSE_RIGHT_KEY),
            MouseButton::Other(_) => {}
        };
        helper.request_redraw();
    }

    fn on_mouse_button_up(&mut self, helper: &mut WindowHelper<()>, _button: MouseButton) {
        helper.request_redraw()
    }

    fn on_key_down(
        &mut self,
        helper: &mut WindowHelper<()>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode,
    ) {
    }

    fn on_key_up(
        &mut self,
        helper: &mut WindowHelper<()>,
        virtual_key_code: Option<VirtualKeyCode>,
        scancode: KeyScancode,
    ) {
    }

    fn on_keyboard_char(&mut self, helper: &mut WindowHelper<()>, unicode_codepoint: char) {
        self.keys.push(unicode_codepoint);
        helper.request_redraw();
    }

    fn on_keyboard_modifiers_changed(
        &mut self,
        helper: &mut WindowHelper<()>,
        state: window::ModifiersState,
    ) {
        self.mods = Some(state);
        helper.request_redraw();
    }
}

fn stone_pos(pos: Position) -> Vector2<f32> {
    let x = pos.x as f32;
    let y = pos.y as f32;
    let stone = Vector2::new(
        x * STONE_RADIUS * 4. + STONE_RADIUS * 2. + y * STONE_RADIUS * 2.,
        STONE_RADIUS * 2. + y * STONE_RADIUS * 4. * f32::cos(PI / 6.),
    );
    stone
}

fn main() {
    let window = Window::new_centered("Title", (640, 480)).unwrap();

    window.run_loop(MyWindowHandler::new())
}
