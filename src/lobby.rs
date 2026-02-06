use std::sync::Arc;
use tokio::sync::broadcast;

/// A generic broadcast channel with optional server-side validation.
///
/// Clone a `Lobby` and pass it into each app instance — they all share
/// the same underlying channel. Messages sent by any app are received
/// by all others.
///
/// # Example
///
/// ```ignore
/// // In main, create the lobby and optionally add validation:
/// let lobby = sshui::Lobby::<ChatMessage>::new(100)
///     .with_validator(|msg| msg.content.len() <= 200);
///
/// // Pass it to each app via the factory closure:
/// sshui::new_server(config, addr, move || {
///     Box::new(App::new(lobby.clone()))
/// }).await?;
/// ```
///
/// ```ignore
/// // Inside your App, subscribe and use try_recv in render:
/// let rx = lobby.subscribe();
///
/// // In draw():
/// while let Ok(msg) = self.rx.try_recv() {
///     self.messages.push(msg);
/// }
///
/// // To send:
/// self.lobby.send(my_message);
/// ```
pub struct Lobby<T: Clone + Send + 'static> {
    tx: broadcast::Sender<T>,
    validator: Arc<dyn Fn(&T) -> bool + Send + Sync>,
}

impl<T: Clone + Send + 'static> Clone for Lobby<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            validator: self.validator.clone(),
        }
    }
}

impl<T: Clone + Send + 'static> Lobby<T> {
    /// Creates a new lobby with the given channel capacity.
    ///
    /// The capacity determines how many messages can be buffered before
    /// slow receivers start missing older ones.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);

        Self {
            tx,
            validator: Arc::new(|_| true),
        }
    }

    /// Adds a validation function that gates every message before broadcast.
    ///
    /// If the validator returns `false`, the message is dropped and `send()` returns `false`.
    pub fn with_validator(mut self, f: impl Fn(&T) -> bool + Send + Sync + 'static) -> Self {
        self.validator = Arc::new(f);
        self
    }

    /// Creates a new receiver for this lobby. Each app should call this once.
    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.tx.subscribe()
    }

    /// Sends a message to all subscribers, if it passes validation.
    ///
    /// Returns `true` if the message was broadcast, `false` if rejected.
    pub fn send(&self, msg: T) -> bool {
        if (self.validator)(&msg) {
            let _ = self.tx.send(msg);
            true
        } else {
            false
        }
    }
}
