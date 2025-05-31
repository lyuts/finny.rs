// disabling until no_std + alloc becomes stable
// #![no_std]

#![warn(missing_docs)]

use finny::{finny_fsm, inspect::null::InspectNull, FsmEventQueueArray, FsmFactory, FsmTimersNull};

pub fn main() {
    // Since we are passing a C string the final null character is mandatory
    const HELLO: &'static str = "Hello, world!\n\0";
    unsafe {
        libc::printf(HELLO.as_ptr() as *const _);
    }

    {
        let ctx = StateMachineContext::default();
        let queue = FsmEventQueueArray::<_, 16>::new();
        let inspect = InspectNull::new();
        let timers = FsmTimersNull;
        let mut fsm = StateMachine::new_with(ctx, queue, inspect, timers).unwrap();
        fsm.start().unwrap();
    }
}

///////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct StateMachineContext {
    count: usize,
    total_time: usize,
}

#[derive(Default)]
pub struct StateA {
    enter: usize,
    exit: usize,
}
#[derive(Default)]
pub struct StateB {
    counter: usize,
}
#[derive(Default)]
pub struct StateC;

#[derive(Clone)]
pub struct EventClick {
    time: usize,
}
#[derive(Clone)]
pub struct EventEnter {
    shift: bool,
}

#[finny_fsm]
fn build_fsm(mut fsm: FsmBuilder<StateMachine, StateMachineContext>) -> BuiltFsm {
    fsm.initial_state::<StateA>();

    fsm.state::<StateA>()
        .on_entry(|state_a, ctx| {
            ctx.context.count += 1;
            state_a.enter += 1;
        })
        .on_exit(|state_a, ctx| {
            ctx.context.count += 1;
            state_a.exit += 1;
        })
        .on_event::<EventClick>()
        .transition_to::<StateB>()
        .guard(|ev, _ctx, _| ev.time > 100)
        .action(|ev, ctx, _state_from, _state_to| {
            ctx.context.total_time += ev.time;
        });

    fsm.state::<StateB>()
        .on_entry(|state_b, _ctx| {
            state_b.counter += 1;
        })
        .on_event::<EventEnter>()
        .internal_transition()
        .guard(|ev, _ctx, _| ev.shift == false)
        .action(|_ev, _ctx, state_b| {
            state_b.counter += 1;
        });

    fsm.build()
}
