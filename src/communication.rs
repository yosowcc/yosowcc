use std::collections::BTreeMap;

#[derive(Default)]
pub struct BroadcastChannel {
    ///Stores all messages that were broadcast as a tuple (round sent, messages from that round) 
    messages: BTreeMap<u64, Vec<BroadcastMessage>>
}

impl BroadcastChannel {
    pub fn store_round(&self, round: u64, messages: &Vec<BroadcastMessage>) {
        //self.messages.insert(round, messages.to_vec());
    }

    pub fn read_round(&self, round: u64) -> Option<&Vec<BroadcastMessage>> {
        self.messages.get(&round)
    }
}

#[derive(Clone)]
pub struct PrivateChannel {
    ///Stores the message that were sent in this private channel (round sent, payload>
    message: Option<Vec<u8>>
}

impl PrivateChannel {
    pub fn store_msg(&self, message: &Vec<u8>) {
       // self.message = Some(message.to_vec());
    }

    pub fn read_msg_from_round(&self) -> &Option<Vec<u8>> {
        &self.message
    }
}


#[derive(Clone)]
pub struct BroadcastMessage {
    sender: u64,
    payload: Vec<u8>
}