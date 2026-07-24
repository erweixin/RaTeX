//! `texify.go` — turn parser output into TeX (KaTeX mhchem 3.3).

use crate::mhchem::error::{MhchemError, MhchemResult};
use crate::parser::MAX_RECURSION_DEPTH;
use serde_json::Value;

pub fn go(input: &[Value], is_inner: bool) -> MhchemResult<String> {
    go_with_depth(input, is_inner, 0)
}

fn go_with_depth(input: &[Value], is_inner: bool, depth: usize) -> MhchemResult<String> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(MhchemError::msg("Recursion limit exceeded"));
    }
    let mut res = String::new();
    let mut cee = false;
    for inputi in input {
        match inputi {
            Value::String(s) => res.push_str(s),
            v => {
                if type_str(v).as_deref() == Some("1st-level escape") {
                    cee = true;
                }
                res.push_str(&go2(v, depth)?);
            }
        }
    }
    if !is_inner && !cee && !res.is_empty() {
        res = format!("{{{res}}}");
    }
    Ok(res)
}

fn arr_inner(v: &Value, key: &str, depth: usize) -> MhchemResult<String> {
    let sl = v
        .get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);
    go_with_depth(sl, true, depth)
}

fn type_str(v: &Value) -> Option<String> {
    v.get("type_").and_then(|x| x.as_str()).map(String::from)
}

fn go2(buf: &Value, depth: usize) -> MhchemResult<String> {
    let typ = type_str(buf).ok_or_else(|| MhchemError::msg("texify: missing type_"))?;
    Ok(match typ.as_str() {
        "chemfive" => {
            let b5a = arr_inner(buf, "a", depth + 1)?;
            let b5b = arr_inner(buf, "b", depth + 1)?;
            let b5p = arr_inner(buf, "p", depth + 1)?;
            let b5o = arr_inner(buf, "o", depth + 1)?;
            let b5q = arr_inner(buf, "q", depth + 1)?;
            let b5d = arr_inner(buf, "d", depth + 1)?;
            let mut res = String::new();
            if !b5a.is_empty() {
                let a = if b5a.starts_with('+') || b5a.starts_with('-') {
                    format!("{{{b5a}}}")
                } else {
                    b5a
                };
                res.push_str(&a);
                res.push_str("\\,");
            }
            if !b5b.is_empty() || !b5p.is_empty() {
                res.push_str("{\\vphantom{X}}");
                res.push_str(&format!(
                    "^{{\\hphantom{{{}}}}}_{{\\hphantom{{{}}}}}",
                    b5b, b5p
                ));
                res.push_str("{\\vphantom{X}}");
                res.push_str(&format!(
                    "^{{\\smash[t]{{\\vphantom{{2}}}}\\mathllap{{{}}}}}",
                    b5b
                ));
                res.push_str(&format!(
                    "_{{\\vphantom{{2}}\\mathllap{{\\smash[t]{{{}}}}}}}",
                    b5p
                ));
            }
            if !b5o.is_empty() {
                let o = if b5o.starts_with('+') || b5o.starts_with('-') {
                    format!("{{{b5o}}}")
                } else {
                    b5o
                };
                res.push_str(&o);
            }
            let d_ty = buf.get("dType").and_then(|x| x.as_str()).unwrap_or("");
            if d_ty == "kv" {
                if !b5d.is_empty() || !b5q.is_empty() {
                    res.push_str("{\\vphantom{X}}");
                }
                if !b5d.is_empty() {
                    res.push_str(&format!("^{{{b5d}}}"));
                }
                if !b5q.is_empty() {
                    res.push_str(&format!("_{{\\smash[t]{{{b5q}}}}}"));
                }
            } else if d_ty == "oxidation" {
                if !b5d.is_empty() {
                    res.push_str("{\\vphantom{X}}");
                    res.push_str(&format!("^{{{b5d}}}"));
                }
                if !b5q.is_empty() {
                    res.push_str("{\\vphantom{X}}");
                    res.push_str(&format!("_{{\\smash[t]{{{b5q}}}}}"));
                }
            } else {
                if !b5q.is_empty() {
                    res.push_str("{\\vphantom{X}}");
                    res.push_str(&format!("_{{\\smash[t]{{{b5q}}}}}"));
                }
                if !b5d.is_empty() {
                    res.push_str("{\\vphantom{X}}");
                    res.push_str(&format!("^{{{b5d}}}"));
                }
            }
            res
        }
        "rm" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("\\mathrm{{{p1}}}")
        }
        "text" => {
            let mut p1 = buf
                .get("p1")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if p1.contains('^') || p1.contains('_') {
                p1 = p1.replace(' ', "~").replace('-', "\\text{-}");
                format!("\\mathrm{{{p1}}}")
            } else {
                format!("\\text{{{p1}}}")
            }
        }
        "roman numeral" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("\\mathrm{{{p1}}}")
        }
        "state of aggregation" => {
            let i = arr_inner(buf, "p1", depth + 1)?;
            format!("\\mskip2mu {i}")
        }
        "state of aggregation subscript" => {
            let i = arr_inner(buf, "p1", depth + 1)?;
            format!("\\mskip1mu {i}")
        }
        "bond" => {
            let k = buf.get("kind_").and_then(|x| x.as_str()).unwrap_or("");
            get_bond(k)
                .ok_or_else(|| MhchemError::msg(format!("Unknown bond ({k})")))?
                .to_string()
        }
        "frac" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            let p2 = buf.get("p2").and_then(|x| x.as_str()).unwrap_or("");
            let c = format!("\\frac{{{p1}}}{{{p2}}}");
            format!(r"\mathchoice{{\textstyle{}}}{{{}}}{{{}}}{{{}}}", c, c, c, c)
        }
        "pu-frac" => {
            let p1 = arr_inner(buf, "p1", depth + 1)?;
            let p2 = arr_inner(buf, "p2", depth + 1)?;
            let d = format!("\\frac{{{p1}}}{{{p2}}}");
            format!(r"\mathchoice{{\textstyle{}}}{{{}}}{{{}}}{{{}}}", d, d, d, d)
        }
        "tex-math" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("{p1} ")
        }
        "frac-ce" => {
            let p1 = arr_inner(buf, "p1", depth + 1)?;
            let p2 = arr_inner(buf, "p2", depth + 1)?;
            format!("\\frac{{{p1}}}{{{p2}}}")
        }
        "overset" => {
            let p1 = arr_inner(buf, "p1", depth + 1)?;
            let p2 = arr_inner(buf, "p2", depth + 1)?;
            format!("\\overset{{{p1}}}{{{p2}}}")
        }
        "underset" => {
            let p1 = arr_inner(buf, "p1", depth + 1)?;
            let p2 = arr_inner(buf, "p2", depth + 1)?;
            format!("\\underset{{{p1}}}{{{p2}}}")
        }
        "underbrace" => {
            let p1 = arr_inner(buf, "p1", depth + 1)?;
            let p2 = arr_inner(buf, "p2", depth + 1)?;
            format!("\\underbrace{{{p1}}}_{{{p2}}}")
        }
        "color" => {
            let c1 = buf.get("color1").and_then(|x| x.as_str()).unwrap_or("");
            let c2 = arr_inner(buf, "color2", depth + 1)?;
            format!("{{\\color{{{c1}}}{{{c2}}}}}")
        }
        "color0" => {
            let c = buf.get("color").and_then(|x| x.as_str()).unwrap_or("");
            format!("\\color{{{c}}}")
        }
        "arrow" => {
            let rd = arr_inner(buf, "rd", depth + 1)?;
            let rq = arr_inner(buf, "rq", depth + 1)?;
            let r = buf.get("r").and_then(|x| x.as_str()).unwrap_or("");
            let mut arrow = format!("\\x{}", get_arrow(r)?);
            if !rq.is_empty() {
                arrow.push_str(&format!("[{{{rq}}}]"));
            }
            if !rd.is_empty() {
                arrow.push_str(&format!("{{{rd}}}"));
            } else {
                arrow.push_str("{}");
            }
            arrow
        }
        "operator" => {
            let k = buf.get("kind_").and_then(|x| x.as_str()).unwrap_or("");
            get_operator(k).ok_or_else(|| MhchemError::msg(format!("Unknown operator ({k})")))?
        }
        "1st-level escape" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("{p1} ")
        }
        "space" => " ".to_string(),
        "entitySkip" => "~".to_string(),
        "pu-space-1" => "~".to_string(),
        "pu-space-2" => "\\mkern3mu ".to_string(),
        "1000 separator" => "\\mkern2mu ".to_string(),
        "commaDecimal" => "{,}".to_string(),
        "comma enumeration L" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("{{{p1}}}\\mkern6mu ")
        }
        "comma enumeration M" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("{{{p1}}}\\mkern3mu ")
        }
        "comma enumeration S" => {
            let p1 = buf.get("p1").and_then(|x| x.as_str()).unwrap_or("");
            format!("{{{p1}}}\\mkern1mu ")
        }
        "hyphen" => "\\text{-}".to_string(),
        "addition compound" => "\\,{\\cdot}\\,".to_string(),
        "electron dot" => "\\mkern1mu \\bullet\\mkern1mu ".to_string(),
        "KV x" => "{\\times}".to_string(),
        "prime" => "\\prime ".to_string(),
        "cdot" => "\\cdot ".to_string(),
        "tight cdot" => "\\mkern1mu{\\cdot}\\mkern1mu ".to_string(),
        "times" => "\\times ".to_string(),
        "circa" => "{\\sim}".to_string(),
        "^" => "\\uparrow ".to_string(),
        "v" => "\\downarrow ".to_string(),
        "ellipsis" => "\\ldots ".to_string(),
        "/" => "/".to_string(),
        " / " => "\\,/\\,".to_string(),
        t => {
            return Err(MhchemError::msg(format!("texify: unknown type ({t})")));
        }
    })
}

fn get_arrow(a: &str) -> MhchemResult<&'static str> {
    Ok(match a {
        "->" | "\u{2192}" | "\u{27f6}" => "rightarrow",
        "<-" => "leftarrow",
        "<->" => "leftrightarrow",
        "<-->" => "rightleftarrows",
        "<=>" | "\u{21cc}" => "rightleftharpoons",
        "<=>>" => "rightequilibrium",
        "<<=>" => "leftequilibrium",
        _ => return Err(MhchemError::msg(format!("unknown arrow {a:?}"))),
    })
}

fn get_bond(a: &str) -> Option<&'static str> {
    Some(match a {
        "-" | "1" => "{-}",
        "=" | "2" => "{=}",
        "#" | "3" => "{\\equiv}",
        "~" => "{\\tripledash}",
        "~-" => "{\\mathrlap{\\raisebox{-.1em}{$-$}}\\raisebox{.1em}{$\\tripledash$}}",
        "~=" => "{\\mathrlap{\\raisebox{-.2em}{$-$}}\\mathrlap{\\raisebox{.2em}{$\\tripledash$}}-}",
        "~--" => {
            "{\\mathrlap{\\raisebox{-.2em}{$-$}}\\mathrlap{\\raisebox{.2em}{$\\tripledash$}}-}"
        }
        "-~-" => {
            "{\\mathrlap{\\raisebox{-.2em}{$-$}}\\mathrlap{\\raisebox{.2em}{$-$}}\\tripledash}"
        }
        "..." => "{{\\cdot}{\\cdot}{\\cdot}}",
        "...." => "{{\\cdot}{\\cdot}{\\cdot}{\\cdot}}",
        "->" => "{\\rightarrow}",
        "<-" => "{\\leftarrow}",
        "<" => "{<}",
        ">" => "{>}",
        _ => return None,
    })
}

fn get_operator(a: &str) -> Option<String> {
    Some(match a {
        "+" => " {}+{} ".to_string(),
        "-" => " {}-{} ".to_string(),
        "=" => " {}={} ".to_string(),
        "<" => " {}<{} ".to_string(),
        ">" => " {}>{} ".to_string(),
        "<<" => " {}\\ll{} ".to_string(),
        ">>" => " {}\\gg{} ".to_string(),
        "\\pm" => " {}\\pm{} ".to_string(),
        "\\approx" | "$\\approx$" => " {}\\approx{} ".to_string(),
        "v" | "(v)" => " \\downarrow{} ".to_string(),
        "^" | "(^)" => " \\uparrow{} ".to_string(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::go;

    fn nested_texify_value(depth: usize) -> Value {
        let mut value = json!({"type_": "rm", "p1": "H"});
        for _ in 0..depth {
            value = json!({
                "type_": "state of aggregation",
                "p1": [value],
            });
        }
        value
    }

    #[test]
    fn nested_values_have_a_depth_budget() {
        assert!(go(&[nested_texify_value(32)], false).is_ok());
        let error = go(&[nested_texify_value(33)], false).unwrap_err();
        assert!(
            error.to_string().contains("Recursion limit exceeded"),
            "unexpected error: {error}"
        );
    }
}
