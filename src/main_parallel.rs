use itertools::Itertools;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt::Display;
use std::ops::Range;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use rand::seq::SliceRandom;
use std::collections::HashMap;
// has largest value at the top
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::rc::Rc;
use std::io::Write;
use std::cmp::min;
use serde::{Serialize, Deserialize};


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
    let to = to+1;
    let from = from+1;
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


    // find unused sled-mapX file in a temporary directory (_CONDOR_SCRATCH_DIR or /tmp/ else)
    let tmp_dir = std::env::var("_CONDOR_SCRATCH_DIR").unwrap_or("/tmp".to_string());
    let mut i = 0;
    let mut path = format!("{}/sled-map{}", tmp_dir, i);
    while std::path::Path::new(&path).exists() {
        i += 1;
        path = format!("{}/sled-map{}", tmp_dir, i);
    }
    println!("Using sled map: {}", path);

    // if in git repository, print hash
    let git_hash = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("failed to execute git")
        .stdout;
    let git_hash = String::from_utf8(git_hash).unwrap();
    println!("Git hash: {}", git_hash);
    println!("n = {}", NUMBERS);
    println!("max_len = {}", MAX_LEN);
    println!("swaps = {}", SWAPS);


    // let length_map = sled::open(path).unwrap();
    let mut seen = HashSet::new();

    // extend numerical permutations with register for swap and flags
    // we use RC to avoid cloning the state
    let initial_state : State = (permutations
        .iter()
        .map(|p| {
            let mut perm = Permutation([0; REGS + 2]);
            for (i, &x) in p.iter().enumerate() {
                perm[i] = x;
            }
            perm
        })
        .collect());

    // length_map.insert(state_positions(&initial_state), vec![0 as u8]).unwrap();

    // let node0 = Node{cmd: (0,0,0), prev: None};

    let mut visited : u64 = 0;
    let mut duplicate : u64 = 0;
    // let mut cut : u64 = 0;

    let mut min_perm_count = [init_perm_count; (MAX_LEN as usize)+1];

    let start = std::time::Instant::now();




    let mut frontier = vec![initial_state.clone()];

    let mut length = 0;
    while length<MAX_LEN {
        print!("Length: {}, ", length);
        print!("Frontier: {}, ", frontier.len());
        print!("Seen: {}, ", seen.len());
        print!("Elapsed: {:?}, ", start.elapsed());
        println!();


        min_perm_count[length as usize] = 
            frontier.iter()
            .map(|state| 
                state.iter().map(|p| &p[0..NUMBERS]).unique().count()
            )
            .min()
            .unwrap();

        // check for solutions
        let found = 
            frontier.iter().any(|state| 
                state.iter().all(|p| p[0..NUMBERS] == state[0][0..NUMBERS])
            );
        if found {
            println!("Found: solution of length: {}", length);
            let elapsed = start.elapsed();
            println!("Elapsed: {:?}", elapsed);
            // solution_lengths.lock().unwrap().push(length);
            // exit program
            std::process::exit(0);
            // return vec![state];
        }

        visited += frontier.len() as u64;
        let new_frontier =
            frontier
            .into_par_iter()
            // .into_iter()
            .flat_map(|state| {
                // visited.inc();
                // if visited.get() % 1000 == 0 {
                //     println!("Visited: {}, Duplicate: {} (length: {})", visited.get(), duplicate.get(), length);
                // }

                possible_cmds
                    .iter()
                    .filter_map(|cmd| {
                        let new_state = apply_all(cmd, &state);

                        if !viable(&new_state) {
                            return None;
                        }
                        // if seen.lock().unwrap().contains(&new_state) {
                        //     duplicate.inc();
                        //     return None;
                        // }
                        // seen.lock().unwrap().insert(new_state.clone());

                        if seen.contains(&new_state) {
                        //     duplicate += 1;
                            return None;
                        }

                        // CUT
                        // let new_perm_count = new_state.iter().map(|p| &p[0..NUMBERS]).unique().count();
                        // if new_perm_count < min_perm_count[length as usize] {
                        //     // cut += 1;
                        //     return None;
                        // }

                        Some(new_state)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let new_frontier_length = new_frontier.len();
        // visited += new_frontier_length;

        println!("Filter out duplicates");
        let frontier_filtered = new_frontier
            // filter seen
            .into_iter()
            .unique()
            // .filter(|state| { return !seen.contains(state); })
            .collect::<Vec<_>>();
        duplicate += (new_frontier_length - frontier_filtered.len()) as u64;
        println!("Visited: {}, Duplicate: {} (length: {})", visited, duplicate, length);

        // add all to seen
        seen.extend(frontier_filtered.iter().cloned());
        // if solution_lengths.lock().unwrap().len() > 0 {
        //     println!("Found: {:?} of length: {}", solution_lengths.lock().unwrap(), length);
        //     break;
        // }
        length += 1;
        frontier = frontier_filtered;
    }








    // println!("Found {} solutions", solution_count);

    println!("Visited: {}, Duplicate: {}", visited, duplicate);
    println!("Elapsed: {:?}", start.elapsed());
}