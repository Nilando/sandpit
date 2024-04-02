use super::allocate::{Allocate, GenerationalArena};
use super::tracer_controller::{TracePacket, TracerController};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

pub trait Tracer {}

impl<A: Allocate> Tracer for TracerWorker<A> {}

pub struct TracerWorker<A: Allocate> {
    _allocator: PhantomData<A>,
}

impl<A: Allocate> TracerWorker<A> {
    pub fn spawn(
        unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>,
        mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
    ) {
    }
}
/*

let new_packet: Option<TracePacket<A>> = None;

loop {
    let packet = unscanned.as_ref().lock().unwrap().pop();

    match packet {
        Some(packet) => {
            // scan
                // create a new empty work packet
                // as you scan fill the empty work packet
                // if it fills send it to the tracer, start filling a new one
        }
        None => {
            // scan the new_packet if it exists
                // create a new empty work packet
                // as you scan fill the empty work packet
                // if it fills send it to the tracer, start filling a new one
            // if new packet does not exist/is empty
                // we are done!
                // break
            match new_packet {
                Some(_) => {}
                None => break
            }
        }
    }
}
*/
