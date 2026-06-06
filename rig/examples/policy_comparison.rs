use rig::{Arena, GrowthPolicy, RigVec};

const PUSHES: usize = 50_000;

fn run_policy(policy: GrowthPolicy, label: &str) -> Arena {
    run_policy_for_pushes(policy, label, PUSHES)
}

fn run_policy_for_pushes(policy: GrowthPolicy, label: &str, pushes: usize) -> Arena {
    let mut arena = Arena::new(format!("policy-comparison-{label}"));
    let mut values = RigVec::with_policy(&mut arena, "values", policy);

    for value in 0..pushes {
        values.push(value);
    }

    arena
}

fn print_summary_row(arena: &Arena) {
    let snapshot = arena.snapshot();
    let container = &snapshot.containers[0];
    let summary = snapshot.growth_summary();
    println!(
        "{:<22} {:>8} {:>10} {:>13} {:>14} {:>10}",
        container.growth_policy,
        container.len,
        container.current_capacity,
        summary.total_growth_events,
        summary.largest_growth_delta,
        container.total_operations
    );
}

fn main() {
    let policies = [
        (GrowthPolicy::RustDefault, "rust-default"),
        (GrowthPolicy::Double, "double"),
        (GrowthPolicy::Exact, "exact"),
        (GrowthPolicy::ReserveAhead(1024), "reserve-ahead"),
    ];

    let arenas = policies
        .into_iter()
        .map(|(policy, label)| run_policy(policy, label))
        .collect::<Vec<_>>();

    println!("Policy comparison summary ({PUSHES} pushes per policy)");
    println!(
        "{:<22} {:>8} {:>10} {:>13} {:>14} {:>10}",
        "policy", "len", "capacity", "growth_events", "largest_delta", "operations"
    );
    for arena in &arenas {
        print_summary_row(arena);
    }

    println!(
        "\nCompact human reports (raw growth history is preserved in JSON and verbose reports):"
    );
    for arena in &arenas {
        println!("\n{}", arena.report());
    }

    println!("\nJSON report sample (Double policy, includes raw growth_history):");
    println!("{}", arenas[1].report_json());

    println!("\nVerbose report sample (intentionally small Exact workload):");
    let small_exact = run_policy_for_pushes(GrowthPolicy::Exact, "small-exact-verbose", 4);
    println!("{}", small_exact.report_verbose());

    let mut capped_arena = Arena::new("policy-comparison-capped");
    let mut capped = RigVec::with_policy(
        &mut capped_arena,
        "capped_values",
        GrowthPolicy::Capped { max_capacity: 8 },
    );
    for value in 0..8 {
        capped.try_push(value).expect("value fits capped policy");
    }
    match capped.try_push(8) {
        Ok(()) => println!("capped workload unexpectedly succeeded"),
        Err(error) => println!("Capped failure: {error}"),
    }
    println!("\n{}", capped_arena.report());
}
