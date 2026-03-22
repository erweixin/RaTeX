//! State machine actions (global + per-machine), mirroring KaTeX `mhchem.js`.

use crate::mhchem::buffer::Buffer;
use crate::mhchem::engine;
use crate::mhchem::error::{MhchemError, MhchemResult};
use crate::mhchem::json::ActionSpec;
use crate::mhchem::patterns::{match_pattern, MatchToken};
use crate::mhchem::ParserCtx;
use regex::Regex;
use serde_json::{json, Value};

pub fn apply(
    ctx: &ParserCtx,
    machine: &str,
    buffer: &mut Buffer,
    m: &MatchToken,
    spec: &ActionSpec,
) -> MhchemResult<Vec<Value>> {
    let t = spec.type_.as_str();
    match (machine, t) {
        ("ce", "output") => ce_output(ctx, buffer, spec.option.as_ref()),
        ("ce", "o after d") => ce_o_after_d(ctx, buffer, m),
        ("ce", "d= kv") => {
            buffer.d = Some(token_string(m));
            buffer.d_type = Some("kv".into());
            Ok(vec![])
        }
        ("ce", "charge or bond") => ce_charge_or_bond(ctx, buffer, m),
        ("ce", "- after o/d") => {
            let after_d = spec
                .option
                .as_ref()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            ce_after_od(ctx, buffer, m, after_d)
        }
        ("ce", "a to o") => {
            buffer.o = buffer.a.take();
            Ok(vec![])
        }
        ("ce", "sb=true") => {
            buffer.sb = true;
            Ok(vec![])
        }
        ("ce", "sb=false") => {
            buffer.sb = false;
            Ok(vec![])
        }
        ("ce", "beginsWithBond=true") => {
            buffer.begins_with_bond = true;
            Ok(vec![])
        }
        ("ce", "beginsWithBond=false") => {
            buffer.begins_with_bond = false;
            Ok(vec![])
        }
        ("ce", "parenthesisLevel++") => {
            buffer.parenthesis_level += 1;
            Ok(vec![])
        }
        ("ce", "parenthesisLevel--") => {
            buffer.parenthesis_level -= 1;
            Ok(vec![])
        }
        ("ce", "state of aggregation") => Ok(vec![json!({
            "type_": "state of aggregation",
            "p1": engine::go_machine(ctx, &token_string(m), "o")?,
        })]),
        ("ce", "comma") => ce_comma(buffer, m),
        ("ce", "oxidation-output") => {
            let mut v = vec![Value::String("{".into())];
            v.extend(engine::go_machine(ctx, &token_string(m), "oxidation")?);
            v.push(Value::String("}".into()));
            Ok(v)
        }
        ("ce", "frac-output") => {
            let MatchToken::A(parts) = m else {
                return Err(MhchemError::msg("frac-output"));
            };
            if parts.len() < 2 {
                return Err(MhchemError::msg("frac-output len"));
            }
            Ok(vec![json!({
                "type_": "frac-ce",
                "p1": engine::go_machine(ctx, &parts[0], "ce")?,
                "p2": engine::go_machine(ctx, &parts[1], "ce")?,
            })])
        }
        ("ce", "overset-output") | ("ce", "underset-output") | ("ce", "underbrace-output") => {
            let MatchToken::A(parts) = m else {
                return Err(MhchemError::msg("two-arg output"));
            };
            if parts.len() < 2 {
                return Err(MhchemError::msg("two-arg len"));
            }
            let ty = match spec.type_.as_str() {
                "overset-output" => "overset",
                "underset-output" => "underset",
                _ => "underbrace",
            };
            Ok(vec![json!({
                "type_": ty,
                "p1": engine::go_machine(ctx, &parts[0], "ce")?,
                "p2": engine::go_machine(ctx, &parts[1], "ce")?,
            })])
        }
        ("ce", "color-output") => {
            let MatchToken::A(parts) = m else {
                return Err(MhchemError::msg("color-output"));
            };
            if parts.len() < 2 {
                return Err(MhchemError::msg("color-output len"));
            }
            Ok(vec![json!({
                "type_": "color",
                "color1": parts[0],
                "color2": engine::go_machine(ctx, &parts[1], "ce")?,
            })])
        }
        ("ce", "operator") => {
            let kind = spec
                .option
                .as_ref()
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| match_str(m))
                .unwrap_or_default();
            Ok(vec![json!({ "type_": "operator", "kind_": kind })])
        }

        ("text", "output") => {
            if let Some(tx) = buffer.text_.take() {
                buffer.clear_all();
                Ok(vec![json!({"type_": "text", "p1": tx})])
            } else {
                Ok(vec![])
            }
        }

        ("pq", "state of aggregation") => Ok(vec![json!({
            "type_": "state of aggregation subscript",
            "p1": engine::go_machine(ctx, &token_string(m), "o")?,
        })]),
        ("pq", "color-output") | ("bd", "color-output") => {
            let MatchToken::A(parts) = m else {
                return Err(MhchemError::msg("color bd/pq"));
            };
            let sm = if machine == "pq" { "pq" } else { "bd" };
            Ok(vec![json!({
                "type_": "color",
                "color1": &parts[0],
                "color2": engine::go_machine(ctx, &parts[1], sm)?,
            })])
        }

        ("oxidation", "roman-numeral") => Ok(vec![json!({
            "type_": "roman numeral",
            "p1": match_str(m).unwrap_or_default(),
        })]),

        ("tex-math", "output") | ("tex-math tight", "output") => {
            if let Some(o) = buffer.o.take() {
                buffer.clear_all();
                Ok(vec![json!({"type_": "tex-math", "p1": o})])
            } else {
                Ok(vec![])
            }
        }
        ("tex-math tight", "tight operator") => {
            let op = match_str(m).unwrap_or_default();
            let cur = buffer.o.get_or_insert_with(String::new);
            *cur = format!("{cur}{{{op}}}");
            Ok(vec![])
        }

        ("9,9", "comma") => Ok(vec![json!({"type_": "commaDecimal"})]),

        ("pu", "enumber") => pu_enumber(ctx, m),
        ("pu", "number^") => pu_number_pow(ctx, m),
        ("pu", "space") => Ok(vec![json!({"type_": "pu-space-1"})]),
        ("pu", "output") => pu_output(ctx, buffer),

        ("pu-2", "cdot") => Ok(vec![json!({"type_": "tight cdot"})]),
        ("pu-2", "^(-1)") => {
            let pow = match_str(m).unwrap_or_default();
            let cur = buffer.rm.get_or_insert_with(String::new);
            *cur = format!("{cur}^{{{pow}}}");
            Ok(vec![])
        }
        ("pu-2", "space") => Ok(vec![json!({"type_": "pu-space-2"})]),
        ("pu-2", "output") => pu2_output(ctx, buffer),

        ("pu-9,9", "comma") => Ok(vec![json!({"type_": "commaDecimal"})]),
        ("pu-9,9", "output-0") => pu99_out(buffer, false),
        ("pu-9,9", "output-o") => pu99_out(buffer, true),

        _ => global_action(ctx, machine, buffer, m, spec),
    }
}

fn match_str(m: &MatchToken) -> Option<String> {
    match m {
        MatchToken::S(s) => Some(s.clone()),
        MatchToken::A(v) if !v.is_empty() => Some(v[0].clone()),
        _ => None,
    }
}

fn token_string(m: &MatchToken) -> String {
    match m {
        MatchToken::S(s) => s.clone(),
        MatchToken::A(v) => v.join(""),
    }
}

fn ce_comma(buffer: &mut Buffer, m: &MatchToken) -> MhchemResult<Vec<Value>> {
    let raw = token_string(m);
    let trimmed = raw.trim_end().to_string();
    let with_space = trimmed != raw;
    let ty = if with_space && buffer.parenthesis_level == 0 {
        "comma enumeration L"
    } else {
        "comma enumeration M"
    };
    Ok(vec![json!({"type_": ty, "p1": trimmed})])
}

fn ce_o_after_d(ctx: &ParserCtx, buffer: &mut Buffer, m: &MatchToken) -> MhchemResult<Vec<Value>> {
    let mut ret = vec![];
    let digits_only = buffer
        .d
        .as_deref()
        .map(|d| Regex::new(r"^[0-9]+$").unwrap().is_match(d))
        .unwrap_or(false);
    if digits_only {
        let tmp = buffer.d.take().unwrap();
        buffer.d = None;
        ret.extend(ce_output(ctx, buffer, None)?);
        buffer.b = Some(tmp);
    } else {
        ret.extend(ce_output(ctx, buffer, None)?);
    }
    cat(&mut buffer.o, m);
    Ok(ret)
}

fn ce_charge_or_bond(ctx: &ParserCtx, buffer: &mut Buffer, m: &MatchToken) -> MhchemResult<Vec<Value>> {
    if buffer.begins_with_bond {
        let mut ret = ce_output(ctx, buffer, None)?;
        ret.push(json!({"type_": "bond", "kind_": "-"}));
        Ok(ret)
    } else {
        buffer.d = Some(token_string(m));
        Ok(vec![])
    }
}

fn ce_after_od(
    ctx: &ParserCtx,
    buffer: &mut Buffer,
    m: &MatchToken,
    is_after_d: bool,
) -> MhchemResult<Vec<Value>> {
    let dash = token_string(m);
    let o = buffer.o.as_deref().unwrap_or("");
    let c1 = match_pattern(ctx.data, "orbital", o).ok().flatten();
    let c2 = match_pattern(ctx.data, "one lowercase greek letter $", o).ok().flatten();
    let c3 = match_pattern(ctx.data, "one lowercase latin letter $", o).ok().flatten();
    let c4 = match_pattern(ctx.data, "$one lowercase latin letter$ $", o).ok().flatten();
    let h_orb = c1.as_ref().map(|h| h.remainder.is_empty()).unwrap_or(false);
    let hyphen_follows =
        dash == "-" && (h_orb || c2.is_some() || c3.is_some() || c4.is_some());

    if hyphen_follows
        && buffer.a.is_none()
        && buffer.b.is_none()
        && buffer.p.is_none()
        && buffer.d.is_none()
        && buffer.q.is_none()
        && c1.is_none()
        && c3.is_some()
    {
        let oo = buffer.o.take().unwrap_or_default();
        buffer.o = Some(format!("${oo}$"));
    }

    let mut ret = vec![];
    if hyphen_follows {
        ret.extend(ce_output(ctx, buffer, None)?);
        ret.push(json!({"type_": "hyphen"}));
        return Ok(ret);
    }

    let digits_d =
        match_pattern(ctx.data, "digits", buffer.d.as_deref().unwrap_or("")).ok().flatten();
    let d_only = digits_d.map(|h| h.remainder.is_empty()).unwrap_or(false);
    if is_after_d && d_only {
        cat(&mut buffer.d, m);
        ret.extend(ce_output(ctx, buffer, None)?);
    } else {
        ret.extend(ce_output(ctx, buffer, None)?);
        ret.push(json!({"type_": "bond", "kind_": "-"}));
    }
    Ok(ret)
}

fn ce_output(ctx: &ParserCtx, buffer: &mut Buffer, entity: Option<&Value>) -> MhchemResult<Vec<Value>> {
    let entity_follows = entity.and_then(|v| {
        if let Some(n) = v.as_u64() {
            Some(n as i32)
        } else {
            v.as_i64().map(|x| x as i32)
        }
    });

    if buffer.r.is_none() {
        let empty_piece = buffer.a.is_none()
            && buffer.b.is_none()
            && buffer.p.is_none()
            && buffer.o.is_none()
            && buffer.q.is_none()
            && buffer.d.is_none()
            && entity_follows.is_none();
        if empty_piece {
            buffer.clear_soft();
            return Ok(vec![]);
        }

        let mut ret: Vec<Value> = vec![];
        if buffer.sb {
            ret.push(json!({"type_": "entitySkip"}));
        }

        let mut d_type = buffer.d_type.clone();
        if buffer.o.is_some()
            && d_type.as_deref() == Some("kv")
            && match_pattern(ctx.data, "d-oxidation$", buffer.d.as_deref().unwrap_or(""))
                .ok()
                .flatten()
                .is_some()
        {
            d_type = Some("oxidation".into());
        } else if buffer.o.is_some() && d_type.as_deref() == Some("kv") && buffer.q.is_none() {
            d_type = None;
        }

        if buffer.o.is_none() && buffer.q.is_none() && buffer.d.is_none() && buffer.b.is_none() && buffer.p.is_none()
            && entity_follows != Some(2)
        {
            buffer.o = buffer.a.take();
        } else if buffer.o.is_none() && buffer.q.is_none() && buffer.d.is_none() && (buffer.b.is_some() || buffer.p.is_some())
        {
            buffer.o = buffer.a.take();
            buffer.d = buffer.b.take();
            buffer.q = buffer.p.take();
        }

        let a = engine::go_machine(ctx, buffer.a.take().unwrap_or_default().as_str(), "a")?;
        let b = engine::go_machine(ctx, buffer.b.take().unwrap_or_default().as_str(), "bd")?;
        let p = engine::go_machine(ctx, buffer.p.take().unwrap_or_default().as_str(), "pq")?;
        let o = engine::go_machine(ctx, buffer.o.take().unwrap_or_default().as_str(), "o")?;
        let q = engine::go_machine(ctx, buffer.q.take().unwrap_or_default().as_str(), "pq")?;
        let d_sm = if d_type.as_deref() == Some("oxidation") { "oxidation" } else { "bd" };
        let d = engine::go_machine(ctx, buffer.d.take().unwrap_or_default().as_str(), d_sm)?;

        let mut chem = serde_json::Map::new();
        chem.insert("type_".into(), json!("chemfive"));
        chem.insert("a".into(), Value::Array(a));
        chem.insert("b".into(), Value::Array(b));
        chem.insert("p".into(), Value::Array(p));
        chem.insert("o".into(), Value::Array(o));
        chem.insert("q".into(), Value::Array(q));
        chem.insert("d".into(), Value::Array(d));
        if let Some(dt) = d_type {
            chem.insert("dType".into(), json!(dt));
        }
        ret.push(Value::Object(chem));
        buffer.clear_soft();
        Ok(ret)
    } else {
        let r = buffer.r.take().unwrap_or_default();
        let rdt = buffer.rdt.as_deref();
        let rd = buffer.rd.take().unwrap_or_default();
        let rd_ast = if rdt == Some("M") {
            engine::go_machine(ctx, &rd, "tex-math")?
        } else if rdt == Some("T") {
            vec![json!({"type_": "text", "p1": rd})]
        } else {
            engine::go_machine(ctx, &rd, "ce")?
        };

        let rqt = buffer.rqt.as_deref();
        let rq = buffer.rq.take().unwrap_or_default();
        let rq_ast = if rqt == Some("M") {
            engine::go_machine(ctx, &rq, "tex-math")?
        } else if rqt == Some("T") {
            vec![json!({"type_": "text", "p1": rq})]
        } else {
            engine::go_machine(ctx, &rq, "ce")?
        };

        let node = json!({
            "type_": "arrow",
            "r": r,
            "rd": rd_ast,
            "rq": rq_ast,
        });
        buffer.clear_soft();
        Ok(vec![node])
    }
}

fn pu_enumber(ctx: &ParserCtx, m: &MatchToken) -> MhchemResult<Vec<Value>> {
    let MatchToken::A(parts) = m else {
        return Ok(vec![]);
    };
    let mut ret: Vec<Value> = vec![];
    let g0 = parts.first().map(String::as_str).unwrap_or("");
    if g0 == "+-" || g0 == "+/-" {
        ret.push(Value::String("\\pm ".into()));
    } else if !g0.is_empty() {
        ret.push(Value::String(g0.to_string()));
    }
    if parts.get(1).map(|s| !s.is_empty()).unwrap_or(false) {
        ret.extend(engine::go_machine(ctx, &parts[1], "pu-9,9")?);
        if let Some(p2) = parts.get(2) {
            if !p2.is_empty() {
                if p2.contains(',') || p2.contains('.') {
                    ret.extend(engine::go_machine(ctx, p2, "pu-9,9")?);
                } else {
                    ret.push(Value::String(p2.clone()));
                }
            }
        }
        // Regex group 5 = `*` / `×` branch; group 4 = `e`/`E` or `\s*...\s*10\^`. When `e` matches,
        // group 5 is present as empty string — `Some("")` so `.or_else(|| g4)` must not be used;
        // pick non-empty branch like KaTeX `m[5] || m[4]`.
        let g_star = parts.get(4).cloned().unwrap_or_default();
        let g_e_or_times = parts.get(3).cloned().unwrap_or_default();
        let mult = {
            let a = g_star.trim();
            let b = g_e_or_times.trim();
            if !a.is_empty() {
                a.to_string()
            } else {
                b.to_string()
            }
        };
        // KaTeX mhchem: lowercase `e` → \\cdot, uppercase `E` → \\times (see golden 0100/0101 vs 0102/0103).
        if !mult.is_empty() {
            if mult == "e" || mult.starts_with('*') {
                ret.push(json!({"type_": "cdot"}));
            } else {
                ret.push(json!({"type_": "times"}));
            }
        }
    }
    if let Some(exp) = parts.get(5).filter(|s| !s.is_empty()) {
        ret.push(Value::String(format!("10^{{{exp}}}")));
    }
    Ok(ret)
}

fn pu_number_pow(ctx: &ParserCtx, m: &MatchToken) -> MhchemResult<Vec<Value>> {
    let MatchToken::A(parts) = m else {
        return Ok(vec![]);
    };
    let mut ret: Vec<Value> = vec![];
    let g0 = parts.first().map(String::as_str).unwrap_or("");
    if g0 == "+-" || g0 == "+/-" {
        ret.push(Value::String("\\pm ".into()));
    } else if !g0.is_empty() {
        ret.push(Value::String(g0.to_string()));
    }
    if let Some(base) = parts.get(1) {
        ret.extend(engine::go_machine(ctx, base, "pu-9,9")?);
    }
    if let Some(exp) = parts.get(2) {
        ret.push(Value::String(format!("^{{{exp}}}")));
    }
    Ok(ret)
}

fn pu_output(ctx: &ParserCtx, buffer: &mut Buffer) -> MhchemResult<Vec<Value>> {
    let mut d = buffer.d.clone().unwrap_or_default();
    if let Some(md) = match_pattern(ctx.data, "{(...)}", &d).ok().flatten() {
        if md.remainder.is_empty() {
            if let MatchToken::S(inner) = md.token {
                d = inner;
            }
        }
    }
    let mut qv = buffer.q.clone().unwrap_or_default();
    if let Some(mq) = match_pattern(ctx.data, "{(...)}", &qv).ok().flatten() {
        if mq.remainder.is_empty() {
            if let MatchToken::S(inner) = mq.token {
                qv = inner;
            }
        }
    }
    let re_c = Regex::new(r"\u{00B0}C|\^oC|\^\{o\}C").unwrap();
    d = re_c.replace_all(&d, "{}^{\\circ}C").to_string();
    qv = re_c.replace_all(&qv, "{}^{\\circ}C").to_string();
    let re_f = Regex::new(r"\u{00B0}F|\^oF|\^\{o\}F").unwrap();
    let d = re_f.replace_all(&d, "{}^{\\circ}F").to_string();
    let qv = re_f.replace_all(&qv, "{}^{\\circ}F").to_string();

    let res = if !qv.is_empty() {
        let b5d = engine::go_machine(ctx, &d, "pu")?;
        let b5q = engine::go_machine(ctx, &qv, "pu")?;
        if buffer.o.as_deref() == Some("//") {
            vec![json!({"type_": "pu-frac", "p1": b5d, "p2": b5q})]
        } else {
            let blen = b5d.len();
            let mut v = b5d;
            if blen > 1 || b5q.len() > 1 {
                v.push(json!({"type_": " / "}));
            } else {
                v.push(json!({"type_": "/"}));
            }
            v.extend(b5q);
            v
        }
    } else {
        engine::go_machine(ctx, &d, "pu-2")?
    };
    buffer.clear_all();
    Ok(res)
}

fn pu2_output(ctx: &ParserCtx, buffer: &mut Buffer) -> MhchemResult<Vec<Value>> {
    let res = if let Some(rm) = buffer.rm.take() {
        if let Some(mrm) = match_pattern(ctx.data, "{(...)}", &rm).ok().flatten() {
            if mrm.remainder.is_empty() {
                if let MatchToken::S(inner) = mrm.token {
                    engine::go_machine(ctx, &inner, "pu")?
                } else {
                    vec![]
                }
            } else {
                vec![json!({"type_": "rm", "p1": rm})]
            }
        } else {
            vec![json!({"type_": "rm", "p1": rm})]
        }
    } else {
        vec![]
    };
    buffer.clear_all();
    Ok(res)
}

fn pu99_out(buffer: &mut Buffer, reverse_triplets: bool) -> MhchemResult<Vec<Value>> {
    let t = buffer.text_.take().unwrap_or_default();
    let mut ret: Vec<Value> = vec![];
    if t.len() > 4 {
        if reverse_triplets {
            let mut a = t.len() % 3;
            if a == 0 {
                a = 3;
            }
            let mut i = t.len() as i32 - 3;
            while i > 0 {
                ret.push(Value::String(t[i as usize..i as usize + 3].to_string()));
                ret.push(json!({"type_": "1000 separator"}));
                i -= 3;
            }
            ret.push(Value::String(t[..a].to_string()));
            ret.reverse();
        } else {
            let a = t.len() - 3;
            let mut i = 0;
            while i < a {
                ret.push(Value::String(t[i..i + 3].to_string()));
                ret.push(json!({"type_": "1000 separator"}));
                i += 3;
            }
            ret.push(Value::String(t[i..].to_string()));
        }
    } else {
        ret.push(Value::String(t));
    }
    buffer.clear_all();
    Ok(ret)
}

fn global_action(
    ctx: &ParserCtx,
    machine: &str,
    buffer: &mut Buffer,
    m: &MatchToken,
    spec: &ActionSpec,
) -> MhchemResult<Vec<Value>> {
    match spec.type_.as_str() {
        "a=" => {
            cat(&mut buffer.a, m);
            Ok(vec![])
        }
        "b=" => {
            cat(&mut buffer.b, m);
            Ok(vec![])
        }
        "p=" => {
            cat(&mut buffer.p, m);
            Ok(vec![])
        }
        "o=" => {
            cat(&mut buffer.o, m);
            Ok(vec![])
        }
        "q=" => {
            cat(&mut buffer.q, m);
            Ok(vec![])
        }
        "d=" => {
            cat(&mut buffer.d, m);
            Ok(vec![])
        }
        "rm=" => {
            cat(&mut buffer.rm, m);
            Ok(vec![])
        }
        "text=" => {
            cat(&mut buffer.text_, m);
            Ok(vec![])
        }
        "r=" => {
            buffer.r = Some(token_string(m));
            Ok(vec![])
        }
        "rdt=" | "rqt=" => {
            if spec.type_ == "rdt=" {
                buffer.rdt = Some(token_string(m));
            } else {
                buffer.rqt = Some(token_string(m));
            }
            Ok(vec![])
        }
        "rd=" | "rq=" => {
            if spec.type_ == "rd=" {
                buffer.rd = Some(token_string(m));
            } else {
                buffer.rq = Some(token_string(m));
            }
            Ok(vec![])
        }
        "insert" => {
            let opt = spec
                .option
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(vec![json!({"type_": opt})])
        }
        "insert+p1" => Ok(vec![json!({
            "type_": spec.option.as_ref().and_then(|v| v.as_str()).unwrap_or(""),
            "p1": match_str(m).unwrap_or_default(),
        })]),
        "insert+p1+p2" => {
            let MatchToken::A(v) = m else {
                return Err(MhchemError::msg("insert+p1+p2"));
            };
            if v.len() < 2 {
                return Err(MhchemError::msg("insert+p1+p2 short"));
            }
            Ok(vec![json!({
                "type_": spec.option.as_ref().and_then(|x| x.as_str()).unwrap_or(""),
                "p1": &v[0],
                "p2": &v[1],
            })])
        }
        "copy" => Ok(vec![Value::String(token_string(m))]),
        "rm" => Ok(vec![json!({"type_": "rm", "p1": match_str(m).unwrap_or_default()})]),
        "text" => Ok(engine::go_machine(ctx, &token_string(m), "text")?),
        "{text}" => {
            let mut v = vec![Value::String("{".into())];
            v.extend(engine::go_machine(ctx, &token_string(m), "text")?);
            v.push(Value::String("}".into()));
            Ok(v)
        }
        "tex-math" => Ok(engine::go_machine(ctx, &token_string(m), "tex-math")?),
        "tex-math tight" => Ok(engine::go_machine(
            ctx,
            &token_string(m),
            "tex-math tight",
        )?),
        "bond" => {
            let kind = spec
                .option
                .as_ref()
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| match_str(m))
                .unwrap_or_else(|| "-".into());
            Ok(vec![json!({ "type_": "bond", "kind_": kind })])
        }
        "color0-output" => {
            let MatchToken::A(v) = m else {
                return Err(MhchemError::msg("color0-output"));
            };
            Ok(vec![json!({"type_": "color0", "color": v.first().cloned().unwrap_or_default()})])
        }
        "ce" => Ok(engine::go_machine(ctx, &token_string(m), "ce")?),
        "1/2" => half_action(m),
        "9,9" => Ok(engine::go_machine(ctx, &token_string(m), "9,9")?),
        "operator" => {
            let kind = spec
                .option
                .as_ref()
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| match_str(m))
                .unwrap_or_default();
            Ok(vec![json!({ "type_": "operator", "kind_": kind })])
        }
        _ => Err(MhchemError::msg(format!(
            "mhchem bug A: {} / {}",
            machine, spec.type_
        ))),
    }
}

fn cat(slot: &mut Option<String>, m: &MatchToken) {
    let t = token_string(m);
    *slot = Some(format!("{}{}", slot.take().unwrap_or_default(), t));
}

fn half_action(m: &MatchToken) -> MhchemResult<Vec<Value>> {
    let mut s = token_string(m);
    let mut ret: Vec<Value> = vec![];
    if s.starts_with('+') || s.starts_with('-') {
        ret.push(Value::String(s[..1].to_string()));
        s = s[1..].to_string();
    }
    let re = Regex::new(r"^([0-9]+|\$[a-z]\$|[a-z])\/([0-9]+)(\$[a-z]\$|[a-z])?$").unwrap();
    let Some(c) = re.captures(&s) else {
        return Ok(vec![]);
    };
    let n1 = c.get(1).unwrap().as_str().replace('$', "");
    let n2 = c.get(2).unwrap().as_str();
    ret.push(json!({"type_": "frac", "p1": n1.clone(), "p2": n2}));
    if let Some(t) = c.get(3) {
        let t3 = t.as_str().replace('$', "");
        ret.push(json!({"type_": "tex-math", "p1": t3}));
    }
    Ok(ret)
}
