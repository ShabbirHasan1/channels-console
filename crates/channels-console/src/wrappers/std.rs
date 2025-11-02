use std::mem;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};

use crate::{init_stats_state, ChannelType, StatsEvent};

/// Wrap a bounded std channel with proxy ends. Returns (outer_tx, outer_rx).
/// All messages pass through the two forwarders running in separate threads.
pub(crate) fn wrap_sync_channel<T: Send + 'static>(
    inner: (SyncSender<T>, Receiver<T>),
    channel_id: &'static str,
    label: Option<&'static str>,
    capacity: usize,
) -> (SyncSender<T>, Receiver<T>) {
    let (inner_tx, inner_rx) = inner;
    let type_name = std::any::type_name::<T>();

    let (outer_tx, to_inner_rx) = mpsc::sync_channel::<T>(capacity);
    let (from_inner_tx, outer_rx) = mpsc::sync_channel::<T>(capacity);

    let (stats_tx, _) = init_stats_state();

    let _ = stats_tx.send(StatsEvent::Created {
        id: channel_id,
        display_label: label,
        channel_type: ChannelType::Bounded(capacity),
        type_name,
        type_size: mem::size_of::<T>(),
    });

    let stats_tx_send = stats_tx.clone();
    let stats_tx_recv = stats_tx.clone();

    // Create a signal channel to notify send-forwarder when outer_rx is closed
    let (close_signal_tx, close_signal_rx) = mpsc::channel::<()>();

    // Forward outer -> inner (proxy the send path)
    std::thread::spawn(move || {
        loop {
            // Check for close signal (non-blocking)
            match close_signal_rx.try_recv() {
                Ok(_) => {
                    // Outer receiver was closed/dropped
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Close signal sender dropped, which means recv forwarder ended
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No close signal, continue
                }
            }

            // Try to receive with timeout to periodically check close signal
            match to_inner_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(msg) => {
                    if inner_tx.send(msg).is_err() {
                        // Inner receiver dropped
                        break;
                    }
                    let _ = stats_tx_send.send(StatsEvent::MessageSent { id: channel_id });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // No message, loop again to check close signal
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Outer sender dropped
                    break;
                }
            }
        }
        // Channel is closed
        let _ = stats_tx_send.send(StatsEvent::Closed { id: channel_id });
    });

    // Forward inner -> outer (proxy the recv path)
    std::thread::spawn(move || {
        while let Ok(msg) = inner_rx.recv() {
            if from_inner_tx.send(msg).is_err() {
                // Outer receiver was closed
                let _ = close_signal_tx.send(());
                break;
            }
            let _ = stats_tx_recv.send(StatsEvent::MessageReceived { id: channel_id });
        }
        // Channel is closed (either inner sender dropped or outer receiver closed)
        let _ = stats_tx_recv.send(StatsEvent::Closed { id: channel_id });
    });

    (outer_tx, outer_rx)
}

/// Wrap an unbounded std channel with proxy ends. Returns (outer_tx, outer_rx).
pub(crate) fn wrap_channel<T: Send + 'static>(
    inner: (Sender<T>, Receiver<T>),
    channel_id: &'static str,
    label: Option<&'static str>,
) -> (Sender<T>, Receiver<T>) {
    let (inner_tx, inner_rx) = inner;
    let type_name = std::any::type_name::<T>();

    let (outer_tx, to_inner_rx) = mpsc::channel::<T>();
    let (from_inner_tx, outer_rx) = mpsc::channel::<T>();

    let (stats_tx, _) = init_stats_state();

    let _ = stats_tx.send(StatsEvent::Created {
        id: channel_id,
        display_label: label,
        channel_type: ChannelType::Unbounded,
        type_name,
        type_size: mem::size_of::<T>(),
    });

    let stats_tx_send = stats_tx.clone();
    let stats_tx_recv = stats_tx.clone();

    // Create a signal channel to notify send-forwarder when outer_rx is closed
    let (close_signal_tx, close_signal_rx) = mpsc::channel::<()>();

    // Forward outer -> inner (proxy the send path)
    std::thread::spawn(move || {
        loop {
            // Check for close signal (non-blocking)
            match close_signal_rx.try_recv() {
                Ok(_) => {
                    // Outer receiver was closed/dropped
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Close signal sender dropped, which means recv forwarder ended
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No close signal, continue
                }
            }

            // Try to receive with timeout to periodically check close signal
            match to_inner_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(msg) => {
                    if inner_tx.send(msg).is_err() {
                        // Inner receiver dropped
                        break;
                    }
                    let _ = stats_tx_send.send(StatsEvent::MessageSent { id: channel_id });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // No message, loop again to check close signal
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Outer sender dropped
                    break;
                }
            }
        }
        // Channel is closed
        let _ = stats_tx_send.send(StatsEvent::Closed { id: channel_id });
    });

    // Forward inner -> outer (proxy the recv path)
    std::thread::spawn(move || {
        while let Ok(msg) = inner_rx.recv() {
            if from_inner_tx.send(msg).is_err() {
                // Outer receiver was closed
                let _ = close_signal_tx.send(());
                break;
            }
            let _ = stats_tx_recv.send(StatsEvent::MessageReceived { id: channel_id });
        }
        // Channel is closed (either inner sender dropped or outer receiver closed)
        let _ = stats_tx_recv.send(StatsEvent::Closed { id: channel_id });
    });

    (outer_tx, outer_rx)
}

use crate::Instrument;

impl<T: Send + 'static> Instrument for (std::sync::mpsc::Sender<T>, std::sync::mpsc::Receiver<T>) {
    type Output = (std::sync::mpsc::Sender<T>, std::sync::mpsc::Receiver<T>);
    fn instrument(
        self,
        channel_id: &'static str,
        label: Option<&'static str>,
        _capacity: Option<usize>,
    ) -> Self::Output {
        wrap_channel(self, channel_id, label)
    }
}

impl<T: Send + 'static> Instrument
    for (std::sync::mpsc::SyncSender<T>, std::sync::mpsc::Receiver<T>)
{
    type Output = (std::sync::mpsc::SyncSender<T>, std::sync::mpsc::Receiver<T>);
    fn instrument(
        self,
        channel_id: &'static str,
        label: Option<&'static str>,
        capacity: Option<usize>,
    ) -> Self::Output {
        if capacity.is_none() {
            panic!("Capacity is required for bounded std channels, because they don't expose their capacity in a public API");
        }
        wrap_sync_channel(self, channel_id, label, capacity.unwrap())
    }
}
