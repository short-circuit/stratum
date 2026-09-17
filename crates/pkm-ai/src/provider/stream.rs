//! Streaming buffer helper for chunked HTTP responses.

/// A buffer for accumulating streaming data and extracting complete,
/// delimiter-separated messages. Handles partial data split across
/// TCP chunks, preventing silent data loss.
pub(crate) struct StreamBuffer {
    buffer: String,
}

impl StreamBuffer {
    pub(crate) fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed a chunk of streaming data and extract all complete messages
    /// delimited by `delimiter`. Any incomplete trailing data is retained
    /// in the internal buffer for the next call to `feed`.
    pub(crate) fn feed(&mut self, chunk: &str, delimiter: &str) -> Vec<String> {
        self.buffer.push_str(chunk);

        if self.buffer.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let delim_len = delimiter.len();

        while let Some(pos) = self.buffer.find(delimiter) {
            let msg = self.buffer[..pos].to_string();
            if !msg.trim().is_empty() {
                results.push(msg);
            }
            self.buffer = self.buffer[pos + delim_len..].to_string();
        }

        results
    }
}
