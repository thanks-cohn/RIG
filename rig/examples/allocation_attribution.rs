use rig::{Arena, GrowthPolicy, RigString, RigVec};

fn main() {
    let mut arena = Arena::new("allocation-attribution");
    let mut raw_log_buffer = RigString::with_policy(
        &mut arena,
        "raw_log_buffer",
        GrowthPolicy::ReserveAhead(16 * 1024),
    );
    let mut values = RigVec::with_policy(&mut arena, "values", GrowthPolicy::Double);
    let mut mixed_workload = RigVec::with_policy(&mut arena, "mixed_workload", GrowthPolicy::Exact);

    for batch in 0..64 {
        raw_log_buffer.push_str(&"x".repeat(4096 + batch));
    }

    for value in 0..8192 {
        values.push(value);
    }

    for value in 0..128 {
        mixed_workload.push((value, value * 2));
        if value % 16 == 0 {
            raw_log_buffer.push_str("mixed-workload checkpoint\n");
        }
    }

    let report = arena.snapshot();

    println!("Top growth contributors:");
    for (index, container) in report
        .top_growth_containers()
        .into_iter()
        .filter(|container| container.total_capacity_added > 0)
        .enumerate()
    {
        println!(
            "{}. {}\n   total capacity added: {}",
            index + 1,
            container.name,
            container.total_capacity_added
        );
    }

    println!("\nAttribution events:");
    for attribution in &report.growth_attributions {
        println!(
            "{} operation {}: {} -> {} (+{}) under {}",
            attribution.container_name,
            attribution.operation_index,
            attribution.old_capacity,
            attribution.new_capacity,
            attribution.capacity_added,
            attribution.growth_policy
        );
    }

    println!("\nReport JSON:");
    println!("{}", report.report_json());
}
