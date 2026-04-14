use akashic::{
    channel::{AgentChannel, FateWeaverMessage},
    fate_weaver::FateWeaver,
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    protagonist::Protagonist,
    upper_narrator::UpperNarrator,
};



#[tokio::main]
async fn main() {
    let (channel, inboxes) = AgentChannel::new();
    let upper_narrator = UpperNarrator::new();
    let fate_weaver =  FateWeaver::new(
        DEFAULT_PROTAGONIST_PROFILE.to_string(),
        DEFAULT_WORLD_PROFILE.to_string(),
        channel.clone(),
        10,
    );
    let protagonist = Protagonist::new(DEFAULT_PROTAGONIST_PROFILE.to_string());
    let fate_task = tokio::spawn(fate_weaver.start(inboxes.fate_weaver));
    let protagonist_task = tokio::spawn(protagonist.start(inboxes.protagonist));
    let narrator_task = tokio::spawn(upper_narrator.start(inboxes.upper_narrator));
    let _ = channel.send_fate_weaver(FateWeaverMessage::Start).await;

    let _ = fate_task.await;
    let _ = protagonist_task.await;
    let _ = narrator_task.await;
   }
 
