use itertools::Itertools;
use std::collections::VecDeque;
use std::ops::Range;
use std::collections::HashMap;
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::rc::Rc;


const NUMBERS: usize = 5;
const MAX_LEN: u8 = 33;
const SWAPS: usize = 1;
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

// "undo" a command on a permutation to traverse the program backwards
// multiple "origins" that lead to the given permutations are possible
// returns an empty vector if the command can not result in the given permutation
// could alternatively be computed via brute force
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
                // 0,0; 0,1; 1,0; 1,1 as possibilities for the flags
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
    }


    // now try any instructions -> relax heuristic (ignore all other dependencies)
    // could be used to only investigate programs that lead to a relaxed solution
    // there might be an instruction that is suboptimal across all individual but optimal global 
    // let's ignore that
    let mut useful_instructions = HashMap::new();
    {
        let mut frontier = VecDeque::new();
        let mut init_perm = Permutation([0; REGS + 2]);
        for (i, x) in init_perm[0..NUMBERS].iter_mut().enumerate() {
            *x = (i+1) as u8;
        }
        let init_perms : Vec<Permutation> = 
            // any swap and any flags
            // possible flags
            [(0,0), (0,1), (1,0), (1,1)].iter().map(|(lt,gt)| {
                // possible swap values
                // for general swap count, we need {0,...,N}^swap
                let numbers = (0..=NUMBERS_U8).collect::<Vec<u8>>();
                itertools::repeat_n(numbers, SWAPS).multi_cartesian_product().map(|swap| {
                    let mut new_perm = init_perm.clone();
                    for (i, &x) in swap.iter().enumerate() {
                        new_perm[NUMBERS+i] = x;
                    }
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
        println!("Computed instructions for {} permutation states", instructions_needed.len());
    }


    let mut queue = PriorityQueue::new();

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


    let length_map = sled::open(path).unwrap();

    // extend numerical permutations with register for swap and flags
    // we use RC to avoid cloning the state
    let initial_state: Rc<State> = Rc::new(permutations
        .iter()
        .map(|p| {
            let mut perm = Permutation([0; REGS + 2]);
            for (i, &x) in p.iter().enumerate() {
                perm[i] = x;
            }
            perm
        })
        .collect());

    length_map.insert(state_positions(&initial_state), vec![0 as u8]).unwrap();

    let node0 = Node{cmd: (0,0,0), prev: None};
    queue.push((node0,Rc::clone(&initial_state),0 as u8), Reverse(0));

    let mut visited : u64 = 0;
    let mut duplicate : u64 = 0;
    let mut cut : u64 = 0;

    let mut min_perm_count = [init_perm_count; (MAX_LEN as usize)+1];

    let start = std::time::Instant::now();
    while let Some(((prg,state,length), _)) = queue.pop() {

        visited += 1;
        if visited % 100000 == 0 {
            print!("Open: {}, ", queue.len());
            print!("Visited: {}, ", visited);
            print!("Duplicate: {}, ", duplicate);
            print!("Cut: {}, ", cut);
            print!("Current length: {}, ", length);
            print!("Time: {:?}", start.elapsed());
            println!("");
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

        if length >= MAX_LEN {
            continue;
        }

        let prev_box = Some(Box::new(prg));

        // let commands = possible_cmds;
        let commands = 
            state.iter().flat_map(|p| useful_instructions.get(p).unwrap_or(&possible_cmds).iter())
            .unique()
            // .cloned()
            .collect::<Vec<_>>();

        // for cmd in &possible_cmds {
        for cmd in commands {
            let new_state = Rc::new(apply_all(&cmd, &state));
            let new_length = length + 1;

            if !viable(&new_state) {
                cut += 1;
                continue;
            }

            // cut before insertion to save memory (and have value ready for heuristics)
            let needed_instructions = new_state.iter().map(|p| instructions_needed.get(p).unwrap()).max().unwrap();
            if needed_instructions + new_length > MAX_LEN {
                cut += 1;
                continue;
            }

            let new_perm_count = new_state.iter().map(|p| &p[0..NUMBERS]).unique().count();
            let new_length_u = new_length as usize;

            // > INSERT HEURISTICS CUTS HERE

            // for length (including swap states)
            // if min_perm_count[new_length_u] > new_state.len() {
            //     min_perm_count[new_length_u] = new_state.len();
            // }
            // only perm
            if min_perm_count[new_length_u] > new_perm_count {
                min_perm_count[new_length_u] = new_perm_count;
            }


            // if already found with smaller length, skip
            let state_repr = state_positions(&new_state);
            if let Some(old_length_vec) = length_map.get(&state_repr).unwrap() {
                let old_length = old_length_vec[0];
                if old_length <= new_length {
                    duplicate += 1;
                    continue;
                }
            }
            length_map.insert(state_repr, vec![new_length]).unwrap();

            /*
                For the heuristic, we could:
                - use the number of unique permutations remaining
                - use the number of unique register states remaining (permutations with flags and swaps)
                - the number of required swaps (roughly log of permutation count as each swap roughly halves the permutation count)
                - weight the swap count with 4 for rough instruction count
                - use the precomputed swap count (cayley distance)
                - use the number of instructions needed per permutation (precomputed -- relaxed plan ignoring dependencies)

                However, these seem to be slower (or not much faster) than the permutation count heuristic
             */


            let heuristic = new_perm_count as u8;
            // try with instruction heuristic instead
            // let heuristic = new_state.iter().map(|p| instructions_needed[p]).max().unwrap();
            // let heuristic = 0;

            let new_score = new_length + heuristic;
            // we can use A* (f+h) or Dijkstra (f) or greedy (h)
            let prg = Node{cmd: *cmd, prev: prev_box.clone()};
            queue.push((prg,Rc::clone(&new_state),new_length), Reverse(new_score));
        }
    }


    println!("Visited: {}, Duplicate: {}", visited, duplicate);
    println!("Elapsed: {:?}", start.elapsed());
}