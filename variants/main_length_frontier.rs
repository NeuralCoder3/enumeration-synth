use atomic_counter::RelaxedCounter;
use itertools::Itertools;
// use priority_queue::PriorityQueue; // Import itertools crate
use std::collections::{HashSet, VecDeque, HashMap};
// use std::cmp::Reverse;
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use dynqueue::IntoDynQueue as _;
use atomic_counter::AtomicCounter;

const NUMBERS: usize = 3;
const SWAPS: usize = 1;
const REGS: usize = NUMBERS + SWAPS;
const CMP: usize = 0;
const MOV: usize = 1;
const CMOVG: usize = 2;
const CMOVL: usize = 3;

// Represents a command: (instruction, to, from)
type Command = (usize, usize, usize);

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

fn apply(cmd: &Command, perm: &mut [usize]) {
    let (instr, to, from) = *cmd;
    match instr {
        CMP => {
            perm[REGS + 0] = (perm[to] < perm[from]) as usize;
            perm[REGS + 1] = (perm[to] > perm[from]) as usize;
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

fn apply_all(cmd: &Command, state: &[Vec<usize>]) -> Vec<Vec<usize>> {
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

fn viable(state: &[Vec<usize>]) -> bool {
    for perm in state {
        for n in 1..=NUMBERS {
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

fn main() {
    let possible_cmds = possible_commands();
    let permutations: Vec<Vec<usize>> = (1..=NUMBERS).permutations(NUMBERS).collect(); // Use itertools permutations

    // let mut queue = VecDeque::new();
    // let mut queue : VecDeque<_> = VecDeque::new();
    // let mut seen = HashSet::new();
    let mut visited = RelaxedCounter::new(0 as usize);
    let mut duplicate = RelaxedCounter::new(0 as usize);

    // index (visited) -> (operation, previous)
    // let mut info : HashMap<usize, (usize, usize)> = HashMap::new();
    // state -> (operation, previous)
    // let mut info : HashMap<Vec<Vec<usize>>, (Command, Vec<Vec<usize>>)> = HashMap::new();
    // let mut program_length_map : HashMap<Vec<Vec<usize>>, usize> = HashMap::new();

    let initial_state: Vec<Vec<usize>> = permutations
        .iter()
        .map(|p| {
            let mut perm = p.clone();
            perm.extend(&[0; SWAPS]);
            perm.extend(&[0, 0]); // Flags
            perm
        })
        .collect();

    // queue.push_back((initial_state.clone(),0));
    // queue.push(initial_state.clone(), 100-initial_state.len() as i32);
    // queue.push(initial_state.clone(), Reverse(0 as usize));
    // seen.insert(initial_state.clone()); // Insert vector instead of HashSet

    println!("Starting search");

    // let mut final_states = Vec::new();
    let goal_perm = initial_state[0][0..NUMBERS].to_vec();


    let seen = flurry::HashSet::new();
    let guard = seen.guard();

    let mut frontier: Vec<Vec<Vec<usize>>> = vec![initial_state.clone()];

    let mut length = 0;
    'outer:
    while length<20 {
        let mut new_frontier = Vec::new();
        for state in frontier {
            visited.inc();
            if visited.get() % 1000 == 0 {
                println!("Visited: {}, Duplicate: {} (length: {})", visited.get(), duplicate.get(), length);
            }
            if state.iter().all(|p| p[0..NUMBERS] == goal_perm) {
                println!("Found: {:?} of length: {}", state, length);
                break 'outer;
            }

            for cmd in &possible_cmds {
                let new_state = apply_all(cmd, &state);

                if !viable(&new_state) {
                    continue;
                }
                if seen.contains(&new_state, &guard) {
                    duplicate.inc();
                    continue;
                }
                seen.insert(new_state.clone(), &guard);

                new_frontier.push(new_state);
            }
        }
        length += 1;
        frontier = new_frontier;
    }

    println!("Visited: {}, Duplicate: {}", visited.get(), duplicate.get());

    // vec![(initial_state.clone(),0)]
    // .into_dyn_queue()
    // .into_par_iter()
    // .for_each(|(handler, (state,prog_len))| {
    //     visited.inc();
    //     if visited.get() % 1000 == 0 {
    //         println!("Visited: {}, Duplicate: {}", visited.get(), duplicate.get());
    //     }
    //     if state.iter().all(|p| p[0..NUMBERS] == goal_perm) {
    //         println!("Found: {:?} of length: {}", state, prog_len);
    //         return;
    //     }

    //     for cmd in &possible_cmds {
    //         let new_state = apply_all(cmd, &state);

    //         if !viable(&new_state) {
    //             continue;
    //         }
    //         if seen.contains(&new_state, &guard) {
    //             duplicate.inc();
    //             continue;
    //         }
    //         seen.insert(new_state.clone(), &guard);

    //         handler.enqueue((new_state, prog_len + 1));
    //     }
    // });


}