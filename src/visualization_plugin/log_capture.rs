use bevy::prelude::Resource;
use crossbeam_channel::{Receiver, Sender};
use std::io;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter};

/// A resource to hold the receiving end of the log channel.
#[derive(Resource)]
pub struct LogReceiver(pub Receiver<String>);

/// A `MakeWriter` that creates a writer for sending log lines to a channel.
#[derive(Clone)]
struct UiLogMakeWriter {
    sender: Sender<String>,
}

impl<'a> MakeWriter<'a> for UiLogMakeWriter {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ChannelWriter {
            sender: self.sender.clone(),
        }
    }
}

/// An `io::Write` implementation that sends each line written to it over a channel.
struct ChannelWriter {
    sender: Sender<String>,
}

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // The buffer from tracing-subscriber's formatter usually includes a newline.
        // We trim it before sending to avoid double newlines in the UI.
        let msg = String::from_utf8_lossy(buf).trim_end().to_string();
        if !msg.is_empty() {
            // We don't care if the receiver has been dropped.
            let _ = self.sender.send(msg);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Sets up the global logger to send to both the console and our UI channel.
/// Must be called before the Bevy app is initialized.
pub fn setup_logging() -> Receiver<String> {
    use tracing_subscriber::prelude::*;

    let (tx, rx) = crossbeam_channel::unbounded();

    // This layer will print to the console.
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true);

    // This layer will capture logs for the UI
    let ui_log_make_writer = UiLogMakeWriter { sender: tx };
    let ui_layer = tracing_subscriber::fmt::layer()
        .with_writer(ui_log_make_writer)
        .with_ansi(false);

    // This filter will respect the RUST_LOG environment variable.
    // If RUST_LOG is not set, it defaults to showing `info` level logs for our crate,
    // and `warn` for other noisy crates.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(ui_layer)
        .init();

    rx
}
