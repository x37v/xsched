use crate::sched::{MidiValueQueue, Sched};
use sched::pqueue::TickPriorityDequeue;
pub struct Jack {
    client: Option<jack::AsyncClient<Notifications, SchedProcessHandler>>,
}

struct SchedProcessHandler {
    sched: Sched,
    midi_out: jack::Port<jack::MidiOut>,
    midi_queue: MidiValueQueue,
}

impl jack::ProcessHandler for SchedProcessHandler {
    fn process(&mut self, client: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        //get 'now' at the start of this frame.
        let now = self.sched.tick_next();
        self.sched
            .run(ps.n_frames() as usize, client.sample_rate() as usize);
        {
            let mut midi_out = self.midi_out.writer(ps);
            let mut write_midi = |tick: u32, bytes: &[u8]| {
                let _ = midi_out.write(&jack::RawMidi { time: tick, bytes });
            };
            let mut midi_queue = self.midi_queue.lock();

            //get all midi events that should be scheduled within this frame
            let next = self.sched.tick_next();
            while let Some((t, midi)) = midi_queue.dequeue_lt(next) {
                //compute the tick offset from the start of the frame
                let tick = (std::cmp::max(t, now) - now) as u32;
                let iter = &mut midi.iter();
                match iter.len() {
                    3 => write_midi(
                        tick,
                        &[
                            iter.next().unwrap(),
                            iter.next().unwrap(),
                            iter.next().unwrap(),
                        ],
                    ),
                    2 => write_midi(tick, &[iter.next().unwrap(), iter.next().unwrap()]),
                    1 => write_midi(tick, &[iter.next().unwrap()]),
                    _ => (),
                };
            }
        }
        jack::Control::Continue
    }
}

impl Jack {
    pub fn new(sched: Sched) -> Self {
        let (client, _status) =
            jack::Client::new("xsched", jack::ClientOptions::NO_START_SERVER).unwrap();
        let midi_out = client
            .register_port("midi", jack::MidiOut::default())
            .expect("failed to create midi out port");
        let notify = Notifications::new();
        let midi_queue = sched.midi_queue();
        let handler = SchedProcessHandler {
            sched,
            midi_out,
            midi_queue,
        };

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
