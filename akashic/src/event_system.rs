use tokio::sync::broadcast;

// 待补充
#[derive(Debug, Clone, PartialEq)]
pub enum Event {}

pub struct EventChannel {
    channel: broadcast::Sender<Event>,
}

impl EventChannel {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<Event>(10);
        Self { channel: tx }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.channel.subscribe()
    }
    pub fn send(&self, event: Event) {
        self.channel.send(event).ok();
    }
}
