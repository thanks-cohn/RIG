use rig::{Arena, RigString, RigVec};
use std::collections::{HashMap, HashSet, VecDeque};

const WIDTH: usize = 180;
const HEIGHT: usize = 180;
type Point = (usize, usize);

fn neighbors((x, y): Point) -> impl Iterator<Item = Point> {
    let mut points = Vec::with_capacity(4);
    if x > 0 {
        points.push((x - 1, y));
    }
    if y > 0 {
        points.push((x, y - 1));
    }
    if x + 1 < WIDTH {
        points.push((x + 1, y));
    }
    if y + 1 < HEIGHT {
        points.push((x, y + 1));
    }
    points.into_iter()
}

fn is_open((x, y): Point) -> bool {
    x == 0 || y == HEIGHT - 1 || (x + (y * 3)) % 17 != 0
}

fn main() {
    let mut arena = Arena::new("pathfinding");

    let mut frontier = RigVec::new(&mut arena, "frontier");
    let mut visited = RigVec::new(&mut arena, "visited");
    let mut came_from = RigVec::new(&mut arena, "came_from");
    let mut path = RigVec::new(&mut arena, "path");
    let mut search_log = RigString::new(&mut arena, "search_log");

    let before = arena.snapshot();

    let start = (0, 0);
    let goal = (WIDTH - 1, HEIGHT - 1);
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    let mut parents: HashMap<Point, Point> = HashMap::new();

    queue.push_back(start);
    seen.insert(start);
    frontier.push(start);
    visited.push(start);
    search_log.push_str(&format!("start={start:?} goal={goal:?}\n"));

    while let Some(current) = queue.pop_front() {
        if current == goal {
            search_log.push_str(&format!("goal reached at {current:?}\n"));
            break;
        }

        for next in neighbors(current) {
            if is_open(next) && seen.insert(next) {
                queue.push_back(next);
                parents.insert(next, current);
                frontier.push(next);
                visited.push(next);
                came_from.push((next, current));
            }
        }
    }

    let mut reversed_path = Vec::new();
    if seen.contains(&goal) {
        let mut current = goal;
        reversed_path.push(current);
        while current != start {
            current = parents[&current];
            reversed_path.push(current);
        }
        for point in reversed_path.iter().rev().copied() {
            path.push(point);
        }
        search_log.push_str(&format!("path length={}\n", path.len()));
    } else {
        search_log.push_str("path length=0\n");
    }

    let after = arena.snapshot();
    let diff = before.diff(&after);

    println!("Path found: {}", !path.is_empty());
    println!("Path length: {}", path.len());
    println!("Visited nodes: {}", visited.len());
    println!();
    println!("{}", arena.report());
    println!();
    println!("{}", diff.report());
    println!();
    println!("Growth history count: {}", after.growth_history.len());
    println!();
    println!("{}", after.report_json());
}
