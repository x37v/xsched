use crate::sched::Sched;
pub struct Jack {
    client: Option<jack::AsyncClient<Notifications, SchedProcessHandler>>,
}

struct SchedProcessHandler {
    sched: Sched,
}

impl jack::ProcessHandler for SchedProcessHandler {
    fn process(&mut self, client: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.sched
            .run(ps.n_frames() as usize, client.sample_rate() as usize);
        jack::Control::Continue
    }
}

impl Jack {
    pub fn new(mut sched: Sched) -> Self {
        let (client, _status) =
            jack::Client::new("xsched", jack::ClientOptions::NO_START_SERVER).unwrap();
        let mut midi_out = client
            .register_port("midi", jack::MidiOut::default())
            .expect("failed to create midi out port");
        let notify = Notifications::new();
        let handler = SchedProcessHandler { sched };

        // Activate the client, which starts the processing.
        let client = client
            .activate_async(notify, handler)
            .expect("failed to create client");

        Self {
            client: Some(client),
        }
    }
}

struct Notifications {}

impl Notifications {
    pub fn new() -> Self {
        Notifications {}
    }
}

impl jack::NotificationHandler for Notifications {}

impl Drop for Jack {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            let _ = client.deactivate();
        }
    }
}
