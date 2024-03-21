use itertools::Itertools;
use std::collections::HashSet;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use rand::seq::SliceRandom;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    // let permutations = permutations.choose_multiple(&mut rand::thread_rng(), 10).cloned().collect::<Vec<_>>();
    // let permutations = permutations.into_iter().take(10).collect::<Vec<_>>();

    let mut visited = 0;
    let mut duplicate = 0;

    let initial_state: State = permutations
        .iter()
        .map(|p| {
            // let mut perm = p.clone();
            // perm.extend(&[0; SWAPS]);
            // perm.extend(&[0, 0]); // Flags
            // perm

            // as array
            let mut perm = [0; REGS + 2];
            for (i, &x) in p.iter().enumerate() {
                perm[i] = x;
            }
            perm
        })
        .collect();

    println!("Starting search");
    // let goal_perm = initial_state[0][0..NUMBERS].to_vec();


    let mut seen = HashSet::new();

    let mut frontier: Vec<(Node,State)> = vec![(Node{cmd: (0,0,0), prev: None}, initial_state)];
    let start_time = std::time::Instant::now();

    let mut length = 0;
    while length<20 {

        println!("Length: {}", length);
        println!("Frontier: {}", frontier.len());

        println!("Check solutions");

        let solutions = 
            frontier
            .iter()
            // .filter(|(_,state)| state.iter().all(|p| p[0..NUMBERS] == goal_perm))
            .filter(|(_,state)| state.iter().all(|p| p[0..NUMBERS] == state[0][0..NUMBERS]))
            .collect::<Vec<_>>();
        if solutions.len() > 0 {
            println!("Found: {:?} of length: {}", solutions.len(), length);
            let elapsed = start_time.elapsed();
            println!("Elapsed: {:?}", elapsed);
            break;
        }

        println!("Compute new frontier");

        let frontier_len = frontier.len();
        visited += frontier_len;
        let new_frontier =
            frontier
            .into_par_iter()
            // .into_iter()
            .flat_map(|(prg,state)| {
                // if state.iter().all(|p| p[0..NUMBERS] == goal_perm) {
                //     println!("Found: {:?} of length: {}", state, length);
                //     let elapsed = start_time.elapsed();
                //     println!("Elapsed: {:?}", elapsed);
                //     // solution_lengths.lock().unwrap().push(length);
                //     println!("a bit older: Visited: {}, Duplicate: {}", visited, duplicate);

                //     // reconstruct program
                //     let mut prg = prg;
                //     let mut cmds = vec![];
                //     while let Some(node) = prg.prev {
                //         cmds.push(prg.cmd);
                //         prg = *node;
                //     }
                //     cmds.reverse();
                //     println!("Program:");
                //     for cmd in cmds {
                //         println!("{}", show_command(&cmd));
                //     }

                //     std::process::exit(0);
                // }

                let prev_box = Some(Box::new(prg));

                possible_cmds
                    .iter()
                    .filter_map(|cmd| {
                        let new_state = apply_all(cmd, &state);

                        if !viable(&new_state) {
                            return None;
                        }

                        // let eq_repr = new_state.iter().map(|p| perm_positions(p)).collect::<Vec<_>>();
                        let eq_repr = Arc::new(state_positions(&new_state));
                        if seen.contains(&eq_repr) {
                            return None;
                        }
                        // if seen.contains(&new_state) {
                        //     return None;
                        // }

                        let prg = Node{cmd: *cmd, prev: prev_box.clone()};
                        // Some((prg,new_state))
                        Some((prg,eq_repr,new_state))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        println!("Filter out duplicates");
        let frontier_filtered = new_frontier
            // filter seen (as seen is not updated sequentially, we dedup manually)
            .into_iter()
            .filter(|(_,eq_repr,_)| {
                // return !seen.contains(state);
                // if seen.contains(state) {
                //     return false;
                // }
                // important for runtime
                // seen.insert(state.clone());
                if seen.contains(eq_repr) {
                    return false;
                }
                // seen.insert(eq_repr.clone());
                seen.insert(Arc::clone(eq_repr));
                true
            })
            .collect::<Vec<_>>();
        duplicate += frontier_len * possible_cmds.len() - frontier_filtered.len();
        println!("Visited: {}, Duplicate: {} (length: {})", visited, duplicate, length);

        // add all to seen
        // seen.extend(frontier_filtered.iter().map(|(_,eq_repr,_)| eq_repr.clone()));
        // if solution_lengths.lock().unwrap().len() > 0 {
        //     println!("Found: {:?} of length: {}", solution_lengths.lock().unwrap(), length);
        //     break;
        // }
        length += 1;
        frontier = frontier_filtered.into_iter().map(|(len,_,state)| (len,state)).collect();
    }

    println!("Visited: {}, Duplicate: {}", visited, duplicate);
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



// flag as bool arr: 25s
// perm as vec: 16s
// perm as u8 arr: 14s