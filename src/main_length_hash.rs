use itertools::Itertools;
use std::collections::{HashSet, VecDeque, HashMap};

const NUMBERS: usize = 3;
const SWAPS: usize = 1;
const REGS: usize = NUMBERS + SWAPS;
const CMP: usize = 0;
const MOV: usize = 1;
const CMOVG: usize = 2;
const CMOVL: usize = 3;
const NUMBERS_U8: u8 = NUMBERS as u8;

// Represents a command: (instruction, to, from)
type Command = (usize, usize, usize);
type Permutation = Vec<u8>;
type State = Vec<Permutation>;

fn possible_commands() -> Vec<Command> {
    let mut commands = vec![];
    for instr in &[MOV, CMOVG, CMOVL] {
        for to in 0..REGS {
            for from in 0..REGS {
                commands.push((*instr, to, from));
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

fn apply(cmd: &Command, perm: &mut [u8]) {
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

fn apply_all(cmd: &Command, state: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut new_state = Vec::new();
    // let mut new_state = HashSet::new();
    for perm in state {
        let mut new_perm = perm.clone();
        apply(cmd, &mut new_perm);
        new_state.push(new_perm);
        // new_state.insert(new_perm);
    }
    // new_state.into_iter().collect()
    // strict sort to avoid duplicates
    new_state.sort();
    new_state.dedup();
    new_state
}

fn viable(state: &[Vec<u8>]) -> bool {
    for perm in state {
        for n in 1..=(NUMBERS_U8) {
            if !perm[0..REGS].contains(&n) {
                return false;
            }
        }
    }
    true
}

fn show_command(cmd: &Command) -> String {
    let (instr, to, from) = *cmd;
    match instr {
        CMP => format!("CMP {} {}", to, from),
        MOV => format!("MOV {} {}", to, from),
        CMOVG => format!("CMOVG {} {}", to, from),
        CMOVL => format!("CMOVL {} {}", to, from),
        _ => panic!("Unknown instruction"),
    }
}


// hash a permutation into a 64-bit number
// 2 bit per register ((3+1)*2) + 1 bit per flag (2) 
fn perm_hash(perm: &Permutation) -> u16 {
    let mut hash = 0;
    for i in 0..REGS {
        hash |= (perm[i] as u16) << (i * 2);
    }
    hash |= (perm[REGS] as u16) << (REGS * 2);
    hash |= (perm[REGS + 1] as u16) << (REGS * 2 + 1);
    hash
}

// at most 6 permutations => 60 bits
// concat all permutation hash
fn state_hash(state: &State) -> u64 {
    let mut hash = 0;
    for perm in state {
        hash = (hash << 10) | perm_hash(perm) as u64;
    }
    hash
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct StateStruct {
    state: State,
}

impl std::hash::Hash for StateStruct {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(state_hash(&self.state));
    }
}

// struct StateHasher;

// impl std::hash::BuildHasher for StateHasher {
//     type Hasher = StateStruct;
//     fn build_hasher(&self) -> StateStruct {
//         StateStruct { state: 0 }
//     }
// }


fn main() {
    let possible_cmds = possible_commands();
    let permutations: Vec<Vec<u8>> = (1..=(NUMBERS_U8)).permutations(NUMBERS).collect(); // Use itertools permutations

    let mut queue = VecDeque::new();
    // let mut queue : PriorityQueue<Vec<Vec<usize>>, _> = PriorityQueue::new();
    // let mut seen = HashSet::new();
    // overload hashset to use state_hash
    // let mut seen = HashSet::new();
    let mut seen = HashSet::new();
    let mut visited = 0;
    let mut duplicate = 0;
    // index (visited) -> (operation, previous)
    // let mut info : HashMap<usize, (usize, usize)> = HashMap::new();
    // state -> (operation, previous)
    // let mut info : HashMap<Vec<Vec<usize>>, (Command, Vec<Vec<usize>>)> = HashMap::new();
    // let mut program_length_map : HashMap<Vec<Vec<usize>>, usize> = HashMap::new();

    let initial_state: State = permutations
        .iter()
        .map(|p| {
            let mut perm : Permutation = p.clone();
            perm.extend(&[0 as u8; SWAPS]);
            perm.extend(&[0 as u8, 0 as u8]); // Flags
            perm
        })
        .collect();

    queue.push_back((initial_state.clone(), 0));
    // queue.push(initial_state.clone(), 100-initial_state.len() as i32);
    // queue.push(initial_state.clone(), Reverse(0 as usize));
    // seen.insert(initial_state.clone()); // Insert vector instead of HashSet

    println!("Starting search");

    // let mut final_states = Vec::new();
    let goal_perm = initial_state[0][0..NUMBERS].to_vec();

    let start_time = std::time::Instant::now();

    // while let Some(state) = queue.pop_front() {
    while let Some((state,prog_len)) = queue.pop_front() {
        // if seen.contains(&state) {
        //     duplicate += 1;
        //     continue;
        // }

        // visited += 1;
        // if !viable(&state) {
        //     continue;
        // }

        visited += 1;
        // seen.insert(state.clone()); 

        if visited % 1000 == 0 {
            // println!("Visited: {}, Duplicate: {}, Queue: {}, Final: {}", visited, duplicate, queue.len(), final_states.len());
            println!("Visited: {}, Duplicate: {}, Queue: {}", visited, duplicate, queue.len());
            println!("Current length: {}", prog_len);
        }

        // all perm in state are 1..=NUMBERS in the first few registers
        // if state.iter().all(|p| p[0..NUMBERS] == initial_state[0][0..NUMBERS]) {
        if state.iter().all(|p| p[0..NUMBERS] == goal_perm) {
            println!("Found: {:?} of length: {}", state, prog_len);
            let elapsed = start_time.elapsed();
            println!("Elapsed: {:?}", elapsed);
            // final_state = state;
            break;
            // final_states.push(state.clone());
            // continue;
        }
        // if state.len() == 1 {
        //     println!("Found: {:?} of length: {}", state, prog_len.0);
        //     // final_state = state;
        //     break;
        //     // final_states.push(state.clone());
        //     // continue;
        // }

        // all permutations are the same
        // if state.iter().all(|p| p[0..NUMBERS] == initial_state[0][0..NUMBERS]) {
        //     println!("Found: {:?}", state);
        //     break;
        // }

        for cmd in &possible_cmds {
            let new_state = apply_all(cmd, &state);

            if !viable(&new_state) {
                continue;
            }
            // if seen.contains(&new_state) {
            //     duplicate += 1;
            //     continue;
            // }
            // seen.insert(new_state.clone());
            let state_hash = state_hash(&new_state);
            if seen.contains(&state_hash) {
                duplicate += 1;
                continue;
            }
            seen.insert(state_hash);
            // let state_struct = StateStruct { state: new_state.clone() };
            // if seen.contains(&state_struct) {
            //     duplicate += 1;
            //     continue;
            // }
            // seen.insert(state_struct);

            // TODO: need update if new shorter (possible ? we do dijkstra)

            // queue.push_back(new_state);
            // let len = new_state.len() as i32;
            // info.insert(new_state.clone(), (*cmd, state.clone()));
            queue.push_back((new_state, prog_len + 1));
        }
    }

    println!("Visited: {}, Duplicate: {}", visited, duplicate);

    // for final_state in final_states {
    //     let mut state = final_state;
    //     let length = program_length_map.get(&state).unwrap();
    //     println!("Program of length: {}", length);
    // }

    // if final_state.len() > 0 {
    //     let mut state = final_state;
    //     let mut program_states = vec![];
    //     while state.len() > 0 {
    //         if let Some((op, prev)) = info.get(&state) {
    //             // println!("{}", show_command(op));
    //             program_states.push((op, state.clone()));
    //             state = prev.clone();
    //         } else {
    //             break;
    //         }
    //     }
    //     program_states.reverse();

    //     println!("Program of length: {}", program_states.len());
 
    //     for (op, state) in program_states {
    //         println!("{}", show_command(op));
    //         for p in state {
    //             println!("{:?}", p);
    //         }
    //         println!();
    //     }
    // }
}

/*
Visited: 9764000, Duplicate: 9423494, Queue: 86853009
Killed
*/