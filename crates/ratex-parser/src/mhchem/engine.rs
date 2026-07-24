//! `mhchemParser.go` state machine driver.

use std::cell::Cell;

use crate::mhchem::actions;
use crate::mhchem::buffer::Buffer;
use crate::mhchem::error::{MhchemError, MhchemResult};
use crate::mhchem::patterns::match_pattern;
use crate::mhchem::ParserCtx;
use crate::parser::MAX_RECURSION_DEPTH;
use serde_json::Value;

thread_local! {
    /// mhchem recursively invokes sub-machines from actions, independently of the
    /// main parser's recursion counter.
    static MHCHEM_RECURSION_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn normalize_input(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push(' '),
            '\u{2212}' | '\u{2013}' | '\u{2014}' | '\u{2010}' => out.push('-'),
            '\u{2026}' => out.push_str("..."),
            _ => out.push(c),
        }
    }
    out
}

pub(crate) fn go_machine(
    ctx: &ParserCtx<'_>,
    input: &str,
    machine: &str,
) -> MhchemResult<Vec<Value>> {
    let previous_depth = MHCHEM_RECURSION_DEPTH.with(Cell::get);
    // mhchem inputs fan out through helper machines before texify. Guard the
    // active engine-call depth directly so engine callers fail before building
    // an over-budget AST, while preserving the public parser boundary.
    if previous_depth >= MAX_RECURSION_DEPTH {
        return Err(MhchemError::msg("Recursion limit exceeded"));
    }
    MHCHEM_RECURSION_DEPTH.with(|depth| depth.set(previous_depth + 1));
    let result = go_machine_impl(ctx, input, machine);
    MHCHEM_RECURSION_DEPTH.with(|depth| depth.set(previous_depth));
    result
}

fn go_machine_impl(ctx: &ParserCtx<'_>, input: &str, machine: &str) -> MhchemResult<Vec<Value>> {
    let mut input = normalize_input(input);
    if input.is_empty() {
        return Ok(vec![]);
    }

    let Some(mdef) = ctx.data.machines.0.get(machine) else {
        return Err(MhchemError::msg(format!("unknown state machine {machine}")));
    };
    let mut state = "0".to_string();
    let mut buffer = Buffer::new();
    let mut output: Vec<Value> = vec![];
    let mut last_input_ptr = std::ptr::null();
    let mut last_input_len = 0;
    let mut watchdog = 10i32;

    loop {
        let ptr = input.as_ptr();
        let len = input.len();
        if ptr != last_input_ptr || len != last_input_len {
            watchdog = 10;
            last_input_ptr = ptr;
            last_input_len = len;
        } else {
            watchdog -= 1;
        }
        if watchdog <= 0 {
            return Err(MhchemError::msg("mhchem bug U"));
        }

        let transitions = mdef
            .transitions
            .get(&state)
            .or_else(|| mdef.transitions.get("*"))
            .ok_or_else(|| MhchemError::msg(format!("no transitions for state {state}")))?;

        let mut consumed_transition = false;
        'iter: for tr in transitions {
            let Some(hit) = match_pattern(ctx.data, &tr.pattern, &input)? else {
                continue;
            };
            consumed_transition = true;
            let empty_at_match = input.is_empty();
            for spec in &tr.task.action_ {
                let piece = actions::apply(ctx, machine, &mut buffer, &hit.token, spec)?;
                extend_json(&mut output, piece);
            }
            if let Some(ns) = &tr.task.next_state {
                state.clone_from(ns);
            }

            if empty_at_match {
                return Ok(output);
            }
            if !tr.task.revisit {
                input = hit.remainder.clone();
            }
            if !tr.task.to_continue {
                break 'iter;
            }
        }

        if !consumed_transition {
            return Err(MhchemError::msg("mhchem bug U"));
        }
    }
}

fn extend_json(out: &mut Vec<Value>, piece: Vec<Value>) {
    out.extend(piece);
}
