use akashic::{
    channel::{AgentChannel, AgentMessage, FateWeaverMessage},
    fate_weaver::FateWeaver,
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    protagonist::Protagonist,
    upper_narrator::UpperNarrator,
};



#[tokio::main]
async fn main() {
    let channel = AgentChannel::new();
    let upper_narrator = UpperNarrator::new();
    let fate_weaver =  FateWeaver::new(
        DEFAULT_PROTAGONIST_PROFILE.to_string(),
        DEFAULT_WORLD_PROFILE.to_string(),
        channel.clone(),
        10,
    );
    let protagonist = Protagonist::new(DEFAULT_PROTAGONIST_PROFILE.to_string());
    let fate_task = tokio::spawn(fate_weaver.start());
    // let protagonist_task = tokio::spawn(protagonist.start());
    // let narrator_task = tokio::spawn(upper_narrator.start());
    channel.send_msg(AgentMessage::FateWeaver(FateWeaverMessage::Start));

    let _ = fate_task.await;
    // let _ = protagonist_task.await;
    // let _ =narrator_task.await;
   }
 