use itertools::Itertools;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use std::sync::Arc;
use std::sync::Mutex;

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

#[derive(Eq, PartialEq, Hash,Clone)]
// #[derive(Eq, Clone)]
struct StateStruct {
    state: State,
}

// std
// Elapsed: 54.640449249s
// a bit older: Visited: 10641527, Duplicate: 14499482

// custom: (sort hash, sort eq)
// Elapsed: 65.332813548s
// a bit older: Visited: 10641527, Duplicate: 14499482

// custom: (xor hash, sort eq)


// fn calculate_hash<T: Hash>(t: &T) -> u64 {
//     let mut s = DefaultHasher::new();
//     t.hash(&mut s);
//     s.finish()
// }

// // custom hash and equality that ignores order of permutation
// impl std::hash::Hash for StateStruct {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         // let mut state2 = self.state.clone();
//         // state2.sort();
//         // for perm in state2 {
//         //     for p in perm {
//         //         p.hash(state);
//         //     }
//         // }

//         // without clone using xor
//         let mut hash = 0;
//         for perm in &self.state {
//             for p in perm {
//                 hash ^= calculate_hash(p);
//             }
//         }
//         state.write_u64(hash);
//     }
// }

// impl std::cmp::PartialEq for StateStruct {
//     fn eq(&self, other: &Self) -> bool {
//         let mut state1 = self.state.clone();
//         let mut state2 = other.state.clone();
//         state1.sort();
//         state2.sort();
//         state1 == state2
//     }
// }




fn main() {
    let possible_cmds = possible_commands();
    let permutations: Vec<Vec<u8>> = (1..=NUMBERS_U8).permutations(NUMBERS).collect(); 

    let mut visited = 0;
    let mut duplicate = 0;

    let initial_state: State = permutations
        .iter()
        .map(|p| {
            let mut perm = p.clone();
            perm.extend(&[0; SWAPS]);
            perm.extend(&[0, 0]); // Flags
            perm
        })
        .collect();

    println!("Starting search");
    let goal_perm = initial_state[0][0..NUMBERS].to_vec();


    // let mut seen : HashSet<StateStruct> = HashSet::new();
    let mut seen = HashSet::new();
    let solution_lengths = Arc::new(Mutex::new(Vec::new()));

    let mut frontier: Vec<(Node,StateStruct)> = vec![(Node{cmd: (0,0,0), prev: None}, StateStruct{state: initial_state.clone()})];
    let start_time = std::time::Instant::now();

    let mut length = 0;
    while length<20 {

        println!("Length: {}", length);
        println!("Frontier: {}", frontier.len());

        visited += frontier.len();
        let new_frontier =
            frontier
            .into_par_iter()
            // .into_iter()
            .flat_map(|(prg,state_struct)| {
                let state = &state_struct.state;
                if state.iter().all(|p| p[0..NUMBERS] == goal_perm) {
                    println!("Found: {:?} of length: {}", state, length);
                    let elapsed = start_time.elapsed();
                    println!("Elapsed: {:?}", elapsed);
                    solution_lengths.lock().unwrap().push(length);
                    println!("a bit older: Visited: {}, Duplicate: {}", visited, duplicate);

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

                    std::process::exit(0);
                }

                // let prev_box = Box::new(prg);
                let prev_box = Some(Box::new(prg));

                possible_cmds
                    .iter()
                    .filter_map(|cmd| {
                        let new_state = apply_all(cmd, state);

                        if !viable(&new_state) {
                            return None;
                        }

                        let new_state_struct = StateStruct{state: new_state};
                        if seen.contains(&new_state_struct) {
                            return None;
                        }

                        // if seen.contains(&new_state) {
                        //     return None;
                        // }

                        let prg = Node{cmd: *cmd, prev: prev_box.clone()};
                        Some((prg,new_state_struct))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let new_frontier_length = new_frontier.len();

        println!("Filter out duplicates");
        let frontier_filtered = new_frontier
            // filter seen (as seen is not updated sequentially, we dedup manually)
            .into_iter()
            .filter(|(_,state_struct)| {
                if seen.contains(state_struct) {
                    duplicate += 1;
                    return false;
                }
                seen.insert(state_struct.clone());
                true
            })
            .collect::<Vec<_>>();
        duplicate += new_frontier_length - frontier_filtered.len();
        println!("Visited: {}, Duplicate: {} (length: {})", visited, duplicate, length);

        // add all to seen
        seen.extend(frontier_filtered.iter().map(|(_,state)| state.clone()));
        if solution_lengths.lock().unwrap().len() > 0 {
            println!("Found: {:?} of length: {}", solution_lengths.lock().unwrap(), length);
            break;
        }
        length += 1;
        frontier = frontier_filtered;
    }

    println!("Visited: {}, Duplicate: {}", visited, duplicate);
}