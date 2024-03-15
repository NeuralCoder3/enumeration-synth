// use atomic_counter::RelaxedCounter;
// use itertools::Either;
use itertools::Itertools;
use std::collections::{HashSet, VecDeque, HashMap};
use rayon::iter::IntoParallelIterator as _;
use rayon::iter::ParallelIterator as _;
// use dynqueue::IntoDynQueue as _;
// use atomic_counter::AtomicCounter;
use std::sync::Arc;
use std::sync::Mutex;
// use std::thread;

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
    let permutations: Vec<Vec<u8>> = (1..=NUMBERS_U8).permutations(NUMBERS).collect(); // Use itertools permutations

    // let mut queue = VecDeque::new();
    // let mut queue : VecDeque<_> = VecDeque::new();
    // let mut seen = HashSet::new();
    // let visited = RelaxedCounter::new(0 as usize);
    // let duplicate = RelaxedCounter::new(0 as usize);
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


    let mut seen = HashSet::new();
    // let guard = seen.guard();
    // let seen = Arc::new(Mutex::new(HashSet::new()));
    let solution_lengths = Arc::new(Mutex::new(Vec::new()));

    let mut frontier: Vec<State> = vec![initial_state.clone()];
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
            .flat_map(|state| {
                // visited.inc();
                // if visited.get() % 1000 == 0 {
                //     println!("Visited: {}, Duplicate: {} (length: {})", visited.get(), duplicate.get(), length);
                // }
                if state.iter().all(|p| p[0..NUMBERS] == goal_perm) {
                    println!("Found: {:?} of length: {}", state, length);
                    let elapsed = start_time.elapsed();
                    println!("Elapsed: {:?}", elapsed);
                    solution_lengths.lock().unwrap().push(length);
                    // exit program
                    println!("a bit older: Visited: {}, Duplicate: {}", visited, duplicate);
                    std::process::exit(0);
                    // return vec![state];
                }

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
        duplicate += new_frontier_length - frontier_filtered.len();
        println!("Visited: {}, Duplicate: {} (length: {})", visited, duplicate, length);

        // add all to seen
        seen.extend(frontier_filtered.iter().cloned());
        if solution_lengths.lock().unwrap().len() > 0 {
            println!("Found: {:?} of length: {}", solution_lengths.lock().unwrap(), length);
            break;
        }
        length += 1;
        frontier = frontier_filtered;
    }

    println!("Visited: {}, Duplicate: {}", visited, duplicate);

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