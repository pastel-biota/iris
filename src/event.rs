use crate::model::Identifier;

#[derive(Debug, Clone)]
pub enum Event {
    PhotoRegistered { photo_id: Identifier },
}

pub type EventSender = tokio::sync::mpsc::Sender<Event>;
pub type EventReceiver = tokio::sync::mpsc::Receiver<Event>;

pub fn create_event_bus(capacity: usize) -> (EventSender, EventReceiver) {
    tokio::sync::mpsc::channel(capacity)
}
