use smol::channel::Sender;

#[derive(Debug)]
pub(super) enum Action {
    Sub {
        sub_name: String,
        msg_sender: Sender<String>,
    },
    Pub,
}
