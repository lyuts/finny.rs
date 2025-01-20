extern crate alloc;

use crate::lib::*;
use crate::{FsmBackend, FsmBackendImpl, FsmEvent, Inspect, InspectEvent, InspectFsmEvent};
use alloc::format;
use alloc::string::ToString;
use core::any::Any;
use core::fmt::Debug;
use tracing::{error, info};
use AsRef;

#[derive(Default)]
pub struct InspectTracing {}

impl InspectTracing {
    pub fn new() -> Self {
        InspectTracing::default()
    }
}

impl Inspect for InspectTracing {
    fn new_event<F: FsmBackend>(
        &self,
        event: &FsmEvent<<F as FsmBackend>::Events, <F as FsmBackend>::Timers>,
        fsm: &FsmBackendImpl<F>,
    ) -> Self {
        let event_display = match event {
            FsmEvent::Timer(t) => format!("Fsm::Timer({:?})", t),
            _ => event.as_ref().to_string(),
        };

        let current_state = format!("{:?}", fsm.get_current_states());

        info!(
            event = event_display,
            start_state = current_state,
            "Dispatching"
        );
        InspectTracing {}
    }

    fn for_transition<T>(&self) -> Self {
        let transition = type_name::<T>();
        info!(transition = transition, "Matched transition");
        InspectTracing {}
    }

    fn for_sub_machine<FSub: FsmBackend>(&self) -> Self {
        let sub_fsm = type_name::<FSub>();
        info!(sub_fsm = sub_fsm, "Dispatching to a submachine");
        InspectTracing {}
    }

    fn for_timer<F>(&self, timer_id: <F as FsmBackend>::Timers) -> Self
    where
        F: FsmBackend,
    {
        info!(timer_id = format!("{:?}", timer_id), "");
        InspectTracing {}
    }

    fn on_guard<T>(&self, guard_result: bool) {
        let guard = type_name::<T>();
        info!(
            "Guard {guard} evaluated to {guard_result}",
            guard = guard,
            guard_result = guard_result
        );
    }

    fn on_state_enter<S>(&self) {
        let state = type_name::<S>();
        info!("Entering {state}", state = state);
    }

    fn on_state_exit<S>(&self) {
        let state = type_name::<S>();
        info!("Exiting {state}", state = state);
    }

    fn on_action<S>(&self) {
        let action = type_name::<S>();
        info!("Executing {action}", action = action);
    }

    fn event_done<F: FsmBackend>(self, fsm: &FsmBackendImpl<F>) {
        let states = format!("{:?}", fsm.get_current_states());
        info!(stop_state = states, "Dispatch done");
    }

    fn on_error<E>(&self, msg: &str, error: &E)
    where
        E: Debug,
    {
        let error_msg = format!("{:?}", error);
        error!(error = error_msg, "{}", msg);
    }

    fn info(&self, msg: &str) {
        info!("{}", msg);
    }
}

impl InspectEvent for InspectTracing {
    fn on_event<S: Any + Debug + Clone>(&self, event: &InspectFsmEvent<S>) {
        info!("Inspection event {:?}", event);
    }
}
