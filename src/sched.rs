use ::sched::{
    event::EventContainer,
    item_sink::{ItemDispose, ItemSink},
    item_source::ItemSource,
    midi::MidiValue,
    pqueue::{BinaryHeapQueue, TickPriorityDequeue, TickPriorityEnqueue},
    schedule::ScheduleExecutor,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub type ArcMutex<T> = ::std::sync::Arc<::sched::mutex::Mutex<T>>;

type SchedEnqueue = ArcMutex<dyn TickPriorityEnqueue<EventContainer>>;
type SchedDequeue = ArcMutex<dyn TickPriorityDequeue<EventContainer>>;
type EventSink = ArcMutex<dyn ItemSink<EventContainer>>;

pub type MidiValueQueue = ArcMutex<BinaryHeapQueue<MidiValue>>;
pub type MidiEventSource = ArcMutex<dyn ItemSource<TickedMidiValueEvent>>;

pub type EventQueue = ArcMutex<BinaryHeapQueue<EventContainer>>;

pub struct Sched {
    fill_dispose_continue: Arc<AtomicBool>,
    fill_dispose_handle: Option<std::thread::JoinHandle<()>>,
    executor: ScheduleExecutor<SchedDequeue, SchedEnqueue, EventSink>,
    queue_sources: Arc<SchedQueueSources>,
}

struct SchedQueueSources {
    midi_queue: MidiValueQueue,
    midi_event_source: MidiEventSource,
    sched_queue: EventQueue,
}

pub trait IntoPtrs {
    fn into_arc(self) -> Arc<Self>;
    fn into_alock(self) -> ArcMutex<Self>;
}

pub trait QueueSource {
    fn midi_queue(&self) -> MidiValueQueue;
    fn midi_event_source(&self) -> MidiEventSource;
    fn sched_queue(&self) -> EventQueue;
}

impl<T> IntoPtrs for T
where
    T: Sized,
{
    fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
    fn into_alock(self) -> ArcMutex<Self> {
        Arc::new(::sched::mutex::Mutex::new(self))
    }
}

type MidiEnqueue = ArcMutex<dyn TickPriorityEnqueue<MidiValue>>;
type TickedMidiValueEvent = ::sched::graph::midi::TickedMidiValueEvent<MidiEnqueue>;

impl Sched {
    pub fn new() -> Self {
        let midi_queue: MidiValueQueue = Default::default();
        let sched_queue: ArcMutex<BinaryHeapQueue<EventContainer>> = Default::default();

        let (dispose_sink, dispose) = ::sched::std::channel_item_sink::channel_item_sink(1024);
        let dispose_sink: ArcMutex<dyn ItemSink<EventContainer>> = dispose_sink.into_alock();

        let (mut midi_creator, midi_event_source) =
            ::sched::std::channel_item_source::item_source(1024);
        let midi_event_source: MidiEventSource =
            Arc::new(::sched::mutex::Mutex::new(midi_event_source));

        let ex = ScheduleExecutor::new(
            dispose_sink,
            sched_queue.clone() as SchedDequeue,
            sched_queue.clone() as SchedEnqueue,
        );

        let fill_dispose_continue = Arc::new(AtomicBool::new(true));

        let mut fill_dispose = move || {
            midi_creator.fill().expect("failed to fill midi");
            dispose.dispose_all().expect("dispose failed");
        };

        let fill_dispose_handle = {
            fill_dispose();
            let fill_dispose_continue = fill_dispose_continue.clone();
            std::thread::spawn(move || {
                while fill_dispose_continue.load(Ordering::Acquire) {
                    fill_dispose();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            })
        };

        let queue_sources = Arc::new(SchedQueueSources::new(
            midi_queue,
            midi_event_source,
            sched_queue,
        ));

        Self {
            fill_dispose_handle: Some(fill_dispose_handle),
            fill_dispose_continue,
            executor: ex,
            queue_sources,
        }
    }

    pub fn run(&mut self, frames: usize, sample_rate: usize) {
        self.executor.run(frames, sample_rate);
    }

    pub fn tick_next(&self) -> usize {
        self.executor.tick_next()
    }

    pub fn queue_sources(&self) -> Arc<dyn QueueSource> {
        self.queue_sources.clone()
    }
}

impl SchedQueueSources {
    pub fn new(
        midi_queue: MidiValueQueue,
        midi_event_source: MidiEventSource,
        sched_queue: EventQueue,
    ) -> Self {
        Self {
            midi_queue,
            midi_event_source,
            sched_queue,
        }
    }
}

impl QueueSource for SchedQueueSources {
    fn midi_queue(&self) -> MidiValueQueue {
        self.midi_queue.clone()
    }

    fn midi_event_source(&self) -> MidiEventSource {
        self.midi_event_source.clone()
    }

    fn sched_queue(&self) -> EventQueue {
        self.sched_queue.clone()
    }
}

impl Drop for Sched {
    fn drop(&mut self) {
        self.fill_dispose_continue.store(false, Ordering::Release);
        if let Some(h) = self.fill_dispose_handle.take() {
            h.join().unwrap();
        }
    }
}
