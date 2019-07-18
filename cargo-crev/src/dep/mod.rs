use rayon::prelude::*;

use crate::prelude::*;
use crate::opts::*;
use crate::shared::*;
use crate::term;

mod dep;
mod computer;
mod print_term;

pub use dep::{
    Progress, TableComputationStatus, DepComputationStatus, CrateCounts, TrustCount,
    Dep, ComputedDep, DepTable,
};
pub use computer::DepComputer;
use print_term::*;

pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {

    let mut term = term::Term::new();

    let mut table = DepTable::new();
    if term.stderr_is_tty && term.stdout_is_tty {
        term_print_header(&mut term, args.verbose);
    }
    let computer = DepComputer::new(&args)?;
    let rx_events = computer.run_computation();

    loop {
        let event = match rx_events.recv() {
            Ok(event) => event,
            Err(_) => {
                break;
            }
        };
        //println!("got event, status={:?}", &event.computation_status);
        if let Some(dep) = &event.finished_dep {
            term_print_dep(&dep, &mut term, args.verbose)?;
        }
        table.update(event);
        if table.is_computation_finished() {
            break;
        }
    }

    let mut nb_unclean_digests = 0;
    let mut nb_unverified = 0;
    for dep in table.deps.iter() {
        if let DepComputationStatus::Ok{computed_dep} = &dep.computation_status {
            if computed_dep.unclean_digest {
                nb_unclean_digests += 1;
            }
            if !computed_dep.verified {
                nb_unverified += 1;
            }
        }
    }

    if nb_unclean_digests > 0 {
        println!(
            "{} unclean package{} detected. Use `cargo crev clean <crate>` to wipe the local source.",
            if nb_unclean_digests > 1 { "s" } else { "" },
            nb_unclean_digests
        );
        for dep in table.deps {
            if dep.is_digest_unclean() {
                term.eprint(
                    format_args!("Unclean crate {} {}\n", &dep.name, &dep.version),
                    ::term::color::RED,
                )?;
            }
        }
    }

    Ok(
        if nb_unverified == 0 {
            CommandExitStatus::Successs
        } else {
            CommandExitStatus::VerificationFailed
        }
    )
}

