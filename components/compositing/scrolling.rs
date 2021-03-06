/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A timer thread that gives the painting task a little time to catch up when the user scrolls.

use compositor_task::{CompositorProxy, Msg};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, Builder};
use std::u32;
use time;
use util::time::duration_from_nanoseconds;

/// The amount of time in nanoseconds that we give to the painting thread to paint new tiles upon
/// processing a scroll event that caused new tiles to be revealed. When this expires, we give up
/// and composite anyway (showing a "checkerboard") to avoid dropping the frame.
static TIMEOUT: u64 = 12_000_000;

pub struct ScrollingTimerProxy {
    sender: Sender<ToScrollingTimerMsg>,
}

pub struct ScrollingTimer {
    compositor_proxy: Box<CompositorProxy>,
    receiver: Receiver<ToScrollingTimerMsg>,
}

enum ToScrollingTimerMsg {
    ExitMsg,
    ScrollEventProcessedMsg(u64),
}

impl ScrollingTimerProxy {
    pub fn new(compositor_proxy: Box<CompositorProxy + Send>) -> ScrollingTimerProxy {
        let (to_scrolling_timer_sender, to_scrolling_timer_receiver) = channel();
        Builder::new().spawn(move || {
            let mut scrolling_timer = ScrollingTimer {
                compositor_proxy: compositor_proxy,
                receiver: to_scrolling_timer_receiver,
            };
            scrolling_timer.run();
        }).unwrap();
        ScrollingTimerProxy {
            sender: to_scrolling_timer_sender,
        }
    }

    pub fn scroll_event_processed(&mut self, timestamp: u64) {
        self.sender.send(ToScrollingTimerMsg::ScrollEventProcessedMsg(timestamp)).unwrap()
    }

    pub fn shutdown(&mut self) {
        self.sender.send(ToScrollingTimerMsg::ExitMsg).unwrap()
    }
}

impl ScrollingTimer {
    pub fn run(&mut self) {
        while let Ok(ToScrollingTimerMsg::ScrollEventProcessedMsg(timestamp)) = self.receiver.recv() {
            let target = timestamp + TIMEOUT;
            let delta_ns = target - time::precise_time_ns();
            thread::sleep(duration_from_nanoseconds(delta_ns));
            self.compositor_proxy.send(Msg::ScrollTimeout(timestamp));
        }
    }
}

