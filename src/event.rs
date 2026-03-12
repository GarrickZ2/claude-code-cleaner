use crossterm::event::{self, Event as CEvent, KeyEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    ScanMessage(crate::scanner::ScanMessage),
    CleanMessage(crate::cleaner::CleanMessage),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> (Self, mpsc::UnboundedSender<Event>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        // Spawn input polling thread
        std::thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                // Calculate remaining time until next tick
                let timeout = tick_rate.saturating_sub(last_tick.elapsed());

                if event::poll(timeout).unwrap_or(false) {
                    if let Ok(CEvent::Key(key)) = event::read() {
                        if event_tx.send(Event::Key(key)).is_err() {
                            return;
                        }
                    }
                }

                // Only send Tick at the actual tick rate, not on every poll cycle
                if last_tick.elapsed() >= tick_rate {
                    if event_tx.send(Event::Tick).is_err() {
                        return;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        (Self { rx }, tx)
    }

    /// Wait for the first event, then drain all remaining buffered events.
    /// Returns them all at once so the caller can process the batch before rendering.
    pub async fn next_batch(&mut self) -> Vec<Event> {
        let mut batch = Vec::new();

        // Block-wait for the first event
        match self.rx.recv().await {
            Some(evt) => batch.push(evt),
            None => return batch,
        }

        // Drain everything else that's already buffered (non-blocking)
        while let Ok(evt) = self.rx.try_recv() {
            batch.push(evt);
        }

        batch
    }
}
