use rayon::prelude::*;

use crate::prelude::*;
use crate::opts::*;
use crate::repo::*;
use crate::shared::*;
use crate::term;

mod dep;
mod computer;

use crate::dep::{dep::*, computer::*};

pub use dep::{
    Progress, TableComputationStatus, RowComputationStatus, CrateCounts, TrustCount,
    DepTable, DepRow,
};

pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {

    let repo = Repo::auto_open_cwd()?;
    let mut term = term::Term::new();

    let package_set = repo.non_local_dep_crates()?;
    let mut table = DepTable::new(&package_set)?;
    if term.stderr_is_tty && term.stdout_is_tty {
        DepRow::term_print_header(&mut term, args.verbose);
    }

    table.rows
        .par_iter_mut()
        .for_each(|row| {
            row.download_if_needed().unwrap(); // FIXME unwrap. What to do exactly apart die ?
            row.count_geiger();
        });

    let mut computer = DepComputer::new(&args)?;

    let mut nb_unclean_digests = 0;
    let mut nb_unverified = 0;
    for row in table.rows.iter_mut() {
        computer.compute(row);
        row.term_print(&mut term, args.verbose)?;
        if let RowComputationStatus::Ok{dep} = &row.computation_status {
            if dep.unclean_digest {
                nb_unclean_digests += 1;
            }
            if !dep.verified {
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
        for row in table.rows {
            if row.is_digest_unclean() {
                let name = row.id.name().as_str();
                let version = row.id.version();
                term.eprint(
                    format_args!("Unclean crate {} {}\n", name, version),
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

