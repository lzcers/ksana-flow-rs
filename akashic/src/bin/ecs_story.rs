use std::time::Duration;

use akashic::system::{
    RoundStatus, StoryRuntime, build_default_story_world, build_story_schedule,
};
use bevy_ecs::world::World;
use tokio::time::sleep;

fn print_new_narrations(world: &mut World, entity: bevy_ecs::entity::Entity, printed: &mut usize) {
    let Some(runtime) = world.get::<StoryRuntime>(entity) else {
        return;
    };

    for round in runtime.history.iter().skip(*printed) {
        if let Some(narration) = &round.narration {
            println!("\n=== Round {} ===", narration.round);
            println!("{}", narration.title);
            println!("{}", narration.content);
        }
    }

    *printed = runtime.history.len();
}

#[tokio::main]
async fn main() {
    let (mut world, story_entity) = build_default_story_world(3);
    let mut schedule = build_story_schedule();
    let mut printed_rounds = 0usize;

    loop {
        schedule.run(&mut world);
        print_new_narrations(&mut world, story_entity, &mut printed_rounds);

        let Some(runtime) = world.get::<StoryRuntime>(story_entity) else {
            eprintln!("story runtime component missing");
            break;
        };

        match runtime.status {
            RoundStatus::Finished => {
                println!("\n故事已完成，共输出 {} 轮叙事。", runtime.history.len());
                break;
            }
            RoundStatus::Failed => {
                eprintln!(
                    "\n故事运行失败：{}",
                    runtime
                        .last_error
                        .as_deref()
                        .unwrap_or("unknown error")
                );
                break;
            }
            _ => {}
        }

        sleep(Duration::from_millis(50)).await;
    }
}
