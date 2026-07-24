use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::parse_node::{ArrayTag, ParseNode, ProofBranch};

pub(crate) const MAX_INPUT_DEPTH: usize = 32;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DepthExceeded;

#[derive(Clone)]
pub(crate) struct DepthBudget {
    inner: Arc<DepthState>,
}

struct DepthState {
    current: AtomicUsize,
    limit: usize,
}

pub(crate) struct DepthGuard {
    state: Arc<DepthState>,
}

impl DepthBudget {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new(DepthState {
                current: AtomicUsize::new(0),
                limit,
            }),
        }
    }

    pub(crate) fn enter(&self) -> Result<DepthGuard, DepthExceeded> {
        loop {
            let current = self.inner.current.load(Ordering::Relaxed);
            if current >= self.inner.limit {
                return Err(DepthExceeded);
            }
            if self
                .inner
                .current
                .compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(DepthGuard {
                    state: self.inner.clone(),
                });
            }
        }
    }

    pub(crate) fn ensure_additional(&self, additional: usize) -> Result<(), DepthExceeded> {
        if self.current().saturating_add(additional) > self.inner.limit {
            Err(DepthExceeded)
        } else {
            Ok(())
        }
    }

    pub(crate) fn current(&self) -> usize {
        self.inner.current.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(crate) fn remaining(&self) -> usize {
        self.inner.limit.saturating_sub(self.current())
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        let old = self.state.current.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(old > 0);
    }
}

pub(crate) fn validate_parse_nodes_depth(
    nodes: &[ParseNode],
    limit: usize,
) -> Result<(), DepthExceeded> {
    if parse_nodes_logical_depth(nodes) > limit {
        Err(DepthExceeded)
    } else {
        Ok(())
    }
}

pub(crate) fn parse_nodes_logical_depth(nodes: &[ParseNode]) -> usize {
    #[derive(Clone, Copy)]
    struct Visit<'a> {
        node: &'a ParseNode,
        depth: usize,
        suppress_group_cost: bool,
    }

    fn push_node<'a>(
        stack: &mut Vec<Visit<'a>>,
        node: &'a ParseNode,
        depth: usize,
        suppress_group_cost: bool,
    ) {
        stack.push(Visit {
            node,
            depth,
            suppress_group_cost,
        });
    }

    fn push_nodes<'a>(stack: &mut Vec<Visit<'a>>, nodes: &'a [ParseNode], depth: usize) {
        for node in nodes.iter().rev() {
            push_node(stack, node, depth, false);
        }
    }

    let mut max_depth = 0usize;
    let mut stack = Vec::new();
    push_nodes(&mut stack, nodes, 0);

    while let Some(visit) = stack.pop() {
        max_depth = max_depth.max(visit.depth);
        match visit.node {
            ParseNode::Atom { .. }
            | ParseNode::MathOrd { .. }
            | ParseNode::TextOrd { .. }
            | ParseNode::OpToken { .. }
            | ParseNode::AccentToken { .. }
            | ParseNode::SpacingNode { .. }
            | ParseNode::ColorToken { .. }
            | ParseNode::Size { .. }
            | ParseNode::DelimSizing { .. }
            | ParseNode::LeftRightRight { .. }
            | ParseNode::Middle { .. }
            | ParseNode::Rule { .. }
            | ParseNode::Kern { .. }
            | ParseNode::Environment { .. }
            | ParseNode::Cr { .. }
            | ParseNode::Infix { .. }
            | ParseNode::Internal { .. }
            | ParseNode::Verb { .. }
            | ParseNode::Url { .. }
            | ParseNode::Raw { .. }
            | ParseNode::NoNumber { .. }
            | ParseNode::IncludeGraphics { .. } => {}

            ParseNode::OrdGroup { body, .. } => {
                let child_depth = if visit.suppress_group_cost {
                    visit.depth
                } else {
                    visit.depth + 1
                };
                max_depth = max_depth.max(child_depth);
                push_nodes(&mut stack, body, child_depth);
            }
            ParseNode::SupSub { base, sup, sub, .. } => {
                if let Some(base) = base.as_deref() {
                    push_node(&mut stack, base, visit.depth, false);
                }
                if let Some(sup) = sup.as_deref() {
                    push_node(&mut stack, sup, visit.depth + 1, true);
                }
                if let Some(sub) = sub.as_deref() {
                    push_node(&mut stack, sub, visit.depth + 1, true);
                }
            }
            ParseNode::GenFrac { numer, denom, .. } => {
                push_node(&mut stack, numer, visit.depth + 1, true);
                push_node(&mut stack, denom, visit.depth + 1, true);
            }
            ParseNode::Sqrt { body, index, .. } => {
                push_node(&mut stack, body, visit.depth + 1, true);
                if let Some(index) = index.as_deref() {
                    push_node(&mut stack, index, visit.depth + 1, true);
                }
            }
            ParseNode::Accent { base, .. } | ParseNode::AccentUnder { base, .. } => {
                push_node(&mut stack, base, visit.depth + 1, true);
            }
            ParseNode::Op { body, .. } => {
                if let Some(body) = body {
                    push_nodes(&mut stack, body, visit.depth + 1);
                }
            }
            ParseNode::OperatorName { body, .. }
            | ParseNode::Text { body, .. }
            | ParseNode::Color { body, .. }
            | ParseNode::Styling { body, .. }
            | ParseNode::Sizing { body, .. }
            | ParseNode::LeftRight { body, .. }
            | ParseNode::Phantom { body, .. }
            | ParseNode::MClass { body, .. }
            | ParseNode::Href { body, .. }
            | ParseNode::HBox { body, .. }
            | ParseNode::Pmb { body, .. }
            | ParseNode::Html { body, .. } => {
                push_nodes(&mut stack, body, visit.depth + 1);
            }
            ParseNode::Font { body, .. }
            | ParseNode::Overline { body, .. }
            | ParseNode::Underline { body, .. }
            | ParseNode::VPhantom { body, .. }
            | ParseNode::Smash { body, .. }
            | ParseNode::HorizBrace { base: body, .. }
            | ParseNode::Enclose { body, .. }
            | ParseNode::Lap { body, .. }
            | ParseNode::RaiseBox { body, .. }
            | ParseNode::VCenter { body, .. } => {
                push_node(&mut stack, body, visit.depth + 1, true);
            }
            ParseNode::Array { body, tags, .. } => {
                if let Some(tags) = tags {
                    for tag in tags.iter().rev() {
                        if let ArrayTag::Explicit(nodes) = tag {
                            push_nodes(&mut stack, nodes, visit.depth + 1);
                        }
                    }
                }
                for row in body.iter().rev() {
                    push_nodes(&mut stack, row, visit.depth + 1);
                }
            }
            ParseNode::MathChoice {
                display,
                text,
                script,
                scriptscript,
                ..
            } => {
                push_nodes(&mut stack, scriptscript, visit.depth + 1);
                push_nodes(&mut stack, script, visit.depth + 1);
                push_nodes(&mut stack, text, visit.depth + 1);
                push_nodes(&mut stack, display, visit.depth + 1);
            }
            ParseNode::XArrow { body, below, .. } => {
                push_node(&mut stack, body, visit.depth + 1, true);
                if let Some(below) = below.as_deref() {
                    push_node(&mut stack, below, visit.depth + 1, true);
                }
            }
            ParseNode::Tag { body, tag, .. } => {
                push_nodes(&mut stack, tag, visit.depth + 1);
                push_nodes(&mut stack, body, visit.depth + 1);
            }
            ParseNode::HtmlMathMl { html, mathml, .. } => {
                push_nodes(&mut stack, mathml, visit.depth + 1);
                push_nodes(&mut stack, html, visit.depth + 1);
            }
            ParseNode::CdLabel { label, .. } => {
                push_node(&mut stack, label, visit.depth + 1, true);
            }
            ParseNode::CdLabelParent { fragment, .. } => {
                push_node(&mut stack, fragment, visit.depth + 1, true);
            }
            ParseNode::CdArrow {
                label_above,
                label_below,
                ..
            } => {
                if let Some(label) = label_above.as_deref() {
                    push_node(&mut stack, label, visit.depth + 1, true);
                }
                if let Some(label) = label_below.as_deref() {
                    push_node(&mut stack, label, visit.depth + 1, true);
                }
            }
            ParseNode::ProofTree { tree, .. } => {
                max_depth = max_depth.max(visit.depth + proof_branch_logical_depth(tree));
            }
        }
    }

    max_depth
}

pub(crate) fn proof_branch_logical_depth(branch: &ProofBranch) -> usize {
    struct Frame<'a> {
        branch: &'a ProofBranch,
        next_premise: usize,
        max_premise_depth: usize,
    }

    let mut stack = vec![Frame {
        branch,
        next_premise: 0,
        max_premise_depth: 0,
    }];
    let mut completed_child_depth = None;

    while let Some(frame) = stack.last_mut() {
        if let Some(depth) = completed_child_depth.take() {
            frame.max_premise_depth = frame.max_premise_depth.max(depth);
            continue;
        }

        if frame.next_premise < frame.branch.premises.len() {
            let child = &frame.branch.premises[frame.next_premise];
            frame.next_premise += 1;
            stack.push(Frame {
                branch: child,
                next_premise: 0,
                max_premise_depth: 0,
            });
            continue;
        }

        let payload_depth = parse_nodes_logical_depth(&frame.branch.conclusion)
            .max(
                frame
                    .branch
                    .left_label
                    .as_deref()
                    .map(parse_nodes_logical_depth)
                    .unwrap_or(0),
            )
            .max(
                frame
                    .branch
                    .right_label
                    .as_deref()
                    .map(parse_nodes_logical_depth)
                    .unwrap_or(0),
            );
        let depth = 1 + payload_depth.max(frame.max_premise_depth);
        stack.pop();
        if stack.is_empty() {
            return depth;
        }
        completed_child_depth = Some(depth);
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_node::{ArrayTag, Mode};

    fn nested_ord_group(depth: usize) -> ParseNode {
        let mut node = ParseNode::MathOrd {
            mode: Mode::Math,
            text: "x".to_string(),
            loc: None,
        };
        for _ in 0..depth {
            node = ParseNode::OrdGroup {
                mode: Mode::Math,
                body: vec![node],
                semisimple: None,
                loc: None,
            };
        }
        node
    }

    #[test]
    fn logical_depth_includes_explicit_array_tags() {
        let array = ParseNode::Array {
            mode: Mode::Math,
            body: vec![],
            row_gaps: vec![],
            hlines_before_row: vec![],
            cols: None,
            col_separation_type: None,
            hskip_before_and_after: None,
            add_jot: None,
            arraystretch: 1.0,
            tags: Some(vec![ArrayTag::Explicit(vec![nested_ord_group(3)])]),
            leqno: None,
            is_cd: None,
            loc: None,
        };

        assert_eq!(parse_nodes_logical_depth(&[array]), 4);
    }
}
