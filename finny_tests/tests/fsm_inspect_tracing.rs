use finny::inspect::tracing::InspectTracing;
use finny::{finny_fsm, timers::std::TimersStd, FsmEventQueueVec, FsmFactory, FsmResult};
use tracing::{info, Level};

#[derive(Default)]
pub struct Context {
    temperature: u32,
}

#[derive(Default)]
pub struct Normal;

#[derive(Default)]
pub struct EvaluatingTemperature;

#[derive(Default)]
pub struct TooHot;

#[derive(Clone)]
pub struct NewTempReading(u32);
#[derive(Clone)]
pub struct HighTemperature;

#[derive(Clone)]
pub struct Noop;

#[finny_fsm]
fn my_fsm(mut fsm: FsmBuilder<MyFsm, Context>) -> BuiltFsm {
    fsm.state::<Normal>()
        .on_entry(|_state, _ctx| {
            info!("Normal");
        })
        .on_event::<NewTempReading>()
        .transition_to::<EvaluatingTemperature>()
        .action(|ev, ctx, _state_a, _state_b| {
            ctx.context.temperature = ev.0;
        });
    fsm.state::<EvaluatingTemperature>()
        .on_entry(|_state, ctx| {
            info!("EvaluatingTemperature");
            if ctx.temperature > 50 {
                ctx.queue.enqueue(HighTemperature {}).unwrap();
            } else {
                ctx.queue.enqueue(Noop {}).unwrap();
            }
        })
        .on_event::<HighTemperature>()
        .transition_to::<TooHot>();
    fsm.state::<EvaluatingTemperature>()
        .on_event::<Noop>()
        .transition_to::<Normal>();
    fsm.state::<TooHot>().on_entry(|_state, _ctx| {
        info!("TooHot");
    });
    fsm.initial_state::<Normal>();
    fsm.build()
}

#[test]
fn test_inspect_tracing() -> FsmResult<()> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .with_max_level(Level::TRACE)
        .with_target(false)
        .with_thread_names(true)
        .init();

    let mut fsm = MyFsm::new_with(
        Context::default(),
        FsmEventQueueVec::new(),
        InspectTracing::new(),
        TimersStd::new(),
    )?;
    fsm.start()?;
    fsm.dispatch(NewTempReading(100))?;
    Ok(())
}
