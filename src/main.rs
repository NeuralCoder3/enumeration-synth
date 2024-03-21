use itertools::Itertools;
use std::collections::HashSet;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use rand::seq::SliceRandom;
use std::collections::HashMap;
// has largest value at the top
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::rc::Rc;
use std::io::Write;


// const NUMBERS: usize = 3;
// const MAX_LEN: usize = 12;
const NUMBERS: usize = 4;
const MAX_LEN: usize = 20;
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
type Permutation = [u8; REGS + 2];
type State = Vec<Permutation>;

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
#[derive(Clone)]
struct Node {
    cmd: Command,
    prev: Option<Box<Node>>,
}

#[derive(Clone, Eq, PartialEq, Hash)]
struct PermInfo 
{
    perm: Vec<Vec<u8>>,
    flags: Vec<bool>,
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
    let flags = perm[REGS..].iter().map(|&x| x == 1).collect();
    PermInfo{perm: pos, flags}
}

fn state_positions(state: &State) -> Vec<PermInfo> {
    // state.iter().map(|p| perm_positions(p)).sorted().collect()
    state.iter().map(|p| perm_positions(p)).collect()
}

fn main() {
    let possible_cmds = possible_commands();
    let permutations: Vec<Vec<u8>> = (1..=NUMBERS_U8).permutations(NUMBERS).collect(); 
    // only take 10 random permutations
    // let perm_count = 6;
    // let permutations = permutations.choose_multiple(&mut rand::thread_rng(), perm_count).cloned().collect::<Vec<_>>();
    // let permutations = permutations.into_iter().take(perm_count).collect::<Vec<_>>();

    // let perm_count = 6;
    // let permutations = permutations.choose_multiple(&mut rand::thread_rng(), perm_count).cloned().collect::<Vec<_>>();


    let mut queue = PriorityQueue::new();

    let mut length_map = HashMap::new();
    // let score_map = HashMap::new();


    let initial_state: Rc<State> = Rc::new(permutations
        .iter()
        .map(|p| {
            // let mut perm = p.clone();
            // perm.extend(&[0; SWAPS]);
            // perm.extend(&[0, 0]); // Flags
            // perm
            let mut perm = [0; REGS + 2];
            for (i, &x) in p.iter().enumerate() {
                perm[i] = x;
            }
            perm
        })
        .collect());

    length_map.insert(state_positions(&initial_state), 0);
    // length_map.insert(Rc::clone(&initial_state), 0);

    // let init_element = (initial_state, 0);
    // queue.push(&init_element, Reverse(0));
    // queue.push(&initial_state, Reverse(0));
    queue.push((Rc::clone(&initial_state),0), Reverse(0));

    let start = std::time::Instant::now();
    let mut visited : u64 = 0;
    let mut duplicate : u64 = 0;
    let mut candidates = 0;

    let tmp_file = "/home/s8maullr/results/tmp_len_15_all_perm.log";
    let mut file = std::fs::File::create(tmp_file).unwrap();

    while let Some(((state,length), _)) = queue.pop() {
        // let length = length_map[&state];
        // let length = 42;

        visited += 1;
        if visited % 100000 == 0 {
            println!("Visited: {}, Duplicate: {}, Current length: {}", visited, duplicate, length);
            println!("Candidates: {}", candidates);
            file.sync_all().unwrap();
        }

        if state.iter().all(|p| p[0..NUMBERS] == state[0][0..NUMBERS]) {
            println!("Found solution: {:?} of length: {}", state, length);
            break;
        }
        
        if length == 15 {
        //     println!("Length 15: {:?}", state);
        //     break;
            // append state to file
            let state_str = format!("{:?}\n", state);
            file.write_all(state_str.as_bytes()).unwrap();
            candidates += 1;
            continue;
        }

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
        for cmd in &possible_cmds {
            let new_state = Rc::new(apply_all(&cmd, &state));
            let new_length = length + 1;

            if !viable(&new_state) {
                duplicate += 1;
                continue;
            }

            // if already found with smaller length, skip
            let state_repr = state_positions(&new_state);
            // let state_repr = new_state;
            if let Some(&old_length) = length_map.get(&state_repr) {
            // if let Some(&old_length) = length_map.get(&*new_state) {
            // if let Some(&old_length) = length_map.get(&new_state) {
                if old_length <= new_length {
                    duplicate += 1;
                    continue;
                }
            }

            length_map.insert(state_repr, new_length);
            // length_map.insert(Rc::clone(&new_state), new_length);
            // length of state as heuristic
            let heuristic = new_state.len();
            // let heuristic = 0;
            let new_score = new_length + heuristic;
            // score_map.insert(new_state, new_score);

            // let element = (new_state, new_length);
            // queue.push(&element, Reverse(new_score));
            // queue.push(&new_state, Reverse(new_score));
            queue.push((Rc::clone(&new_state),new_length), Reverse(new_score));
        }
    }

    // close file
    file.sync_all().unwrap();
    drop(file);

    println!("Visited: {}, Duplicate: {}", visited, duplicate);
    println!("Elapsed: {:?}", start.elapsed());
}



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