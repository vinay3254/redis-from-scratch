use crate::resp::RespFrame;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

pub struct PubSub {
    channels: HashMap<Vec<u8>, HashMap<u64, mpsc::Sender<RespFrame>>>,
    client_subscriptions: HashMap<u64, HashSet<Vec<u8>>>,
    next_client_id: u64,
}

impl PubSub {
    pub fn new() -> Self {
        PubSub {
            channels: HashMap::new(),
            client_subscriptions: HashMap::new(),
            next_client_id: 1,
        }
    }

    pub fn generate_client_id(&mut self) -> u64 {
        let id = self.next_client_id;
        self.next_client_id += 1;
        id
    }

    pub fn publish(&mut self, channel: &[u8], message: &[u8]) -> usize {
        let mut receivers_count = 0;
        if let Some(subscribers) = self.channels.get_mut(channel) {
            let msg_frame = RespFrame::Array(Some(vec![
                RespFrame::BulkString(Some(b"message".to_vec())),
                RespFrame::BulkString(Some(channel.to_vec())),
                RespFrame::BulkString(Some(message.to_vec())),
            ]));

            let mut dead_clients = Vec::new();

            for (&client_id, tx) in subscribers.iter() {
                if tx.send(msg_frame.clone()).is_ok() {
                    receivers_count += 1;
                } else {
                    dead_clients.push(client_id);
                }
            }

            for dead_id in dead_clients {
                subscribers.remove(&dead_id);
            }
        }
        receivers_count
    }

    pub fn subscribe(
        &mut self,
        client_id: u64,
        requested_channels: &[Vec<u8>],
        tx: mpsc::Sender<RespFrame>,
    ) -> Vec<RespFrame> {
        let mut responses = Vec::with_capacity(requested_channels.len());

        let client_subs = self
            .client_subscriptions
            .entry(client_id)
            .or_insert_with(HashSet::new);

        for ch in requested_channels {
            client_subs.insert(ch.clone());
            self.channels
                .entry(ch.clone())
                .or_insert_with(HashMap::new)
                .insert(client_id, tx.clone());

            let count = client_subs.len();
            responses.push(RespFrame::Array(Some(vec![
                RespFrame::BulkString(Some(b"subscribe".to_vec())),
                RespFrame::BulkString(Some(ch.clone())),
                RespFrame::Integer(count as i64),
            ])));
        }

        responses
    }

    pub fn unsubscribe(
        &mut self,
        client_id: u64,
        requested_channels: &[Vec<u8>],
    ) -> Vec<RespFrame> {
        let mut responses = Vec::new();

        let channels_to_unsub: Vec<Vec<u8>> = if requested_channels.is_empty() {
            if let Some(subs) = self.client_subscriptions.get(&client_id) {
                subs.iter().cloned().collect()
            } else {
                Vec::new()
            }
        } else {
            requested_channels.to_vec()
        };

        if channels_to_unsub.is_empty() {
            responses.push(RespFrame::Array(Some(vec![
                RespFrame::BulkString(Some(b"unsubscribe".to_vec())),
                RespFrame::BulkString(None),
                RespFrame::Integer(0),
            ])));
            return responses;
        }

        for ch in channels_to_unsub {
            let mut remaining = 0;
            if let Some(client_subs) = self.client_subscriptions.get_mut(&client_id) {
                client_subs.remove(&ch);
                remaining = client_subs.len();
            }

            if let Some(subscribers) = self.channels.get_mut(&ch) {
                subscribers.remove(&client_id);
                if subscribers.is_empty() {
                    self.channels.remove(&ch);
                }
            }

            responses.push(RespFrame::Array(Some(vec![
                RespFrame::BulkString(Some(b"unsubscribe".to_vec())),
                RespFrame::BulkString(Some(ch)),
                RespFrame::Integer(remaining as i64),
            ])));
        }

        responses
    }

    pub fn remove_client(&mut self, client_id: u64) {
        if let Some(subs) = self.client_subscriptions.remove(&client_id) {
            for ch in subs {
                if let Some(subscribers) = self.channels.get_mut(&ch) {
                    subscribers.remove(&client_id);
                    if subscribers.is_empty() {
                        self.channels.remove(&ch);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubsub_basic() {
        let mut ps = PubSub::new();
        let client_id = ps.generate_client_id();

        let (tx, rx) = mpsc::channel();

        let subs = ps.subscribe(client_id, &[b"ch1".to_vec()], tx);
        assert_eq!(subs.len(), 1);

        assert_eq!(ps.publish(b"ch1", b"hello"), 1);
        let msg = rx.recv().unwrap();

        let expected_msg = RespFrame::Array(Some(vec![
            RespFrame::BulkString(Some(b"message".to_vec())),
            RespFrame::BulkString(Some(b"ch1".to_vec())),
            RespFrame::BulkString(Some(b"hello".to_vec())),
        ]));
        assert_eq!(msg, expected_msg);

        let unsubs = ps.unsubscribe(client_id, &[b"ch1".to_vec()]);
        assert_eq!(unsubs.len(), 1);

        assert_eq!(ps.publish(b"ch1", b"hello2"), 0);
    }
}
