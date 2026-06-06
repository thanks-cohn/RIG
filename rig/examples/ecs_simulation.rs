use rig::{Arena, RigString, RigVec};

const ENTITY_COUNT: u32 = 100_000;
const FRAME_COUNT: u32 = 60;

fn main() {
    let mut arena = Arena::new("ecs-simulation");

    let mut entities = RigVec::new(&mut arena, "entities");
    let mut positions = RigVec::new(&mut arena, "positions");
    let mut velocities = RigVec::new(&mut arena, "velocities");
    let mut active_entities = RigVec::new(&mut arena, "active_entities");
    let mut frame_log = RigString::new(&mut arena, "frame_log");

    let empty = arena.snapshot();

    let mut position_state = Vec::with_capacity(ENTITY_COUNT as usize);
    let mut velocity_state = Vec::with_capacity(ENTITY_COUNT as usize);

    for entity_id in 0..ENTITY_COUNT {
        let x = (entity_id % 1_000) as f32;
        let y = (entity_id / 1_000) as f32;
        let velocity = (
            ((entity_id % 17) as f32 - 8.0) * 0.01,
            ((entity_id % 29) as f32 - 14.0) * 0.01,
        );

        entities.push(entity_id);
        positions.push((x, y));
        velocities.push(velocity);
        if entity_id % 3 != 0 {
            active_entities.push(entity_id);
        }
        position_state.push((x, y));
        velocity_state.push(velocity);
    }

    let loaded = arena.snapshot();

    let mut accumulated_x = 0.0_f32;
    let mut accumulated_y = 0.0_f32;
    for frame in 0..FRAME_COUNT {
        let delta = 1.0 + frame as f32 * 0.001;
        for entity_id in 0..ENTITY_COUNT as usize {
            if entity_id % 3 != 0 {
                let velocity = velocity_state[entity_id];
                let position = &mut position_state[entity_id];
                position.0 += velocity.0 * delta;
                position.1 += velocity.1 * delta;
            }
        }

        let sample_index = ((frame as usize * 1_541) + 97) % ENTITY_COUNT as usize;
        accumulated_x += position_state[sample_index].0;
        accumulated_y += position_state[sample_index].1;
        frame_log.push_str(&format!(
            "frame={frame}; active={}; sample={sample_index}; x={:.3}; y={:.3}\n",
            active_entities.len(),
            position_state[sample_index].0,
            position_state[sample_index].1
        ));
    }

    let simulated = arena.snapshot();
    let empty_to_loaded = empty.diff(&loaded);
    let loaded_to_simulated = loaded.diff(&simulated);

    println!("ECS entities loaded: {}", entities.len());
    println!("ECS frames simulated: {FRAME_COUNT}");
    println!("ECS active entities: {}", active_entities.len());
    println!(
        "ECS sample checksum: {:.3},{:.3}",
        accumulated_x, accumulated_y
    );
    println!();
    println!("{}", arena.report());
    println!();
    println!("Diff: empty to loaded");
    println!("{}", empty_to_loaded.report());
    println!();
    println!("Diff: loaded to simulated");
    println!("{}", loaded_to_simulated.report());
    println!();
    println!("Growth history count: {}", simulated.growth_history.len());
    println!();
    println!("{}", simulated.report_json());
}
