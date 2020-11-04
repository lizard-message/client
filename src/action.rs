use smol::channel::Sender;

#[derive(Debug)]
pub(super) enum Action {
    Sub {
        sub_name: String,
        msg_sender: Sender<String>,
    },
    Pub {
        sub_name: String,
        payload: Vec<u8>,
    },
}
