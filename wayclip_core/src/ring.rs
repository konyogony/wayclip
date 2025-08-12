use crate::{log_to, logging::Logger};
use gstreamer::ClockTime;
use std::collections::VecDeque;

const EBML_MAGIC: &[u8] = b"\x1A\x45\xDF\xA3";

type Frame = Vec<u8>;
type TimedFrame = (Frame, ClockTime);

pub struct RingBuffer {
    pub header: Vec<Frame>,
    pub header_complete: bool,
    pub buffer: VecDeque<TimedFrame>,
    pub capacity_duration: ClockTime,
    pub logger: Logger,
}

impl RingBuffer {
    pub fn new(capacity_duration: ClockTime, logger: &Logger) -> Self {
        log_to!(*logger, Info, [RING] => "RingBuffer initialized with duration {capacity_duration}");
        Self {
            header: Vec::new(),
            header_complete: false,
            buffer: VecDeque::new(),
            capacity_duration,
            logger: logger.clone(),
        }
    }

    pub fn push(&mut self, data: Vec<u8>, is_header: bool, pts: Option<ClockTime>) {
        if !self.header_complete {
            let looks_like_ebml = data.windows(4).any(|w| w == EBML_MAGIC);
            if is_header || (self.header.is_empty() && looks_like_ebml) {
                log_to!(self.logger, Debug, [RING] => "Header chunk captured (heuristic), size: {}", data.len());
                self.header.push(data);
                return;
            }
        }

        if !self.header_complete {
            log_to!(self.logger, Info, [RING] => "First frame detected (non-header). Header is now complete with {} chunks.", self.header.len());
            log_to!(self.logger, Info, [DAEMON] => "Recording successfully started, and live! Ctrl + C for graceful shutdown, ALT+C to save clip (hyprland only so far)");
            self.header_complete = true;
        }

        if let Some(timestamp) = pts {
            self.buffer.push_back((data, timestamp));

            while let (Some((_, first_pts)), Some((_, last_pts))) =
                (self.buffer.front(), self.buffer.back())
            {
                if let Some(duration) = last_pts.checked_sub(*first_pts) {
                    if duration > self.capacity_duration {
                        self.buffer.pop_front();
                    } else {
                        break;
                    }
                } else {
                    log_to!(self.logger, Warn, [RING] => "Timestamp reset detected (last < first). Clearing buffer to resync.");
                    self.buffer.clear();
                    break;
                }
            }
        }
    }

    pub fn get_and_clear(&mut self) -> Vec<Frame> {
        if self.header.is_empty() {
            log_to!(self.logger, Error, [RING] => "get_and_clear called but no header was ever captured.");
            return Vec::new();
        }

        let mut all_data = self.header.clone();
        all_data.extend(self.buffer.drain(..).map(|(frame, _)| frame));

        log_to!(self.logger, Info,
            [RING] => "get_and_clear, returning {} header chunks + {} frames",
            self.header.len(),
            all_data.len() - self.header.len()
        );
        all_data
    }
}
