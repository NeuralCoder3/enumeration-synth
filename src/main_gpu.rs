use itertools::Itertools;
use rand::seq::SliceRandom;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator as _;
use std::arch::x86_64::_mm256_permute4x64_epi64;
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
extern crate ocl;
// needs opencl-headers opencl-info ocl-icd

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
    // new_state.sort(); // sorting important for finding same states (symmetries)
    // new_state.dedup();
    new_state
}

fn viable_perm(perm: &[u8]) -> bool {
    for n in 1..=NUMBERS_U8 {
        if !perm[0..REGS].contains(&n) {
            return false;
        }
    }
    true
}

// check if the state can never reach a solution
// corresponds to delete-relaxed planning check
fn viable(state: &State) -> bool {
    for perm in state {
        if !viable_perm(&perm.0) {
            return false;
        }
        // for n in 1..=NUMBERS_U8 {
        //     if !perm[0..REGS].contains(&n) {
        //         return false;
        //     }
        // }
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
    // let mut seen  : HashSet<[&u8]> = HashSet::new();
    let seen = sled::open(path).unwrap();

    // extend numerical permutations with register for swap and flags
    // we use RC to avoid cloning the state
    let initial_state: State = (permutations
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

    let mut visited: u64 = 0;
    let mut duplicate: u64 = 0;
    // let mut cut : u64 = 0;

    let mut min_perm_count = [init_perm_count; (MAX_LEN as usize) + 1];

    let start = std::time::Instant::now();

    // make flat
    let flat_init_state = initial_state.iter().flat_map(|p| p.0).collect::<Vec<_>>();
    let mut frontier : Vec<u8> = flat_init_state.clone();
    println!("Initial state: {:?}", flat_init_state);

    let permutation_size = REGS + 2;
    let state_size = init_perm_count * permutation_size;
    assert_eq!(state_size, flat_init_state.len());

    let mut length = 0;
    while length < MAX_LEN {
        let state_count = frontier.len() / state_size;
        print!("Length: {}, ", length);
        print!("Frontier Bytes: {}, ", frontier.len());
        print!("Frontier States: {}, ", state_count);
        print!("Seen: {}, ", seen.len());
        print!("Elapsed: {:?}, ", start.elapsed());
        println!();

        // min_perm_count[length as usize] = frontier
        //     .iter()
        //     .map(|state| state.iter().map(|p| &p[0..NUMBERS]).unique().count())
        //     .min()
        //     .unwrap();

        let mut ctx = ocl::ProQue::builder()
            .src(include_str!("gpu.cl"))
            .build()
            .unwrap();
        ctx.set_dims(frontier.len());

        visited += state_count as u64;
        let new_frontier = possible_cmds
            .iter()
            .flat_map(|cmd| {

                // 56s CPU
                // 17s Parallel (22s without dedup) -- 46 vs 17s for main_parallel

                let output_buffer = ctx.create_buffer::<u8>().unwrap();
                let mut output_array = vec![0; state_size];
                output_buffer.write(&output_array).enq().unwrap();

                let state_buffer = ctx.create_buffer::<u8>().unwrap();
                state_buffer.write(&frontier).enq().unwrap();

                let command_buffer = ctx.create_buffer::<u8>().unwrap();
                let command_array = [cmd.0 as u8, cmd.1 as u8, cmd.2 as u8];
                command_buffer.write(command_array.as_slice()).enq().unwrap();

                let program = ctx.program();
                let kernel = ocl::Kernel::builder()
                    .program(&program)
                    .name("apply")
                    .queue(ctx.queue().clone())
                    .global_work_size(frontier.len() / state_size)
                    .arg(&state_buffer)
                    .arg(&command_buffer)
                    .arg(&output_buffer)
                    // .arg(&state_size)
                    // .arg(&permutation_size)
                    .build()
                    .unwrap();

                unsafe {
                    kernel.enq().unwrap();
                }

                output_buffer.read(&mut output_array).enq().unwrap();
                let mut new_frontier = vec![0; state_size * state_count];
                state_buffer.read(&mut new_frontier).enq().unwrap();

                // TODO: only operate on flat structure
                // instead of sort use sorted of idx of perms (via trie?)

                // reconstruct frontier from state_array

                let new_frontier=new_frontier
                    .chunks_exact(state_size)
                    .filter_map(|s| {
                        // println!("State: {:?}", s);
                        // return Some(s.to_vec());
                        // check viable
                        for pi in 0..init_perm_count {
                            let perm = &s[pi * permutation_size..(pi + 1) * permutation_size];
                            if !viable_perm(perm) {
                                return None;
                            }
                        }
                        // return Some(s.to_vec());

                        // sort perm (chunked) without copying
                        // first chunk
                        // then sort
                        // then flatten
                        // check if duplicate
                        // else insert and keep
                        // sorted_state 
                        // let permutations = s.chunks_exact(permutation_size).collect::<Vec<_>>();
                        let sorted_perms = 
                        // s;
                            s.chunks_exact(permutation_size)
                            .sorted()
                            .flatten()
                            .copied()
                            .collect::<Vec<_>>();
                        // if seen.contains(&sorted_perms) {
                        //     return None;
                        // }
                        if let Some(_) = seen.get(&sorted_perms).unwrap() {
                            return None;
                        }
                        // seen.insert(sorted_perms, vec![0]);
                        seen.insert(sorted_perms.clone(), vec![0]).unwrap();
                        Some(sorted_perms)
                        // Some(sorted_perms.iter().copied().collect::<Vec<_>>())
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                // println!("Command: {} -> {}", show_command(cmd), new_frontier.len());
                new_frontier
                
                // state_array
                //     .chunks_exact(state_size)
                //     .map(|s| {
                //         s.chunks_exact(permutation_size)
                //             .map(|p| {
                //                 // let mut perm = Permutation([0; REGS + 2]);
                //                 // for (i, &x) in p.iter().enumerate() {
                //                 //     perm[i] = x;
                //                 // }
                //                 // perm

                //                 // create permutation out of p directly
                //                 let perm = Permutation(p.try_into().unwrap());
                //                 perm
                //             })
                //             .collect::<Vec<_>>()
                //     })
                //     .filter_map(|mut state| {
                //         state.sort();
                //         if !viable(&state) {
                //             return None;
                //         }
                //         if seen.contains(&state) {
                //             return None;
                //         }
                //         Some(state)
                //     })
                //     // .collect::<Vec<_>>()
                //     .for_each(drop);

// Visited: 5383230, Duplicate: 9442652 (length: 10)
// Found: solution of length: 11
// Elapsed: 76.619163288s

// no complex struct
// Visited: 5383230, Duplicate: 0
// Elapsed: 185.780830764s
// without sort long


// Visited: 5383230, Duplicate: 9442652 (length: 10)
// Found: solution of length: 11
// Elapsed: 45.615414616s

                // frontier
                // // .iter()
                // .par_iter()
                // .filter_map(|state| {
                //     let mut new_state = apply_all(cmd, &state);
                //     new_state.sort();
                //     if !viable(&new_state) {
                //         return None;
                //     }
                //     if seen.contains(&new_state) {
                //         // duplicate += 1;
                //         return None;
                //     }
                //     Some(new_state)
                // })
                // .collect::<Vec<_>>()

            })
            .collect::<Vec<_>>();

        let new_frontier_length = new_frontier.len();
        // visited += new_frontier_length;

        println!("Filter out duplicates");
        let frontier_filtered = new_frontier
            // filter seen
            .into_iter()
            // .unique()
            // .filter(|state| { return !seen.contains(state); })
            .collect::<Vec<_>>();
        duplicate += (new_frontier_length - frontier_filtered.len()) as u64;
        println!(
            "Visited: {}, Duplicate: {} (length: {})",
            visited, duplicate, length
        );

        // add all to seen
        // seen.extend(frontier_filtered.iter().cloned());
        // frontier_filtered.
        // if solution_lengths.lock().unwrap().len() > 0 {
        //     println!("Found: {:?} of length: {}", solution_lengths.lock().unwrap(), length);
        //     break;
        // }
        length += 1;
        frontier = frontier_filtered;
        // convert from vec<&u8> to vec<u8>
        // frontier = frontier_filtered.into_iter().copied().collect();


        // check for solutions
        // let found = 
        //     frontier.iter().any(|state| 
        //         // state.iter().all(|p| p[0..NUMBERS] == state[0][0..NUMBERS])
        //         state.iter().all(|p| p[0..NUMBERS] == (1..=NUMBERS_U8).collect::<Vec<_>>()
        //     ));
        let found = 
            frontier.chunks_exact(state_size)
            .any(|s| {
                s.chunks_exact(permutation_size)
                .all(|p| p[0..NUMBERS].iter().all(|&x| x == 1))
            });
        if found {
            println!("Found: solution of length: {}", length);
            let elapsed = start.elapsed();
            println!("Elapsed: {:?}", elapsed);
            // solution_lengths.lock().unwrap().push(length);
            // exit program
            std::process::exit(0);
            // return vec![state];
        }
    }

    // println!("Found {} solutions", solution_count);

    println!("Visited: {}, Duplicate: {}", visited, duplicate);
    println!("Elapsed: {:?}", start.elapsed());
}
