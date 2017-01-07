extern crate quote;

use fsm_def::*;
use viz::*;

use quote::*;

use itertools::Itertools;

pub fn build_state_store(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_name = fsm.get_fsm_ty();
    let impl_suffix = fsm.get_impl_suffix();
    let states_ty = fsm.get_states_ty();
    let states_store_ty = fsm.get_states_store_ty();

    let mut retr = quote::Tokens::new();

    let mut f = quote::Tokens::new();
    let mut n = quote::Tokens::new();
    for state in &fsm.get_all_states() {
        let field_name = FsmDescription::to_state_field_name(&state);
        f.append(quote! { #field_name: #state,  }.as_str());
        n.append(quote! { #field_name: #state::new_state(context), }.as_str());

        retr.append(quote! {
            impl #impl_suffix FsmRetrieveState<#state> for #fsm_name {
                fn get_state(&self) -> &#state {
                    &self.states.#field_name
                }

                fn get_state_mut(&mut self) -> &mut #state {
                    &mut self.states.#field_name
                }
            }
        }.as_str());
    }

    for sub in fsm.get_submachine_types() {
        let field_name = FsmDescription::to_state_sub_started_field_name(sub);
        f.append(quote! { #field_name: bool, }.as_str());
        n.append(quote! { #field_name: false, }.as_str());
    }

    let q = quote! {
        pub struct #states_store_ty {
            #f
        }

        impl #states_store_ty {
            pub fn new<C>(context: &C) -> #states_store_ty {
                #states_store_ty {
                    #n
                }
            }
        }

        #retr
    };

    q
}



pub fn build_enums(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_name = fsm.get_fsm_ty();
    let impl_suffix = fsm.get_impl_suffix();
    let events_ty = fsm.get_events_ty();
    let actions_ty = fsm.get_actions_ty();
    let states_ty = fsm.get_states_ty();
    let history_ty = fsm.get_history_ty();

    // events
    let all_transitions = fsm.get_all_transitions();
    let events = all_transitions.iter().map(|ref x| &x.event).unique_by(|x| *x);

    let mut events_types = quote::Tokens::new();
    for event in events {
        let mut t = quote::Tokens::new();
        event.to_tokens(&mut t);
        if t.as_str() == "NoEvent" { continue; }
        
        events_types.append(quote! { #event(#event), }.as_str());
    }
    events_types.append(quote! { NoEvent(NoEvent) }.as_str());


    // states
    let mut state_types = quote::Tokens::new();

    for state in &fsm.get_all_states() {
        state_types.append(quote! { #state, }.as_str());
    }
    
    quote! {
        // Events
        #[derive(Debug)]
        pub enum #events_ty {
            #events_types
        }
        impl #impl_suffix FsmEvents<#fsm_name> for #events_ty {
            fn new_no_event() -> Self {
                #events_ty::NoEvent(NoEvent)
            }
        }

        // States
        #[derive(PartialEq, Copy, Clone, Debug)]
        pub enum #states_ty {
            #state_types
        }
    }
}

pub fn build_state_transitions(fsm: &FsmDescription) -> quote::Tokens {

    let fsm_ty = fsm.get_fsm_ty();
    let fsm_ty_inline = fsm.get_fsm_ty_inline();
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();
    let actions_ty = fsm.get_actions_ty();
    let history_ty = fsm.get_history_ty();
    let context_ty = &fsm.context_ty;

    // states

    let mut event_dispatch = quote::Tokens::new();
    let mut interrupted_states = quote::Tokens::new();

    for region in &fsm.regions {
        let mut q = quote::Tokens::new();

        for state in &region.get_all_states() {
            let t: Vec<_> = region.transitions.iter().filter(|&x| &x.source_state == state).collect();

            if t.len() == 0 { continue; }

            let mut tq = quote::Tokens::new();

            for transition in t {

                let event = &transition.event;
                let target_state = &transition.target_state;
                let action = &transition.action;


                let source_state_field = FsmDescription::to_state_field_name(&state);
                let target_state_field = FsmDescription::to_state_field_name(&target_state);

                let action_call = if transition.has_same_states() {
                    quote! {
                        <#action as FsmActionSelf<#fsm_ty, #state>>::action(&mut event_ctx, &mut self.states.#source_state_field);
                    }
                } else {
                    quote! {
                        <#action as FsmAction<#fsm_ty, #state, #target_state>>::action(&mut event_ctx, &mut self.states.#source_state_field, &mut self.states.#target_state_field);
                    }
                };

                let mut sub_init = quote! { };
                if fsm.is_submachine(&target_state) {
                    let f = FsmDescription::to_state_sub_started_field_name(&target_state);

                    let is_shallow = fsm.shallow_history_events.iter().find(|ref x| &x.event_ty == event && &x.target_state_ty == target_state).is_some();

                    if is_shallow == false {
                        sub_init = quote! {
                            {
                                self.states.#target_state_field.start();
                                self.states.#f = true;
                                just_called_start = true;
                            }
                        };
                    }
                }

                let mut sub_state_exit = quote! {};
                let mut sub_state_entry = quote! {};

                if fsm.is_submachine(&state) {
                    sub_state_exit = quote! {
                        {
                            let s = self.states.#source_state_field.get_current_state();
                            self.states.#source_state_field.call_on_exit(s);
                        }
                    };
                }

                
                if fsm.is_submachine(&target_state) {
                    sub_state_entry = quote! {
                        {
                            let s = self.states.#target_state_field.get_current_state();
                            self.states.#target_state_field.call_on_entry(s);
                        }
                    };
                }


                let mut state_exit = quote! {
                    self.inspection.on_state_exit(&current_state, &event_ctx);
                    self.states.#source_state_field.on_exit(&mut event_ctx);
                };

                let mut state_entry = quote! {
                    self.inspection.on_state_entry(&#states_ty::#target_state, &event_ctx);
                    self.states.#target_state_field.on_entry(&mut event_ctx);
                };
                
                if transition.transition_type == TransitionType::Internal {
                    state_exit = quote! {};
                    state_entry = quote! {};
                }

                let guard = if let Some(ref guard_ty) = transition.guard {
                    quote! {
                        if #guard_ty::guard(&event_ctx)
                    }
                } else {
                    quote! {}
                };

                let state_set = if fsm.has_multiple_regions() { 
                    let mut q = quote! { self.state. };
                    q.append(&region.id.to_string());
                    q
                } else {
                    quote! { self.state }
                };

                
                let s = quote! {
                    (#states_ty::#state, &#events_ty::#event(_)) #guard => {

                        #state_exit
                        #sub_state_exit
                        
                        self.inspection.on_action(&current_state, &event_ctx);
                        #action_call
                        

                        let mut just_called_start = false;
                        #sub_init

                        #state_entry
                        if just_called_start == false {
                            #sub_state_entry
                        }

                        self.inspection.on_transition(&current_state, &#states_ty::#target_state, &event_ctx);
                        #state_set = #states_ty::#target_state;
                        

                        Ok(())
                    },
                };

                tq.append(s.as_str());
            }

            q.append(tq.as_str());      
        }

        let (region_state_field, result) = if fsm.has_multiple_regions() { 
            let mut q = quote! { self.state. };
            q.append(&region.id.to_string());

            let mut r = quote::Tokens::new();
            r.append(&format!("r{}", region.id));
            (q, r)            
        } else {
            (quote! { self.state }, quote! { res })
        };

        event_dispatch.append(quote! {

            let current_state = #region_state_field;
            let #result = match (current_state, &event) {
                #q
                (_, _) => Err(FsmError::NoTransition)
            };

        }.as_str());


        for interrupted_state in &region.interrupt_states {
            let s_ty = &interrupted_state.interrupt_state_ty;

            let mut m = quote::Tokens::new();
            for e_ty in &interrupted_state.resume_event_ty {
                m.append(quote! {
                    (#states_ty::#s_ty, &#events_ty::#e_ty(_)) => {
                        whitelisted_event = true;
                    },
                }.as_str());
            }

            interrupted_states.append(quote! {
                match (#region_state_field, &event) {
                    #m
                    (#states_ty::#s_ty, _) => {
                        is_interrupted = true;
                    },
                    (_, _) => ()
                }
            }.as_str());
        }


    }
    
    let mut return_result = quote! {
        let mut res = None;
    };
    if fsm.has_multiple_regions() {                 
        let mut r = quote::Tokens::new();

        for region in &fsm.regions {
            let mut q = quote! { self.state. };
            q.append(&region.id.to_string());

            r = quote::Tokens::new();
            r.append(&format!("r{}", region.id));
            
            return_result.append(quote! {
                if #r == Err(FsmError::NoTransition) {
                    self.inspection.on_no_transition(&#q, &event_ctx);
                }
                if res.is_none() && #r.is_ok() {
                    res = Some(#r);
                }
                if res.is_none() && !#r.is_ok() && #r != Err(FsmError::NoTransition) {
                    res = Some(#r);
                }
            }.as_str());
        }

        return_result.append(quote! {            
            let res = res.unwrap_or(Err(FsmError::NoTransition));
        }.as_str());
    } else {
        return_result = quote! {
            if res == Err(FsmError::NoTransition) {
                self.inspection.on_no_transition(&self.state, &event_ctx);
            }
        }
    }

    let f = quote! {
        fn process_event(&mut self, event: #events_ty) -> Result<(), FsmError> {
            if self.execute_queue_pre {
                self.execute_queued_events();
            }

            let res = {
                let mut event_ctx = EventContext {
                    event: &event,
                    queue: &mut self.queue,
                    context: &mut self.context
                };

                {
                    let mut is_interrupted = false;
                    let mut whitelisted_event = false;
                    #interrupted_states
                    if is_interrupted && whitelisted_event == false {
                        return Err(FsmError::Interrupted);
                    }
                }

                #event_dispatch

                #return_result

                res
            };

            if self.execute_queue_post {
                self.execute_queued_events();
            }

            return res;
        }
    };


    f
}


pub fn build_main_struct(fsm: &FsmDescription) -> quote::Tokens {

    let fsm_ty = fsm.get_fsm_ty();
    let fsm_ty_inline = fsm.get_fsm_ty_inline();
    let impl_suffix = fsm.get_impl_suffix();
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();
    let current_state_ty = fsm.get_current_state_ty();
    let states_store_ty = fsm.get_states_store_ty();
    let actions_ty = fsm.get_actions_ty();
    let history_ty = fsm.get_history_ty();
    let inspection_ty = fsm.get_inspection_ty();
    let ctx = &fsm.context_ty;
    
    let transitions = build_state_transitions(fsm);

    let mut start = quote! {
        self.state = Self::new_initial_state();
        let no = Self::E::new_no_event();
    };

    for region in &fsm.regions {
        let initial_state = &region.initial_state_ty;
        let initial_state_field = FsmDescription::to_state_field_name(initial_state);

        start.append(quote! {                        
            {
                let mut event_ctx = EventContext {
                    event: &no,
                    queue: &mut self.queue,
                    context: &mut self.context
                };
                self.states.#initial_state_field.on_entry(&mut event_ctx);
            }
        }.as_str());
    }

    start.append(quote! {
        self.process_event(no);
        self.process_anonymous_transitions();
    }.as_str());

    

    let mut stop = quote! {};
    if fsm.has_multiple_regions() {
        stop.append(quote!{
            let s = self.get_current_state();
        }.as_str());
        for region in &fsm.regions {
            let mut q = Tokens::new();
            q.append(&format!("s.{}", region.id));
            stop.append(quote! {
                self.call_on_exit(#q);
            }.as_str());
        }        
    } else {        
        stop = quote! {
            {
                let s = self.get_current_state();
                self.call_on_exit(s);
            }
        };
    }
    
    let sub_on_handlers = build_on_handlers(fsm);

    let initial_state = {
        let st: Vec<_> = fsm.regions.iter().map(|x| {
            let mut t = quote! { #states_ty:: };            
            x.initial_state_ty.to_tokens(&mut t);
            t
        }).collect();

        quote! {
            ( #(#st),* )
        }
    };
    
    let viz = build_viz(&fsm);

    quote! {
        pub struct #fsm_ty {
	        state: #current_state_ty,
            states: #states_store_ty,
	        context: #ctx,
            queue: FsmEventQueueVec<#fsm_ty>,
            inspection: #inspection_ty,

            pub execute_queue_pre: bool,
            pub execute_queue_post: bool
        }

        impl #impl_suffix Fsm for #fsm_ty {
            type E = #events_ty;
            type S = #states_ty;
            type C = #ctx;
            type CS = #current_state_ty;
            
            fn new(context: Self::C) -> Self {                
                #fsm_ty_inline {
                    state: Self::new_initial_state(),
                    states: #states_store_ty::new(&context),
                    inspection: <#inspection_ty>::new_from_context(&context),
                    context: context,
                    queue: FsmEventQueueVec::new(),

                    execute_queue_pre: true,
                    execute_queue_post: false
                }
            }

            fn start(&mut self) {
                #start
            }

	        fn stop(&mut self) {
                #stop
            }

            fn get_queue(&self) -> &FsmEventQueue<Self> {
                &self.queue
            }

            fn get_queue_mut(&mut self) -> &mut FsmEventQueue<Self> {
                &mut self.queue
            }

            fn get_current_state(&self) -> #current_state_ty {
                self.state
            }

            #transitions
        }

        impl #impl_suffix #fsm_ty {            
            fn new_initial_state() -> #current_state_ty {
                #initial_state
            }
            
            pub fn get_context(&self) -> &#ctx {
                &self.context
            }

            #sub_on_handlers
            #viz
        }
    }
}

pub fn build_on_handlers(fsm: &FsmDescription) -> quote::Tokens {
    
    let fsm_ty = fsm.get_fsm_ty();
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();

    let mut on_entry = quote::Tokens::new();
    let mut on_exit = quote::Tokens::new();

    for state in &fsm.get_all_states() {

        if fsm.is_submachine(&state) { continue; }

        let f = FsmDescription::to_state_field_name(&state);

        on_entry.append(quote!{
            #states_ty::#state => {
                self.states.#f.on_entry(&mut event_ctx);
                self.inspection.on_state_entry(&state, &event_ctx);
            },
        }.as_str());

        on_exit.append(quote!{
            #states_ty::#state => {
                self.states.#f.on_exit(&mut event_ctx);
                self.inspection.on_state_exit(&state, &event_ctx);
            },
        }.as_str());
    }

    quote! {
        pub fn call_on_entry(&mut self, state: #states_ty) {
            let no = #events_ty::new_no_event();
            let mut event_ctx = EventContext {
                event: &no,
                queue: &mut self.queue,
                context: &mut self.context
            };
            match state {
                #on_entry
                _ => ()
            }
        }

        pub fn call_on_exit(&mut self, state: #states_ty) {
            let no = #events_ty::new_no_event();
            let mut event_ctx = EventContext {
                event: &no,
                queue: &mut self.queue,
                context: &mut self.context
            };
            match state {
                #on_exit
                _ => ()
            }
        }
    }
}
