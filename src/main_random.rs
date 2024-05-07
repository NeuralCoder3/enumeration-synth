use itertools::Itertools;
use rand::seq::SliceRandom;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator as _;
use serde::de::Visitor;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt::Display;
use std::ops::Range;
// has largest value at the top
use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::cmp::Reverse;
use std::io::Write;
use std::rc::Rc;

const NUMBERS: usize = 3;
const MAX_LEN: u8 = 11;
// const NUMBERS: usize = 4;
// const MAX_LEN: u8 = 20;
// const MAX_LEN: u8 = 19; // impossible
// const NUMBERS: usize = 5;
// const MAX_LEN: u8 = 33;
const SWAPS: usize = 1;
// const NUMBERS: usize = 6;
// const MAX_LEN: u8 = 45;
// const SWAPS: usize = 2; // increases perm states from 80640 to 1330560
// https://github.com/google-deepmind/alphadev/blob/main/sort_functions_test.cc
const REGS: usize = NUMBERS + SWAPS;
const CMP: usize = 0;
const MOV: usize = 1;
const CMOVG: usize = 2;
const CMOVL: usize = 3;
const NUMBERS_U8: u8 = NUMBERS as u8;

type Command = (usize, usize, usize);
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Copy)]
struct Permutation([u8; REGS + 2]);
type State = Vec<Permutation>;

use std::ops::{Index, IndexMut};

impl Index<usize> for Permutation {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Permutation {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Index<Range<usize>> for Permutation {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<Range<usize>> for Permutation {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self.0[index]
    }
}

// serialize, display for state
impl std::fmt::Display for Permutation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", &self.0)
    }
}

fn possible_commands() -> Vec<Command> {
    let mut commands = vec![];
    for instr in &[MOV, CMOVG, CMOVL] {
        for to in 0..REGS {
            for from in 0..REGS {
                if to != from {
                    commands.push((*instr, to, from));
                }
            }
        }
    }
    for i in 0..REGS {
        for j in (i + 1)..REGS {
            commands.push((CMP, i, j));
        }
    }
    commands
}

// transform a permutation according to a command
fn apply(cmd: &Command, perm: &mut Permutation) {
    let (instr, to, from) = *cmd;
    match instr {
        CMP => {
            perm[REGS + 0] = (perm[to] < perm[from]) as u8;
            perm[REGS + 1] = (perm[to] > perm[from]) as u8;
        }
        MOV => perm[to] = perm[from],
        CMOVG => {
            if perm[REGS + 1] == 1 {
                perm[to] = perm[from];
            }
        }
        CMOVL => {
            if perm[REGS + 0] == 1 {
                perm[to] = perm[from];
            }
        }
        _ => panic!("Unknown instruction"),
    }
}

// map a command over all permutations in a state
fn apply_all(cmd: &Command, state: &State) -> State {
    let mut new_state = Vec::new();
    for perm in state {
        let mut new_perm = perm.clone();
        apply(cmd, &mut new_perm);
        new_state.push(new_perm);
    }
    new_state.sort();
    new_state.dedup();
    new_state
}

// check if the state can never reach a solution
// corresponds to delete-relaxed planning check
fn viable(state: &State) -> bool {
    for perm in state {
        for n in 1..=NUMBERS_U8 {
            if !perm[0..REGS].contains(&n) {
                return false;
            }
        }
    }
    true
}

fn show_command(cmd: &Command) -> String {
    let (instr, to, from) = *cmd;
    // 1-indexed to stay consistent with minizinc
    let to = to + 1;
    let from = from + 1;
    match instr {
        CMP => format!("CMP {} {}", to, from),
        MOV => format!("MOV {} {}", to, from),
        CMOVG => format!("CMOVG {} {}", to, from),
        CMOVL => format!("CMOVL {} {}", to, from),
        _ => panic!("Unknown instruction"),
    }
}

// linked list to store the commands and pointer to last element
// TODO: https://rust-unofficial.github.io/too-many-lists/
// https://rust-unofficial.github.io/too-many-lists/second-option.html
// for how to correctly implement a linked list stack
#[derive(Clone, Eq, PartialEq, Hash)]
struct Node {
    cmd: Command,
    prev: Option<Box<Node>>,
}

// for each permutation, take out register values, concat => serializable byte array
// we could use perm_positions for more informed hashing/equality check
fn state_positions(state: &State) -> Vec<u8> {
    state.iter().flat_map(|p| p.0).collect()
}

fn main() {
    let possible_cmds = possible_commands();
    let permutations: Vec<Vec<u8>> = (1..=NUMBERS_U8).permutations(NUMBERS).collect();
    let init_perm_count = permutations.len();

    // let perm_count = 6;
    // let permutations = permutations.choose_multiple(&mut rand::thread_rng(), perm_count).cloned().collect::<Vec<_>>();

    // extend numerical permutations with register for swap and flags
    // we use RC to avoid cloning the state
    let initial_state: Rc<State> = Rc::new(
        permutations
            .iter()
            .map(|p| {
                let mut perm = Permutation([0; REGS + 2]);
                for (i, &x) in p.iter().enumerate() {
                    perm[i] = x;
                }
                perm
            })
            .collect(),
    );

    let node0 = Node {
        cmd: (0, 0, 0),
        prev: None,
    };

    // let mut queue = PriorityQueue::new();
    // queue.push((node0,Rc::clone(&initial_state),0 as u8), Reverse(0));

    let start = std::time::Instant::now();

    // let mut rng = rand::thread_rng();
    // recursively apply all commands to the state in random order
    // until we reach a solution or the maximum length or not viable

    // let mut visited : u64 = 0;

    println!("{:?}", (1..=NUMBERS_U8).collect::<Vec<_>>());

    let tmp_dir = std::env::var("_CONDOR_SCRATCH_DIR").unwrap_or("/tmp".to_string());
    let mut i = 0;
    let mut path = format!("{}/sled-map{}", tmp_dir, i);
    while std::path::Path::new(&path).exists() {
        i += 1;
        path = format!("{}/sled-map{}", tmp_dir, i);
    }
    println!("Using sled map: {}", path);
    let length_map = sled::open(path).unwrap();
    length_map.insert(state_positions(&initial_state), vec![0 as u8]).unwrap();

    let commands = possible_commands();

    // completely random playouts
    let mut visited : u64 = 0;
    while true {
        // let mut cmds = vec![];
        // for _ in 0..MAX_LEN {
        //     cmds.push(commands.choose(&mut rand::thread_rng()).unwrap());
        // }
        let cmds = (0..MAX_LEN).map(|_| commands.choose(&mut rand::thread_rng()).unwrap()).collect::<Vec<_>>();
        let mut state = Rc::clone(&initial_state).iter().cloned().collect();
        for cmd in cmds.iter() {
            state = apply_all(cmd, &state);
            if !viable(&state) {
                break;
            }
        }
        if state.iter().all(|p| p[0..NUMBERS] == (1..=NUMBERS_U8).collect::<Vec<_>>()) {
            for cmd in cmds {
                println!("{}", show_command(&cmd));
            }
            println!("Length: {}", MAX_LEN);
            std::process::exit(0);
        }
        visited += 1;
        if visited % 100000 == 0 {
            eprint!("\rVisited: {}, Elapsed: {:?}", visited, start.elapsed());
            std::io::stderr().flush().unwrap();
        }
    }


    // fn play(
    //     state: Rc<State>,
    //     length: u8,
    //     mut prg: Node,
    //     visited: &mut u64,
    //     duplicate: &mut u64,
    //     rng: &mut rand::rngs::ThreadRng,
    //     length_map: &sled::Db,
    // ) {
    //     // let play = |state: Rc<State>, length: u8, prg: Node| {
    //     if length == MAX_LEN {
    //         *visited += 1;
    //         if *visited % 100000 == 0 {
    //             eprint!("\rVisited: {}, Duplicates: {}", visited, duplicate);
    //             std::io::stderr().flush().unwrap();
    //         }
    //         if state
    //             .iter()
    //             .all(|p| p[0..NUMBERS] == (1..=NUMBERS_U8).collect::<Vec<_>>())
    //         {
    //             let mut prg = prg;
    //             let mut cmds = vec![];
    //             while let Some(node) = prg.prev {
    //                 cmds.push(node.cmd);
    //                 prg = *node;
    //             }
    //             cmds.reverse();
    //             for cmd in cmds {
    //                 println!("{}", show_command(&cmd));
    //             }
    //             println!("Length: {}", length);
    //             std::process::exit(0);
    //         }
    //         return;
    //     }
    //     let mut cmds = possible_commands();
    //     cmds.shuffle(rng);
    //     let prev_box = Some(Box::new(prg));
    //     let new_length = length + 1;
    //     for cmd in cmds {
    //         let new_state = apply_all(&cmd, &state);

    //         let state_repr = state_positions(&new_state);
    //         if let Some(old_length_vec) = length_map.get(&state_repr).unwrap() {
    //             let old_length = old_length_vec[0];
    //             // if old_length <= new_length { //      solutions_min
    //             if old_length < new_length {
    //                 // solutions_all
    //                 *duplicate += 1;
    //                 continue;
    //             } else {
    //                 // TODO: do something
    //                 // println!("Found shorter path: {} -> {}", old_length, new_length);
    //             }
    //         }
    //         length_map.insert(state_repr, vec![new_length]).unwrap();

    //         if viable(&new_state) {
    //             play(
    //                 Rc::new(new_state),
    //                 new_length,
    //                 Node {
    //                     cmd,
    //                     prev: prev_box.clone(),
    //                 },
    //                 visited,
    //                 duplicate,
    //                 rng,
    //                 length_map,
    //             );
    //         }
    //     }
    // };

    // play(
    //     Rc::clone(&initial_state),
    //     0,
    //     node0,
    //     &mut 0,
    //     &mut 0,
    //     &mut rand::thread_rng(),
    //     &length_map,
    // );

    println!("Elapsed: {:?}", start.elapsed());
}

// recursive visit
// Visited: 4575600000

// rec visit with dedup
// [1, 2, 3]
// Using sled map: /tmp/sled-map0
// Visited: 58800000, Duplicates: 384864807


// completely random playouts
// Visited: 1816200000, Elapsed: 914.330164214s
// Visited: 2846400000, Elapsed: 1431.879537497s