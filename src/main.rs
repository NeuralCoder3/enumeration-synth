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


// use compressible_map::CompressibleMap;
// use diskmap::DiskMap;

// const NUMBERS: usize = 3;
// const MAX_LEN: usize = 12;
const NUMBERS: usize = 4;
const MAX_LEN: u8 = 20;
const SWAPS: usize = 1;
const REGS: usize = NUMBERS + SWAPS;
const CMP: usize = 0;
const MOV: usize = 1;
const CMOVG: usize = 2;
const CMOVL: usize = 3;
const NUMBERS_U8: u8 = NUMBERS as u8;

// Represents a command: (instruction, to, from)
type Command = (usize, usize, usize);
// type Permutation = Vec<u8>;
// type Permutation = [u8; REGS + 2];
type State = Vec<Permutation>;
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Copy)]
struct Permutation([u8; REGS + 2]);
// #[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
// struct State(Vec<Permutation>);

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

// impl IntoIterator for State {
//     type Item = Permutation;
//     type IntoIter = std::vec::IntoIter<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.0.into_iter()
//     }
// }



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

fn apply_invers(cmd: &Command, perm: &Permutation) -> Vec<Permutation> {
    let (instr, to, from) = *cmd;
    match instr {
        CMP => {
            let lt_flag = perm[REGS + 0];
            let gt_flag = perm[REGS + 1];
            // check that flags are set correctly
            if ((lt_flag == 0 && !(perm[to] < perm[from])) || (lt_flag == 1 && perm[to] < perm[from])) &&
                ((gt_flag == 0 && !(perm[to] > perm[from])) || (gt_flag == 1 && perm[to] > perm[from])) {
                // valid flags
                // => return state with flags as anything (would be overwritten)
                return 
                // 0,0; 0,1; 1,0; 1,1
                [(0,0), (0,1), (1,0), (1,1)].iter().map(|(lt,gt)| {
                    let mut new_perm = perm.clone();
                    new_perm[REGS + 0] = *lt;
                    new_perm[REGS + 1] = *gt;
                    new_perm
                }).collect::<Vec<_>>();
            }else {
                return vec![];
            }
        }
        MOV => {
            if perm[to] != perm[from] {
                return vec![];
            }
            // dest could be anything before
            return [0;NUMBERS+1].iter().enumerate().map(|(x, _)| {
                let mut new_perm = perm.clone();
                new_perm[to] = x as u8;
                new_perm
            }).collect::<Vec<_>>();
        }
        CMOVG => {
            let gt_flag = perm[REGS + 1];
            if gt_flag == 0 {
                // flag not set => noop
                return vec![perm.clone()];
            }
            // flag set => was overwrite (same as with MOV)
            return apply_invers(&(MOV, to, from), perm);
        }
        CMOVL => {
            let lt_flag = perm[REGS + 0];
            if lt_flag == 0 {
                // flag not set => noop
                return vec![perm.clone()];
            }
            // flag set => was overwrite (same as with MOV)
            return apply_invers(&(MOV, to, from), perm);
        }
        _ => panic!("Unknown instruction"),
    }
}

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

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PermInfo 
{
    perm: Vec<Vec<u8>>,
    flags: Vec<bool>,
}

impl Display for PermInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?} {:?}", &self.perm, &self.flags)
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct PermInfoVec(Vec<PermInfo>);
impl std::fmt::Display for PermInfoVec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (i, perm_info) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "[{}]", perm_info)?;
        }
        Ok(())
    }
}

impl From<String> for PermInfoVec {
    fn from(s: String) -> Self {
        let mut res = vec![];
        for part in s.split(",") {
            let part = part.trim();
            let part = part.trim_start_matches('[');
            let part = part.trim_end_matches(']');
            let mut iter = part.split_whitespace();
            let perm = iter.next().unwrap();
            let flags = iter.next().unwrap();
            let perm = perm.trim_start_matches('[');
            let perm = perm.trim_end_matches(']');
let perm = perm.split("|").map(|x| {
    x.split(",").map(|y| y.parse::<u8>().unwrap()).collect::<Vec<u8>>()
}).collect::<Vec<Vec<u8>>>();
let flags = flags
    .split(',')
    .map(|x| x.parse::<bool>().unwrap())
    .collect::<Vec<bool>>();
            res.push(PermInfo{perm, flags});
        }
        PermInfoVec(res)
    }
}

impl From<Vec<PermInfo>> for PermInfoVec {
    fn from(vec: Vec<PermInfo>) -> Self {
        PermInfoVec(vec)
    }
}



// permutation -> positions of 1, ..., Number
// e.g. [0,2,1,1] -> [[2,3],[1],[]]
// representation modulo renaming
fn perm_positions(perm: &Permutation) -> PermInfo {
    let mut pos = vec![vec![]; NUMBERS];
    for (i, &n) in perm[0..REGS].iter().enumerate() {
        if n > 0 {
            pos[(n - 1) as usize].push(i as u8);
        }
    }
    // for (i, &n) in perm.iter().enumerate() {
    //     if n > 0 {
    //         pos[(n - 1) as usize].push(i as u8);
    //     }
    // }
    // // sort result to get rid of naming association

    pos.sort();
    // let flags = perm[REGS..].iter().map(|&x| x == 1).collect();
    let flags = perm[REGS..REGS+2].iter().map(|&x| x == 1).collect();
    PermInfo{perm: pos, flags}
}

// fn state_positions(state: &State) -> Vec<PermInfo> {
// fn state_positions(state: &State) -> PermInfoVec {
// fn state_positions(state: &State) -> State {
fn state_positions(state: &State) -> Vec<u8> {
    // state.iter().map(|p| perm_positions(p)).sorted().collect()
    // let res = state.iter().map(|p| perm_positions(p)).collect();
    // PermInfoVec(res)
    // state.clone()
    state.iter().flat_map(|p| p.0).collect()
}

fn main() {
    let possible_cmds = possible_commands();
    let permutations: Vec<Vec<u8>> = (1..=NUMBERS_U8).permutations(NUMBERS).collect(); 
    let init_perm_count = permutations.len();
    // only take 10 random permutations
    // let perm_count = 6;
    // let permutations = permutations.choose_multiple(&mut rand::thread_rng(), perm_count).cloned().collect::<Vec<_>>();
    // let permutations = permutations.into_iter().take(perm_count).collect::<Vec<_>>();

    // let perm_count = 6;
    // let permutations = permutations.choose_multiple(&mut rand::thread_rng(), perm_count).cloned().collect::<Vec<_>>();

    let mut instructions_needed = HashMap::new();
    let mut swaps_needed = HashMap::new();
    // [u8] -> swap count
    {
        // via BFS from 1,...,NUMBERS -> until all permutations found
        let mut frontier = vec![];
        let mut init_perm = [0; NUMBERS];
        for (i, x) in init_perm.iter_mut().enumerate() {
            *x = (i+1) as u8;
        }
        frontier.push(init_perm);
        swaps_needed.insert(init_perm, 0);
        while let Some(perm) = frontier.pop() {
            let swaps = swaps_needed[&perm];
            for i in 0..NUMBERS {
                for j in (i + 1)..NUMBERS {
                    let mut new_perm = perm.clone();
                    new_perm.swap(i, j);
                    if !swaps_needed.contains_key(&new_perm) {
                        swaps_needed.insert(new_perm, swaps + 1);
                        frontier.push(new_perm);
                    }
                }
            }
        }
        println!("Computed swaps for {} permutations", swaps_needed.len());
        if swaps_needed.len() != init_perm_count {
            panic!("Not all permutations found");
        }
        // for perm in swaps_needed.keys() {
        //     println!("Swaps for {:?}: {}", perm, swaps_needed[perm]);
        // }
        // for perm in &permutations {
        //     // println!("Swaps for {:?}: {}", perm, swaps_needed[&perm[..]]);
        //     if !swaps_needed.contains_key(&perm[..]) {
        //         panic!("Permutation {:?} not found", perm);
        //     }
        // }
    }


    // now try any instructions -> relax heuristic (ignore all other dependencies)
    // could be used to only investigate programs that lead to a relaxed solution
    // there might be an instruction that is suboptimal across all individual but optimal global 
    // let's ignore that
    let mut useful_instructions = HashMap::new();
    {
        let mut frontier = VecDeque::new();
        // let mut init_perm :Permutation = [0; REGS + 2];
        let mut init_perm = Permutation([0; REGS + 2]);
        // let mut init_perm : Rc<Permutation> = Rc::new([0; REGS + 2]);
        // 0..NUMBERS -> [1,2,..,NUMBERS]
        for (i, x) in init_perm[0..NUMBERS].iter_mut().enumerate() {
            *x = (i+1) as u8;
        }
        // frontier.push(init_perm);
        // swaps_needed.insert(init_perm, 0);
        let init_perms : Vec<Permutation> = 
            // any swap and any flags
            // possible flags
            [(0,0), (0,1), (1,0), (1,1)].iter().map(|(lt,gt)| {
                // possible swap values
                [0; NUMBERS+1].iter().enumerate().map(|(x, _)| {
                    let mut new_perm = init_perm.clone();
                    new_perm[NUMBERS] = x as u8;
                    new_perm[REGS + 0] = *lt;
                    new_perm[REGS + 1] = *gt;
                    new_perm
                }).collect::<Vec<_>>()
            }).flatten().collect();
        for perm in init_perms {
            instructions_needed.insert(perm, 0);
            frontier.push_back(perm);
        }

        let commands = possible_commands();
        // let mut commands = vec![];
        // for instr in &[MOV, CMOVG, CMOVL, CMP] {
        // // for instr in &[MOV] {
        //     for to in 0..REGS {
        //         for from in 0..REGS {
        //             if to != from {
        //                 commands.push((*instr, to, from));
        //             }
        //         }
        //     }
        // }

        while let Some(perm) = frontier.pop_front() {
            let instructions = instructions_needed[&perm];
            for cmd in &commands {
                for new_perm in apply_invers(cmd, &perm) {
                    if !instructions_needed.contains_key(&new_perm) {
                        instructions_needed.insert(new_perm, instructions + 1);
                        frontier.push_back(new_perm);
                        // add cmd to vec of new_perm
                        useful_instructions.entry(new_perm).or_insert(vec![]).push(*cmd);
                    }
                }
            }
        }
        // for perm in swaps_needed.keys() {
        //     println!("Instructions for {:?}: {}", perm, swaps_needed[perm]);
        // }
    }


    let mut queue = PriorityQueue::new();

    // _CONDOR_SCRATCH_DIR or /tmp/ else
    // find unused sled-mapX file
    let tmp_dir = std::env::var("_CONDOR_SCRATCH_DIR").unwrap_or("/tmp".to_string());
    let mut i = 0;
    let mut path = format!("{}/sled-map{}", tmp_dir, i);
    while std::path::Path::new(&path).exists() {
        i += 1;
        path = format!("{}/sled-map{}", tmp_dir, i);
    }
    println!("Using sled map: {}", path);

    // let mut length_map = HashMap::new();
    // let length_map = sled::open("/tmp/sled-map2").unwrap();
    let length_map = sled::open(path).unwrap();
    // let mut length_map = CompressibleMap::new(compression_params);
    // let length_map : DiskMap<PermInfoVec, usize> = DiskMap::open_new("/tmp/db").unwrap();
    // let score_map = HashMap::new();


    let initial_state: Rc<State> = Rc::new(permutations
        .iter()
        .map(|p| {
            // let mut perm = p.clone();
            // perm.extend(&[0; SWAPS]);
            // perm.extend(&[0, 0]); // Flags
            // perm
            // let mut perm = [0; REGS + 2];
            let mut perm = Permutation([0; REGS + 2]);
            for (i, &x) in p.iter().enumerate() {
                perm[i] = x;
            }
            perm
        })
        .collect());

    // length_map.insert(state_positions(&initial_state).into(), 0);//.unwrap();
    length_map.insert(state_positions(&initial_state), vec![0 as u8]).unwrap();
    // length_map.insert(state_positions(&initial_state), vec![0 as u8]);//.unwrap();
    // length_map.insert(Rc::clone(&initial_state), 0);

    // let init_element = (initial_state, 0);
    // queue.push(&init_element, Reverse(0));
    // queue.push(&initial_state, Reverse(0));
    let node0 = Node{cmd: (0,0,0), prev: None};
    queue.push((node0,Rc::clone(&initial_state),0 as u8), Reverse(0));

    let mut visited : u64 = 0;
    let mut duplicate : u64 = 0;
    let mut candidates = 0;
    let mut cut : u64 = 0;

    // let mut file;
    // // #[cfg(feature = "store-canidates")]
    // {
    //     // environment variable if available
    //     // let tmp_file = std::env::var("TMP_FILE").unwrap_or("/home/s8maullr/results/tmp_len_15_all_perm.log".to_string());
    //     let tmp_file = std::env::var("TMP_FILE").unwrap_or("candidates.log".to_string());
    //     println!("Storing candidates in: {}", tmp_file);
    //     file = std::fs::File::create(tmp_file).unwrap();
    // }

    let mut min_perm_count = [init_perm_count; (MAX_LEN as usize)+1];

    let start = std::time::Instant::now();
    while let Some(((prg,state,length), _)) = queue.pop() {
        // let length = length_map[&state];
        // let length = 42;

        visited += 1;
        if visited % 100000 == 0 {
            // println!("Visited: {}, Duplicate: {}, Current length: {}", visited, duplicate, length);
            print!("Visited: {}, ", visited);
            print!("Duplicate: {}, ", duplicate);
            print!("Cut: {}, ", cut);
            print!("Candidates: {}, ", candidates);
            print!("Current length: {}, ", length);
            print!("Time: {:?}", start.elapsed());
            println!("");
            // #[cfg(feature = "store-canidates")]
            // file.sync_all().unwrap();
        }

        if state.iter().all(|p| p[0..NUMBERS] == state[0][0..NUMBERS]) {
            println!("Found solution: {:?} of length: {}", state, length);


            // reconstruct program
            let mut prg = prg;
            let mut cmds = vec![];
            while let Some(node) = prg.prev {
                cmds.push(prg.cmd);
                prg = *node;
            }
            cmds.reverse();
            println!("Program:");
            for cmd in cmds {
                println!("{}", show_command(&cmd));
            }

            break;
        }

        // if length >= MAX_LEN/3 && state.len() >= init_perm_count/3 {
        //     cut += 1;
        //     continue;
        // }

        // if length >= MAX_LEN/2 && state.len() >= init_perm_count/2 {
        //     cut += 1;
        //     continue;
        // }
        // if length == 6 && state.len() >= init_perm_count-2 {
        //     cut += 1;
        //     continue;
        // }


        // if min_perm_count[length] < state.len() {
        //     // TODO: too strict? => yes
        //     cut += 1;
        //     continue;
        // } else 
        // if min_perm_count[min(length,length-1)] < state.len() {
            // Visited: 15900000, Duplicate: 45826464, Cut: 122872979, Candidates: 0, Current length: 7, 
            // Visited: 15961087, Duplicate: 45826464
            // Elapsed: 470.048455071s

        
        // let needed_instructions = state.iter().map(|p| instructions_needed.get(p).unwrap()).max().unwrap();
        // if needed_instructions + length >= MAX_LEN {
        //     cut += 1;
        //     continue;
        // }

        // if min_perm_count[min(length,length-1)]+2 < state.len() {
        //     // works with 4
        //     cut += 1;
        //     continue;
        // } else 
        // if min_perm_count[length] > state.len() {
        //     min_perm_count[length] = state.len();
        // }
        
        
        // if length == 15 {
        // //     println!("Length 15: {:?}", state);
        // //     break;
        //     // append state to file
        //     // #[cfg(feature = "store-canidates")]
        //     {
        //     let state_str = format!("{:?}\n", state);
        //     file.write_all(state_str.as_bytes()).unwrap();
        //     }
        //     candidates += 1;
        //     continue;
        // }

        if length >= MAX_LEN {
            continue;
        }

        // let successors =
        //     possible_cmds
        //     .iter()
        //     .filter_map(|cmd| {
        //         let new_state = apply_all(cmd, &state);

        //         if !viable(&new_state) {
        //             return None;
        //         }
        //         Some(new_state)
        //     })
        //     .collect::<Vec<_>>();
        // for new_state in successors {
        //     let eq_repr = state_positions(&new_state);
        //     if length_map.contains_key(&eq_repr) {
        //         continue;
        //     }

        //     let new_length = length + 1;
        //     let new_score = new_length + 0; // heuristic

        //     length_map.insert(new_state, new_length);
        //     score_map.insert(new_state, new_score);

        //     queue.push(new_state, Reverse(new_score));
        // }

        let prev_box = Some(Box::new(prg));

        // let commands = possible_cmds;
        let commands = 
            state.iter().flat_map(|p| useful_instructions.get(p).unwrap_or(&possible_cmds).iter())
            .unique()
            .cloned()
            .collect::<Vec<_>>();

        for cmd in &commands {
            let new_state = Rc::new(apply_all(&cmd, &state));
            let new_length = length + 1;

            if !viable(&new_state) {
                // duplicate += 1;
                cut += 1;
                continue;
            }

            // cut before insertion to save memory (and have value ready for heuristics)
            let needed_instructions = new_state.iter().map(|p| instructions_needed.get(p).unwrap()).max().unwrap();
            if needed_instructions + new_length >= MAX_LEN {
                cut += 1;
                continue;
            }

            // if already found with smaller length, skip
            let state_repr = state_positions(&new_state);
            // let state_repr = state_positions(&new_state).into();
            // let state_repr = new_state;
            // if let Some(&old_length) = length_map.get(&state_repr) {
            if let Some(old_length_vec) = length_map.get(&state_repr).unwrap() {
            // if let Some(old_length_vec) = length_map.get(&state_repr) {
                let old_length = old_length_vec[0];
            // if let Ok(old_length) = length_map.get(&state_repr) {
            // if let Some(&old_length) = length_map.get(&*new_state) {
            // if let Some(&old_length) = length_map.get(&new_state) {
                if old_length <= new_length {
                    duplicate += 1;
                    continue;
                }
            }
            // length_map.insert(state_repr, new_length);//.unwrap();
            length_map.insert(state_repr, vec![new_length]).unwrap();
            // length_map.insert(state_repr, vec![new_length]);//.unwrap();
            // length_map.insert(Rc::clone(&new_state), new_length);
            // length of state as heuristic
            // let heuristic = new_state.len();


            // // unique permutations in 0..NUMBERS
            // // we are only interested in the unique permutations
            // // to be precise, the log of the perm count is a good heuristic for the needed swaps
            // // each swap halves the number of permutations
            let heuristic = new_state.iter().map(|p| &p[0..NUMBERS]).unique().count();
            // // fast log2
            // // let heuristic = std::mem::size_of::<usize>() * 8 - (heuristic.leading_zeros() as usize) - 1;
            // // we weigh the swaps with 4 as each swap takes roughly 4 instructions
            // let heuristic = 4*heuristic;


            // max of needed swaps over all permutations
            // for p in new_state.iter() {
            //     if !swaps_needed.contains_key(&p[0..NUMBERS]) {
            //         panic!("Permutation {:?} not found", &p[0..NUMBERS]);
            //     }
            // }
            // let heuristic = new_state.iter().map(|p| swaps_needed[&(p[0..NUMBERS])]).max().unwrap();
            // let heuristic = new_state.iter().map(|p| swaps_needed.get(&p[0..NUMBERS]).unwrap_or(&2)).max().unwrap();
            // let heuristic = new_state.iter().map(|p| swaps_needed.get(&p[0..NUMBERS]).unwrap_or(&1)).max().unwrap();
            // let heuristic = 4*new_state.iter().map(|p| swaps_needed.get(&p[0..NUMBERS]).unwrap_or(&1)).sum::<usize>();
            // let heuristic = new_state.iter().map(|p| swaps_needed.get(&p[0..NUMBERS]).unwrap_or(&1)).sum::<usize>();

            // try with instruction heuristic instead
            // let heuristic = new_state.iter().map(|p| instructions_needed[p]).max().unwrap();

            // let heuristic = 0;
            // let new_score = new_length + heuristic;
            let new_score = heuristic;
            // score_map.insert(new_state, new_score);

            // let element = (new_state, new_length);
            // queue.push(&element, Reverse(new_score));
            // queue.push(&new_state, Reverse(new_score));
            let prg = Node{cmd: *cmd, prev: prev_box.clone()};
            // queue.push((Rc::clone(&new_state),new_length), Reverse(new_score));
            queue.push((prg,Rc::clone(&new_state),new_length), Reverse(new_score));
        }
    }

    // #[cfg(feature = "store-canidates")]
    // {
    // // close file
    // file.sync_all().unwrap();
    // drop(file);
    // }

    println!("Visited: {}, Duplicate: {}", visited, duplicate);
    println!("Elapsed: {:?}", start.elapsed());
}

// TMP_FILE=candidates.log cargo run --release --features "store-candidates"
// TMP_FILE=candidates.log cargo run --release --bin compute_vec --all-features

// cargo build --release --features "store-candidates"
// TMP_FILE=candidates.log






// Frontier: 6005241
// Check solutions
// Found: 18 of length: 11
// Elapsed: 66.044976606s
// Visited: 4636286, Duplicate: 184082486



// Length: 11
// Frontier: 3048404
// Check solutions
// Found: 30 of length: 11
// Elapsed: 66.004879274s
// Visited: 3172209, Duplicate: 127012166



// Length: 11
// Frontier: 3048404
// Check solutions
// Found: 30 of length: 11
// Elapsed: 53.147533951s
// Visited: 3172209, Duplicate: 127012166


// Length: 11
// Frontier: 933598
// Check solutions
// Found: 2 of length: 11
// Elapsed: 19.649023656s
// Visited: 1318079, Duplicate: 53107642








// A* without heuristic = Dijkstra
// Found solution: [[2, 1, 3, 0, 0, 1], [2, 1, 3, 2, 0, 1], [2, 1, 3, 3, 1, 0]] of length: 11
// Visited: 4803316, Duplicate: 190721609
// Elapsed: 74.769883371s


// custom A* without heuristic position hash
// already visit many of length 11 first
// Visited: 2000000, Duplicate: 80978956, Current length: 11
// Found solution: [[3, 2, 1, 1, 0, 1], [3, 2, 1, 2, 1, 0]] of length: 11
// Visited: 2071418, Duplicate: 83909981
// Elapsed: 60.436542603s

// custom A*, len heuristic, position hash
// Found solution: [[1, 2, 3, 1, 0, 1], [1, 2, 3, 2, 1, 0]] of length: 11
// Visited: 39253, Duplicate: 1511701
// Elapsed: 1.302491638s




// greedy cut all permutations for 4 => no solutions
// Visited: 3100000, Duplicate: 9377953, Cut: 22331993, Candidates: 0, Current length: 6, 
// Visited: 3142624, Duplicate: 9377953
// Elapsed: 48.940570828s